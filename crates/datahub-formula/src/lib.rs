mod evaluator;
mod parser;

pub use evaluator::{EvaluationRuntime, evaluate_formulas};
pub use parser::parse_formula;

use std::collections::{BTreeMap, BTreeSet};

use datahub_kernel::{ConfigValue, FieldId, TypeAst};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormulaValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
}

impl FormulaValue {
    /// Converts a scalar configuration value into the formula value domain.
    ///
    /// # Errors
    /// Returns [`FormulaError::UnsupportedValue`] for composite values or an
    /// integer that cannot round-trip through the deterministic `f64` domain.
    pub fn from_config(value: &ConfigValue) -> Result<Self, FormulaError> {
        match value {
            ConfigValue::Null => Ok(Self::Null),
            ConfigValue::Bool(value) => Ok(Self::Bool(*value)),
            ConfigValue::Integer(value) => {
                let number = value.to_f64().ok_or(FormulaError::UnsupportedValue)?;
                if number.to_i64() == Some(*value) {
                    Ok(Self::Number(number))
                } else {
                    Err(FormulaError::UnsupportedValue)
                }
            }
            ConfigValue::Float(value) => Ok(Self::Number(*value)),
            ConfigValue::String(value) => Ok(Self::String(value.clone())),
            _ => Err(FormulaError::UnsupportedValue),
        }
    }

    /// Converts a formula result to the declared configuration field type.
    ///
    /// # Errors
    /// Returns [`FormulaError::TypeMismatch`] when the result does not match
    /// the target type or cannot be represented exactly.
    pub fn to_config(&self, target: &TypeAst) -> Result<ConfigValue, FormulaError> {
        match (self, target) {
            (Self::Null, TypeAst::Optional { .. }) => Ok(ConfigValue::Null),
            (Self::Bool(value), TypeAst::Bool) => Ok(ConfigValue::Bool(*value)),
            (Self::Number(value), TypeAst::Integer { .. })
                if value.is_finite() && value.fract() == 0.0 && value.to_i64().is_some() =>
            {
                Ok(ConfigValue::Integer(
                    value.to_i64().ok_or(FormulaError::TypeMismatch)?,
                ))
            }
            (Self::Number(value), TypeAst::Float { .. }) if value.is_finite() => {
                Ok(ConfigValue::Float(*value))
            }
            (Self::String(value), TypeAst::String { .. }) => Ok(ConfigValue::String(value.clone())),
            (value, TypeAst::Optional { item }) => value.to_config(item),
            _ => Err(FormulaError::TypeMismatch),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FormulaExpr {
    Literal {
        value: FormulaValue,
    },
    Field {
        field_id: FieldId,
    },
    Unary {
        op: UnaryOp,
        expression: Box<Self>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Self>,
        right: Box<Self>,
    },
    If {
        condition: Box<Self>,
        then_expression: Box<Self>,
        else_expression: Box<Self>,
    },
}

impl FormulaExpr {
    #[must_use]
    pub fn dependencies(&self) -> BTreeSet<FieldId> {
        let mut dependencies = BTreeSet::new();
        self.collect_dependencies(&mut dependencies);
        dependencies
    }

    fn collect_dependencies(&self, dependencies: &mut BTreeSet<FieldId>) {
        match self {
            Self::Field { field_id } => {
                dependencies.insert(*field_id);
            }
            Self::Unary { expression, .. } => expression.collect_dependencies(dependencies),
            Self::Binary { left, right, .. } => {
                left.collect_dependencies(dependencies);
                right.collect_dependencies(dependencies);
            }
            Self::If {
                condition,
                then_expression,
                else_expression,
            } => {
                condition.collect_dependencies(dependencies);
                then_expression.collect_dependencies(dependencies);
                else_expression.collect_dependencies(dependencies);
            }
            Self::Literal { .. } => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormulaDefinition {
    pub field_id: FieldId,
    pub source: String,
    pub expression: FormulaExpr,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FormulaSet {
    pub definitions: BTreeMap<FieldId, FormulaDefinition>,
}

impl FormulaSet {
    /// Creates and validates a set of computed-field formulas.
    ///
    /// # Errors
    /// Returns an error for duplicate target fields or dependency cycles.
    pub fn from_definitions(
        definitions: impl IntoIterator<Item = FormulaDefinition>,
    ) -> Result<Self, FormulaError> {
        let mut result = Self::default();
        for definition in definitions {
            if result
                .definitions
                .insert(definition.field_id, definition)
                .is_some()
            {
                return Err(FormulaError::DuplicateFormula);
            }
        }
        result.topological_order()?;
        Ok(result)
    }

    /// Returns a deterministic dependency-first evaluation order.
    ///
    /// # Errors
    /// Returns [`FormulaError::Cycle`] with the complete field path when the
    /// formula graph contains a dependency cycle.
    pub fn topological_order(&self) -> Result<Vec<FieldId>, FormulaError> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Visit {
            Active,
            Done,
        }

        fn visit(
            field_id: FieldId,
            set: &FormulaSet,
            state: &mut BTreeMap<FieldId, Visit>,
            stack: &mut Vec<FieldId>,
            order: &mut Vec<FieldId>,
        ) -> Result<(), FormulaError> {
            if state.get(&field_id) == Some(&Visit::Done) {
                return Ok(());
            }
            if state.get(&field_id) == Some(&Visit::Active) {
                let start = stack.iter().position(|item| *item == field_id).unwrap_or(0);
                let mut cycle = stack[start..].to_vec();
                cycle.push(field_id);
                return Err(FormulaError::Cycle { fields: cycle });
            }
            state.insert(field_id, Visit::Active);
            stack.push(field_id);
            if let Some(definition) = set.definitions.get(&field_id) {
                for dependency in definition.expression.dependencies() {
                    if set.definitions.contains_key(&dependency) {
                        visit(dependency, set, state, stack, order)?;
                    }
                }
            }
            stack.pop();
            state.insert(field_id, Visit::Done);
            order.push(field_id);
            Ok(())
        }

        let mut state = BTreeMap::new();
        let mut order = Vec::with_capacity(self.definitions.len());
        for field_id in self.definitions.keys().copied() {
            visit(field_id, self, &mut state, &mut Vec::new(), &mut order)?;
        }
        Ok(order)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum FormulaError {
    #[error("formula parse error at byte {position}: {message}")]
    Parse { position: usize, message: String },
    #[error("unknown field `{0}`")]
    UnknownField(String),
    #[error("formula is defined more than once for a field")]
    DuplicateFormula,
    #[error("formula dependency cycle: {fields:?}")]
    Cycle { fields: Vec<FieldId> },
    #[error("formula field value is missing: {0}")]
    MissingField(FieldId),
    #[error("formula value has the wrong type")]
    TypeMismatch,
    #[error("formula cannot evaluate this configuration value")]
    UnsupportedValue,
    #[error("formula division by zero")]
    DivisionByZero,
    #[error("formula produced a non-finite number")]
    NonFinite,
    #[error("WebAssembly evaluation failed: {0}")]
    Wasm(String),
}
