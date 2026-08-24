use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{ConfigRow, ConfigValue, FieldDefinition, SchemaDefinition, TypeAst};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationCode {
    EmptyName,
    DuplicateFieldId,
    DuplicateFieldName,
    DuplicateVariantId,
    DuplicateVariantName,
    DuplicateVariantValue,
    EmptyEnum,
    EmptyUnion,
    InvalidBounds,
    InvalidMapKey,
    MissingField,
    UnknownField,
    TypeMismatch,
    ConstraintViolation,
    SchemaMismatch,
    TargetLeak,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: ValidationCode,
    pub path: String,
    pub message: String,
}

impl ValidationIssue {
    pub(crate) fn new(
        code: ValidationCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }
}

#[must_use]
pub fn validate_schema(schema: &SchemaDefinition) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if schema.name.trim().is_empty() {
        issues.push(ValidationIssue::new(
            ValidationCode::EmptyName,
            "schema.name",
            "schema name cannot be empty",
        ));
    }
    validate_fields(&schema.fields, "schema.fields", &mut issues);
    issues
}

#[must_use]
pub fn validate_row(schema: &SchemaDefinition, row: &ConfigRow) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if row.schema_id != schema.id {
        issues.push(ValidationIssue::new(
            ValidationCode::SchemaMismatch,
            "row.schema_id",
            "row belongs to a different schema",
        ));
        return issues;
    }

    let known = schema
        .fields
        .iter()
        .map(|field| field.id)
        .collect::<BTreeSet<_>>();
    for id in row.values.keys().filter(|id| !known.contains(id)) {
        issues.push(ValidationIssue::new(
            ValidationCode::UnknownField,
            format!("row.values.{id}"),
            "row contains a field not present in the schema",
        ));
    }

    for field in &schema.fields {
        let path = format!("row.values.{}", field.id);
        match row.values.get(&field.id) {
            Some(value) => validate_value(&field.ty, value, &path, &mut issues),
            None if field.default.is_none() && !field.ty.is_optional() => {
                issues.push(ValidationIssue::new(
                    ValidationCode::MissingField,
                    path,
                    format!("required field '{}' is missing", field.name),
                ));
            }
            None => {}
        }
    }
    issues
}

fn validate_fields(fields: &[FieldDefinition], path: &str, issues: &mut Vec<ValidationIssue>) {
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for (index, field) in fields.iter().enumerate() {
        let field_path = format!("{path}[{index}]");
        if field.name.trim().is_empty() {
            issues.push(ValidationIssue::new(
                ValidationCode::EmptyName,
                format!("{field_path}.name"),
                "field name cannot be empty",
            ));
        }
        if !ids.insert(field.id) {
            issues.push(ValidationIssue::new(
                ValidationCode::DuplicateFieldId,
                format!("{field_path}.id"),
                "field id must be unique in its struct",
            ));
        }
        if !names.insert(field.name.as_str()) {
            issues.push(ValidationIssue::new(
                ValidationCode::DuplicateFieldName,
                format!("{field_path}.name"),
                "field name must be unique in its struct",
            ));
        }
        validate_type(&field.ty, &format!("{field_path}.type"), issues);
        if let Some(default) = &field.default {
            validate_value(&field.ty, default, &format!("{field_path}.default"), issues);
        }
    }
}

fn validate_type(ty: &TypeAst, path: &str, issues: &mut Vec<ValidationIssue>) {
    match ty {
        TypeAst::Integer { min, max } => validate_bounds(*min, *max, path, issues),
        TypeAst::Float { min, max } => {
            if min.is_some_and(|value| !value.is_finite())
                || max.is_some_and(|value| !value.is_finite())
                || matches!((min, max), (Some(min), Some(max)) if min > max)
            {
                issues.push(ValidationIssue::new(
                    ValidationCode::InvalidBounds,
                    path,
                    "float bounds must be finite and ordered",
                ));
            }
        }
        TypeAst::String {
            min_length,
            max_length,
        } => validate_bounds(*min_length, *max_length, path, issues),
        TypeAst::Optional { item } => validate_type(item, path, issues),
        TypeAst::List {
            item,
            min_items,
            max_items,
        }
        | TypeAst::Set {
            item,
            min_items,
            max_items,
        } => {
            validate_bounds(*min_items, *max_items, path, issues);
            validate_type(item, path, issues);
        }
        TypeAst::FixedArray { item, length } => {
            if *length == 0 {
                issues.push(ValidationIssue::new(
                    ValidationCode::InvalidBounds,
                    path,
                    "fixed array length must be greater than zero",
                ));
            }
            validate_type(item, path, issues);
        }
        TypeAst::Map { key, value } => {
            if !matches!(
                key.as_ref(),
                TypeAst::String { .. } | TypeAst::Integer { .. }
            ) {
                issues.push(ValidationIssue::new(
                    ValidationCode::InvalidMapKey,
                    path,
                    "map keys must be string or integer types",
                ));
            }
            validate_type(key, &format!("{path}.key"), issues);
            validate_type(value, &format!("{path}.value"), issues);
        }
        TypeAst::Struct { fields } => validate_fields(fields, &format!("{path}.fields"), issues),
        TypeAst::Enum { variants } => validate_enum(variants, path, issues),
        TypeAst::Union { variants } => {
            if variants.is_empty() {
                issues.push(ValidationIssue::new(
                    ValidationCode::EmptyUnion,
                    path,
                    "union must contain at least one variant",
                ));
            }
            for (index, variant) in variants.iter().enumerate() {
                validate_type(variant, &format!("{path}.variants[{index}]"), issues);
            }
        }
        TypeAst::Bool
        | TypeAst::Bytes
        | TypeAst::Date
        | TypeAst::DateTime
        | TypeAst::Reference { .. }
        | TypeAst::Custom { .. } => {}
    }
}

