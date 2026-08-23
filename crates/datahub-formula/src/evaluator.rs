use std::collections::BTreeMap;

use datahub_kernel::{ConfigValue, FieldId};
use serde::{Deserialize, Serialize};
use wasmtime::{Engine, Instance, Module, Store, TypedFunc};

use crate::{BinaryOp, FormulaError, FormulaExpr, FormulaSet, FormulaValue, UnaryOp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationRuntime {
    Native,
    Wasm,
}

/// Evaluates every computed field in dependency order.
///
/// # Errors
/// Returns parser-domain type, missing dependency, arithmetic, cycle, or WASM
/// execution errors without partially returning computed values.
pub fn evaluate_formulas(
    formulas: &FormulaSet,
    row_values: &BTreeMap<FieldId, ConfigValue>,
    runtime: EvaluationRuntime,
) -> Result<BTreeMap<FieldId, FormulaValue>, FormulaError> {
    let mut values = row_values
        .iter()
        .filter_map(|(field_id, value)| {
            FormulaValue::from_config(value)
                .ok()
                .map(|value| (*field_id, value))
        })
        .collect::<BTreeMap<_, _>>();
    let mut results = BTreeMap::new();
    let mut operations = Operations::new(runtime)?;
    for field_id in formulas.topological_order()? {
        let definition = &formulas.definitions[&field_id];
        let value = evaluate_expression(&definition.expression, &values, &mut operations)?;
        values.insert(field_id, value.clone());
        results.insert(field_id, value);
    }
    Ok(results)
}

fn evaluate_expression(
    expression: &FormulaExpr,
    values: &BTreeMap<FieldId, FormulaValue>,
    operations: &mut Operations,
) -> Result<FormulaValue, FormulaError> {
    match expression {
        FormulaExpr::Literal { value } => Ok(value.clone()),
        FormulaExpr::Field { field_id } => values
            .get(field_id)
            .cloned()
            .ok_or(FormulaError::MissingField(*field_id)),
        FormulaExpr::Unary { op, expression } => {
            let value = evaluate_expression(expression, values, operations)?;
            match (op, value) {
                (UnaryOp::Negate, FormulaValue::Number(value)) => finite(-value),
                (UnaryOp::Not, FormulaValue::Bool(value)) => Ok(FormulaValue::Bool(!value)),
                _ => Err(FormulaError::TypeMismatch),
            }
        }
        FormulaExpr::If {
            condition,
            then_expression,
            else_expression,
        } => match evaluate_expression(condition, values, operations)? {
            FormulaValue::Bool(true) => evaluate_expression(then_expression, values, operations),
            FormulaValue::Bool(false) => evaluate_expression(else_expression, values, operations),
            _ => Err(FormulaError::TypeMismatch),
        },
        FormulaExpr::Binary {
            op: BinaryOp::And,
            left,
            right,
        } => match evaluate_expression(left, values, operations)? {
            FormulaValue::Bool(false) => Ok(FormulaValue::Bool(false)),
            FormulaValue::Bool(true) => match evaluate_expression(right, values, operations)? {
                FormulaValue::Bool(value) => Ok(FormulaValue::Bool(value)),
                _ => Err(FormulaError::TypeMismatch),
            },
            _ => Err(FormulaError::TypeMismatch),
        },
        FormulaExpr::Binary {
            op: BinaryOp::Or,
            left,
            right,
        } => match evaluate_expression(left, values, operations)? {
            FormulaValue::Bool(true) => Ok(FormulaValue::Bool(true)),
            FormulaValue::Bool(false) => match evaluate_expression(right, values, operations)? {
                FormulaValue::Bool(value) => Ok(FormulaValue::Bool(value)),
                _ => Err(FormulaError::TypeMismatch),
            },
            _ => Err(FormulaError::TypeMismatch),
        },
        FormulaExpr::Binary { op, left, right } => {
            let left = evaluate_expression(left, values, operations)?;
            let right = evaluate_expression(right, values, operations)?;
            evaluate_binary(*op, left, right, operations)
        }
    }
}

fn evaluate_binary(
    op: BinaryOp,
    left: FormulaValue,
    right: FormulaValue,
    operations: &mut Operations,
) -> Result<FormulaValue, FormulaError> {
    match (op, left, right) {
        (BinaryOp::Add, FormulaValue::String(left), FormulaValue::String(right)) => {
            Ok(FormulaValue::String(left + &right))
        }
        (BinaryOp::Equal, FormulaValue::Number(left), FormulaValue::Number(right)) => {
            Ok(FormulaValue::Bool(numbers_equal(left, right)))
        }
        (BinaryOp::NotEqual, FormulaValue::Number(left), FormulaValue::Number(right)) => {
            Ok(FormulaValue::Bool(!numbers_equal(left, right)))
        }
        (BinaryOp::Equal, left, right) => Ok(FormulaValue::Bool(left == right)),
        (BinaryOp::NotEqual, left, right) => Ok(FormulaValue::Bool(left != right)),
        (op, FormulaValue::Number(left), FormulaValue::Number(right)) => {
            operations.numeric(op, left, right)
        }
        _ => Err(FormulaError::TypeMismatch),
    }
}

enum Operations {
    Native,
    Wasm(Box<WasmOperations>),
}

impl Operations {
    fn new(runtime: EvaluationRuntime) -> Result<Self, FormulaError> {
        match runtime {
            EvaluationRuntime::Native => Ok(Self::Native),
            EvaluationRuntime::Wasm => Ok(Self::Wasm(Box::new(WasmOperations::new()?))),
        }
    }

    fn numeric(
        &mut self,
        op: BinaryOp,
        left: f64,
        right: f64,
    ) -> Result<FormulaValue, FormulaError> {
        if op == BinaryOp::Divide && (numbers_equal(right, 0.0) || numbers_equal(right, -0.0)) {
            return Err(FormulaError::DivisionByZero);
        }
        match self {
            Self::Native => native_numeric(op, left, right),
            Self::Wasm(operations) => operations.numeric(op, left, right),
        }
    }
}

fn native_numeric(op: BinaryOp, left: f64, right: f64) -> Result<FormulaValue, FormulaError> {
    match op {
        BinaryOp::Add => finite(left + right),
        BinaryOp::Subtract => finite(left - right),
        BinaryOp::Multiply => finite(left * right),
        BinaryOp::Divide => finite(left / right),
        BinaryOp::Less => Ok(FormulaValue::Bool(left < right)),
        BinaryOp::LessEqual => Ok(FormulaValue::Bool(left <= right)),
        BinaryOp::Greater => Ok(FormulaValue::Bool(left > right)),
        BinaryOp::GreaterEqual => Ok(FormulaValue::Bool(left >= right)),
        BinaryOp::Equal => Ok(FormulaValue::Bool(numbers_equal(left, right))),
        BinaryOp::NotEqual => Ok(FormulaValue::Bool(!numbers_equal(left, right))),
        BinaryOp::And | BinaryOp::Or => Err(FormulaError::TypeMismatch),
    }
}

fn finite(value: f64) -> Result<FormulaValue, FormulaError> {
    if value.is_finite() {
        Ok(FormulaValue::Number(value))
    } else {
        Err(FormulaError::NonFinite)
    }
}

struct WasmOperations {
    store: Store<()>,
    add: TypedFunc<(f64, f64), f64>,
    subtract: TypedFunc<(f64, f64), f64>,
    multiply: TypedFunc<(f64, f64), f64>,
    divide: TypedFunc<(f64, f64), f64>,
    less: TypedFunc<(f64, f64), i32>,
    less_equal: TypedFunc<(f64, f64), i32>,
    greater: TypedFunc<(f64, f64), i32>,
    greater_equal: TypedFunc<(f64, f64), i32>,
}

impl WasmOperations {
    fn new() -> Result<Self, FormulaError> {
        const MODULE: &str = r#"
            (module
              (func (export "add") (param f64 f64) (result f64)
                local.get 0 local.get 1 f64.add)
              (func (export "subtract") (param f64 f64) (result f64)
                local.get 0 local.get 1 f64.sub)
              (func (export "multiply") (param f64 f64) (result f64)
                local.get 0 local.get 1 f64.mul)
              (func (export "divide") (param f64 f64) (result f64)
                local.get 0 local.get 1 f64.div)
              (func (export "less") (param f64 f64) (result i32)
                local.get 0 local.get 1 f64.lt)
              (func (export "less_equal") (param f64 f64) (result i32)
                local.get 0 local.get 1 f64.le)
              (func (export "greater") (param f64 f64) (result i32)
                local.get 0 local.get 1 f64.gt)
              (func (export "greater_equal") (param f64 f64) (result i32)
                local.get 0 local.get 1 f64.ge))
        "#;
        let engine = Engine::default();
        let module = Module::new(&engine, MODULE).map_err(wasm_error)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).map_err(wasm_error)?;
        Ok(Self {
            add: typed(&instance, &mut store, "add")?,
            subtract: typed(&instance, &mut store, "subtract")?,
            multiply: typed(&instance, &mut store, "multiply")?,
            divide: typed(&instance, &mut store, "divide")?,
            less: typed(&instance, &mut store, "less")?,
            less_equal: typed(&instance, &mut store, "less_equal")?,
            greater: typed(&instance, &mut store, "greater")?,
            greater_equal: typed(&instance, &mut store, "greater_equal")?,
            store,
        })
    }

    fn numeric(
        &mut self,
        op: BinaryOp,
        left: f64,
        right: f64,
    ) -> Result<FormulaValue, FormulaError> {
        match op {
            BinaryOp::Add => finite(
                self.add
                    .call(&mut self.store, (left, right))
                    .map_err(wasm_error)?,
            ),
            BinaryOp::Subtract => finite(
                self.subtract
                    .call(&mut self.store, (left, right))
                    .map_err(wasm_error)?,
            ),
            BinaryOp::Multiply => finite(
                self.multiply
                    .call(&mut self.store, (left, right))
                    .map_err(wasm_error)?,
            ),
            BinaryOp::Divide => finite(
                self.divide
                    .call(&mut self.store, (left, right))
                    .map_err(wasm_error)?,
            ),
            BinaryOp::Less => comparison(&mut self.store, &self.less, left, right),
            BinaryOp::LessEqual => comparison(&mut self.store, &self.less_equal, left, right),
            BinaryOp::Greater => comparison(&mut self.store, &self.greater, left, right),
            BinaryOp::GreaterEqual => comparison(&mut self.store, &self.greater_equal, left, right),
            BinaryOp::Equal => Ok(FormulaValue::Bool(numbers_equal(left, right))),
            BinaryOp::NotEqual => Ok(FormulaValue::Bool(!numbers_equal(left, right))),
            BinaryOp::And | BinaryOp::Or => Err(FormulaError::TypeMismatch),
        }
    }
}

fn comparison(
    store: &mut Store<()>,
    function: &TypedFunc<(f64, f64), i32>,
    left: f64,
    right: f64,
) -> Result<FormulaValue, FormulaError> {
    Ok(FormulaValue::Bool(
        function.call(store, (left, right)).map_err(wasm_error)? != 0,
    ))
}

fn typed<Params, Results>(
    instance: &Instance,
    store: &mut Store<()>,
    name: &str,
) -> Result<TypedFunc<Params, Results>, FormulaError>
where
    Params: wasmtime::WasmParams,
    Results: wasmtime::WasmResults,
{
    instance.get_typed_func(store, name).map_err(wasm_error)
}

fn wasm_error(error: impl std::fmt::Display) -> FormulaError {
    FormulaError::Wasm(error.to_string())
}

fn numbers_equal(left: f64, right: f64) -> bool {
    left.total_cmp(&right).is_eq()
}
