use std::collections::BTreeMap;

use datahub_formula::{
    EvaluationRuntime, FormulaDefinition, FormulaError, FormulaSet, FormulaValue,
    evaluate_formulas, parse_formula,
};
use datahub_kernel::{
    Audience, CompilationTarget, ConfigValue, FieldDefinition, FieldId, ProjectId,
    SchemaDefinition, SchemaId, TargetRule, TypeAst,
};

fn schema() -> SchemaDefinition {
    SchemaDefinition {
        id: SchemaId::new(),
        project_id: ProjectId::new(),
        name: "PriceTable".into(),
        description: String::new(),
        fields: ["price", "quantity", "discount", "subtotal", "total"]
            .into_iter()
            .map(|name| FieldDefinition {
                id: FieldId::new(),
                name: name.into(),
                description: String::new(),
                ty: TypeAst::Float {
                    min: None,
                    max: None,
                },
                default: None,
                target: TargetRule {
                    include: vec![CompilationTarget::Rust],
                    audiences: vec![Audience::Client],
                    rename: BTreeMap::new(),
                },
            })
            .collect(),
        target: TargetRule::default(),
    }
}

#[test]
fn parser_binds_names_to_stable_field_ids() {
    let schema = schema();
    let expression = parse_formula("price * quantity", &schema).unwrap();
    let dependencies = expression.dependencies();
    assert!(dependencies.contains(&schema.fields[0].id));
    assert!(dependencies.contains(&schema.fields[1].id));
    assert_eq!(dependencies.len(), 2);
    assert_eq!(
        parse_formula("missing + 1", &schema),
        Err(FormulaError::UnknownField("missing".into()))
    );
}

#[test]
fn parsed_formula_survives_display_name_changes() {
    let mut schema = schema();
    let price = schema.fields[0].id;
    let quantity = schema.fields[1].id;
    let subtotal = schema.fields[3].id;
    let expression = parse_formula("price * quantity", &schema).unwrap();
    schema.fields[0].name = "unit_price".into();
    schema.fields[1].name = "count".into();
    let formulas = FormulaSet::from_definitions([FormulaDefinition {
        field_id: subtotal,
        source: "price * quantity".into(),
        expression,
    }])
    .unwrap();
    let values = BTreeMap::from([
        (price, ConfigValue::Float(8.0)),
        (quantity, ConfigValue::Float(5.0)),
    ]);

    let result = evaluate_formulas(&formulas, &values, EvaluationRuntime::Native).unwrap();
    assert_eq!(result[&subtotal], FormulaValue::Number(40.0));
}

#[test]
fn dependency_cycles_report_the_field_path() {
    let schema = schema();
    let subtotal = schema.fields[3].id;
    let total = schema.fields[4].id;
    let result = FormulaSet::from_definitions([
        FormulaDefinition {
            field_id: subtotal,
            source: "total + 1".into(),
            expression: parse_formula("total + 1", &schema).unwrap(),
        },
        FormulaDefinition {
            field_id: total,
            source: "subtotal + 1".into(),
            expression: parse_formula("subtotal + 1", &schema).unwrap(),
        },
    ]);
    let Err(FormulaError::Cycle { fields }) = result else {
        panic!("expected cycle diagnostic")
    };
    assert_eq!(fields.first(), fields.last());
    assert!(fields.contains(&subtotal));
    assert!(fields.contains(&total));
}

#[test]
fn native_and_wasm_evaluation_are_identical() {
    let schema = schema();
    let price = schema.fields[0].id;
    let quantity = schema.fields[1].id;
    let discount = schema.fields[2].id;
    let subtotal = schema.fields[3].id;
    let total = schema.fields[4].id;
    let formulas = FormulaSet::from_definitions([
        FormulaDefinition {
            field_id: subtotal,
            source: "price * quantity".into(),
            expression: parse_formula("price * quantity", &schema).unwrap(),
        },
        FormulaDefinition {
            field_id: total,
            source: "if(discount > 0, subtotal - discount, subtotal)".into(),
            expression: parse_formula("if(discount > 0, subtotal - discount, subtotal)", &schema)
                .unwrap(),
        },
    ])
    .unwrap();
    let values = BTreeMap::from([
        (price, ConfigValue::Float(12.5)),
        (quantity, ConfigValue::Float(4.0)),
        (discount, ConfigValue::Float(3.0)),
    ]);
    let native = evaluate_formulas(&formulas, &values, EvaluationRuntime::Native).unwrap();
    let wasm = evaluate_formulas(&formulas, &values, EvaluationRuntime::Wasm).unwrap();
    assert_eq!(native, wasm);
    assert_eq!(native[&subtotal], FormulaValue::Number(50.0));
    assert_eq!(native[&total], FormulaValue::Number(47.0));
}