fn validate_enum(variants: &[crate::EnumVariant], path: &str, issues: &mut Vec<ValidationIssue>) {
    if variants.is_empty() {
        issues.push(ValidationIssue::new(
            ValidationCode::EmptyEnum,
            path,
            "enum must contain at least one variant",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut values = BTreeSet::new();
    for (index, variant) in variants.iter().enumerate() {
        let variant_path = format!("{path}.variants[{index}]");
        if !ids.insert(variant.id) {
            issues.push(ValidationIssue::new(
                ValidationCode::DuplicateVariantId,
                format!("{variant_path}.id"),
                "enum variant id must be unique",
            ));
        }
        if !names.insert(variant.name.as_str()) {
            issues.push(ValidationIssue::new(
                ValidationCode::DuplicateVariantName,
                format!("{variant_path}.name"),
                "enum variant name must be unique",
            ));
        }
        if !values.insert(variant.value) {
            issues.push(ValidationIssue::new(
                ValidationCode::DuplicateVariantValue,
                format!("{variant_path}.value"),
                "enum variant value must be unique",
            ));
        }
    }
}

fn validate_bounds<T>(min: Option<T>, max: Option<T>, path: &str, issues: &mut Vec<ValidationIssue>)
where
    T: PartialOrd,
{
    if matches!((min, max), (Some(min), Some(max)) if min > max) {
        issues.push(ValidationIssue::new(
            ValidationCode::InvalidBounds,
            path,
            "minimum cannot exceed maximum",
        ));
    }
}

#[allow(clippy::too_many_lines)]
fn validate_value(
    ty: &TypeAst,
    value: &ConfigValue,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let matches = match (ty, value) {
        (TypeAst::Bool, ConfigValue::Bool(_))
        | (TypeAst::Bytes, ConfigValue::Bytes(_))
        | (TypeAst::Date, ConfigValue::Date(_))
        | (TypeAst::DateTime, ConfigValue::DateTime(_))
        | (TypeAst::Optional { .. }, ConfigValue::Null) => true,
        (TypeAst::Integer { min, max }, ConfigValue::Integer(value)) => {
            if min.is_some_and(|min| *value < min) || max.is_some_and(|max| *value > max) {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "integer is outside configured bounds",
                ));
            }
            true
        }
        (TypeAst::Float { min, max }, ConfigValue::Float(value)) => {
            if !value.is_finite()
                || min.is_some_and(|min| *value < min)
                || max.is_some_and(|max| *value > max)
            {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "float is non-finite or outside configured bounds",
                ));
            }
            true
        }
        (
            TypeAst::String {
                min_length,
                max_length,
            },
            ConfigValue::String(value),
        ) => {
            let length = value.chars().count();
            if min_length.is_some_and(|min| length < min)
                || max_length.is_some_and(|max| length > max)
            {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "string length is outside configured bounds",
                ));
            }
            true
        }
        (TypeAst::Optional { item }, value) => {
            validate_value(item, value, path, issues);
            true
        }
        (TypeAst::FixedArray { item, length }, ConfigValue::FixedArray(values)) => {
            if values.len() != *length {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "fixed array does not contain the declared number of items",
                ));
            }
            for (index, item_value) in values.iter().enumerate() {
                validate_value(item, item_value, &format!("{path}[{index}]"), issues);
            }
            true
        }
        (
            TypeAst::Set {
                item,
                min_items,
                max_items,
            },
            ConfigValue::Set(values),
        ) => {
            if min_items.is_some_and(|min| values.len() < min)
                || max_items.is_some_and(|max| values.len() > max)
            {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "set size is outside configured bounds",
                ));
            }
            let mut seen = BTreeSet::new();
            for (index, item_value) in values.iter().enumerate() {
                if serde_json::to_string(item_value).is_ok_and(|encoded| !seen.insert(encoded)) {
                    issues.push(ValidationIssue::new(
                        ValidationCode::ConstraintViolation,
                        format!("{path}[{index}]"),
                        "set items must be unique",
                    ));
                }
                validate_value(item, item_value, &format!("{path}[{index}]"), issues);
            }
            true
        }
        (
            TypeAst::List {
                item,
                min_items,
                max_items,
            },
            ConfigValue::List(values),
        ) => {
            if min_items.is_some_and(|min| values.len() < min)
                || max_items.is_some_and(|max| values.len() > max)
            {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "list length is outside configured bounds",
                ));
            }
            for (index, item_value) in values.iter().enumerate() {
                validate_value(item, item_value, &format!("{path}[{index}]"), issues);
            }
            true
        }
        (TypeAst::Map { key, value }, ConfigValue::Map(values)) => {
            for (map_key, map_value) in values {
                let key_value = match key.as_ref() {
                    TypeAst::Integer { .. } => map_key.parse::<i64>().map_or_else(
                        |_| ConfigValue::String(map_key.clone()),
                        ConfigValue::Integer,
                    ),
                    _ => ConfigValue::String(map_key.clone()),
                };
                validate_value(key, &key_value, &format!("{path}.{map_key}.key"), issues);
                validate_value(value, map_value, &format!("{path}.{map_key}"), issues);
            }
            true
        }
        (TypeAst::Struct { fields }, ConfigValue::Struct(values)) => {
            let nested = ConfigRow {
                id: crate::RowId::new(),
                schema_id: crate::SchemaId::new(),
                revision_id: crate::RevisionId::new(),
                values: values.clone(),
            };
            validate_struct_values(fields, &nested, path, issues);
            true
        }
        (TypeAst::Enum { variants }, ConfigValue::Enum(id)) => {
            if !variants.iter().any(|variant| variant.id == *id) {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "enum value is not a declared variant",
                ));
            }
            true
        }
        (TypeAst::Union { variants }, ConfigValue::Union { variant, value }) => {
            if let Some(ty) = variants.get(*variant) {
                validate_value(ty, value, path, issues);
            } else {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "union variant index is outside the declared variants",
                ));
            }
            true
        }
        (
            TypeAst::Reference { schema_id, .. },
            ConfigValue::Reference {
                schema_id: actual, ..
            },
        ) => {
            if actual != schema_id {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "reference targets a different schema",
                ));
            }
            true
        }
        (
            TypeAst::Custom { custom_type_id },
            ConfigValue::Custom {
                custom_type_id: actual,
                ..
            },
        ) => {
            if actual != custom_type_id {
                issues.push(ValidationIssue::new(
                    ValidationCode::ConstraintViolation,
                    path,
                    "value uses a different custom type",
                ));
            }
            true
        }
        _ => false,
    };

    if !matches {
        issues.push(ValidationIssue::new(
            ValidationCode::TypeMismatch,
            path,
            "configuration value does not match its field type",
        ));
    }
}

fn validate_struct_values(
    fields: &[FieldDefinition],
    row: &ConfigRow,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let known = fields.iter().map(|field| field.id).collect::<BTreeSet<_>>();
    for id in row.values.keys().filter(|id| !known.contains(id)) {
        issues.push(ValidationIssue::new(
            ValidationCode::UnknownField,
            format!("{path}.{id}"),
            "struct contains an unknown field",
        ));
    }
    for field in fields {
        let field_path = format!("{path}.{}", field.id);
        match row.values.get(&field.id) {
            Some(value) => validate_value(&field.ty, value, &field_path, issues),
            None if field.default.is_none() && !field.ty.is_optional() => {
                issues.push(ValidationIssue::new(
                    ValidationCode::MissingField,
                    field_path,
                    "required struct field is missing",
                ));
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        ConfigRow, ConfigValue, FieldDefinition, FieldId, ProjectId, RevisionId, RowId,
        SchemaDefinition, SchemaId, TargetRule, TypeAst, ValidationCode, validate_row,
        validate_schema,
    };

    fn sample_schema() -> SchemaDefinition {
        SchemaDefinition {
            id: SchemaId::new(),
            project_id: ProjectId::new(),
            name: "Monster".into(),
            description: String::new(),
            fields: vec![FieldDefinition {
                id: FieldId::new(),
                name: "level".into(),
                description: String::new(),
                ty: TypeAst::Integer {
                    min: Some(1),
                    max: Some(100),
                },
                default: None,
                target: TargetRule::default(),
            }],
            target: TargetRule::default(),
        }
    }

    #[test]
    fn detects_duplicate_field_names() {
        let mut schema = sample_schema();
        let mut duplicate = schema.fields[0].clone();
        duplicate.id = FieldId::new();
        schema.fields.push(duplicate);
        assert!(
            validate_schema(&schema)
                .iter()
                .any(|issue| issue.code == ValidationCode::DuplicateFieldName)
        );
    }

    #[test]
    fn validates_required_fields_and_constraints() {
        let schema = sample_schema();
        let field_id = schema.fields[0].id;
        let missing = ConfigRow {
            id: RowId::new(),
            schema_id: schema.id,
            revision_id: RevisionId::new(),
            values: BTreeMap::new(),
        };
        assert_eq!(
            validate_row(&schema, &missing)[0].code,
            ValidationCode::MissingField
        );

        let invalid = ConfigRow {
            values: BTreeMap::from([(field_id, ConfigValue::Integer(101))]),
            ..missing
        };
        assert_eq!(
            validate_row(&schema, &invalid)[0].code,
            ValidationCode::ConstraintViolation
        );
    }
}
