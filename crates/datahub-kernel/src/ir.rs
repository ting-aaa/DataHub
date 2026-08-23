use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use crate::{
    Audience, CustomTypeDefinition, CustomTypeId, FieldId, ReferenceMode, SchemaDefinition,
    SchemaId, TypeAst, ValidationCode, ValidationIssue, validate_schema,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilationTarget {
    Rust,
    CSharp,
    TypeScript,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetType {
    Bool,
    I64,
    F64,
    String,
    Bytes,
    Date,
    DateTime,
    Optional { item: Box<Self> },
    List { item: Box<Self> },
    FixedArray { item: Box<Self>, length: usize },
    Set { item: Box<Self> },
    Map { key: Box<Self>, value: Box<Self> },
    Struct { fields: Vec<TargetField> },
    Enum { variants: Vec<(String, i32)> },
    Union { variants: Vec<Self> },
    Reference { schema_id: SchemaId },
    Custom { custom_type_id: CustomTypeId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetField {
    pub id: FieldId,
    pub source_name: String,
    pub emitted_name: String,
    pub ty: TargetType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetIr {
    pub schema_id: SchemaId,
    pub source_name: String,
    pub emitted_name: String,
    pub target: CompilationTarget,
    pub fields: Vec<TargetField>,
}

/// Builds deterministic, target-aware intermediate representation.
///
/// # Errors
///
/// Returns all schema validation issues when the source schema is invalid.
pub fn build_target_ir(
    schema: &SchemaDefinition,
    target: CompilationTarget,
) -> Result<TargetIr, Vec<ValidationIssue>> {
    build_target_ir_internal(schema, target, None)
}

/// Builds deterministic IR filtered by both output language and C/S/E audience.
///
/// # Errors
/// Returns schema and target-policy diagnostics.
pub fn build_target_ir_for_audience(
    schema: &SchemaDefinition,
    target: CompilationTarget,
    audience: Audience,
) -> Result<TargetIr, Vec<ValidationIssue>> {
    build_target_ir_internal(schema, target, Some(audience))
}

fn build_target_ir_internal(
    schema: &SchemaDefinition,
    target: CompilationTarget,
    audience: Option<Audience>,
) -> Result<TargetIr, Vec<ValidationIssue>> {
    let issues = validate_schema(schema);
    if !issues.is_empty() {
        return Err(issues);
    }
    if !schema.target.includes(target)
        || audience.is_some_and(|audience| !schema.target.includes_audience(audience))
    {
        return Err(vec![ValidationIssue::new(
            ValidationCode::TargetLeak,
            "schema.target",
            "schema is excluded from the requested compilation target",
        )]);
    }

    let schema = schema.canonicalized();
    Ok(TargetIr {
        schema_id: schema.id,
        emitted_name: emitted_type_name(schema.target.emitted_name(target, &schema.name), target),
        source_name: schema.name,
        target,
        fields: schema
            .fields
            .into_iter()
            .filter(|field| {
                field.target.includes(target)
                    && audience.is_none_or(|audience| field.target.includes_audience(audience))
            })
            .map(|field| TargetField {
                id: field.id,
                emitted_name: emitted_field_name(
                    field.target.emitted_name(target, &field.name),
                    target,
                ),
                source_name: field.name,
                ty: lower_type(&field.ty, target),
            })
            .collect(),
    })
}

/// Builds a target IR set and rejects hard references that would leak to a
/// missing or target-excluded schema/custom type.
///
/// # Errors
///
/// Returns schema validation and target reachability diagnostics.
pub fn build_target_ir_set(
    schemas: &[SchemaDefinition],
    custom_types: &[CustomTypeDefinition],
    target: CompilationTarget,
) -> Result<Vec<TargetIr>, Vec<ValidationIssue>> {
    build_target_ir_set_internal(schemas, custom_types, target, None)
}

/// Builds a validated target IR set filtered by a C/S/E audience.
///
/// # Errors
/// Returns schema validation and target reachability diagnostics.
pub fn build_target_ir_set_for_audience(
    schemas: &[SchemaDefinition],
    custom_types: &[CustomTypeDefinition],
    target: CompilationTarget,
    audience: Audience,
) -> Result<Vec<TargetIr>, Vec<ValidationIssue>> {
    build_target_ir_set_internal(schemas, custom_types, target, Some(audience))
}

fn build_target_ir_set_internal(
    schemas: &[SchemaDefinition],
    custom_types: &[CustomTypeDefinition],
    target: CompilationTarget,
    audience: Option<Audience>,
) -> Result<Vec<TargetIr>, Vec<ValidationIssue>> {
    let included_schemas = schemas
        .iter()
        .filter(|schema| target_matches(&schema.target, target, audience))
        .map(|schema| schema.id)
        .collect::<BTreeSet<_>>();
    let included_custom_types = custom_types
        .iter()
        .filter(|custom| target_matches(&custom.target, target, audience))
        .map(|custom| custom.id)
        .collect::<BTreeSet<_>>();
    let mut issues = Vec::new();

    for schema in schemas
        .iter()
        .filter(|schema| target_matches(&schema.target, target, audience))
    {
        issues.extend(validate_schema(schema));
        for field in schema
            .fields
            .iter()
            .filter(|field| target_matches(&field.target, target, audience))
        {
            validate_target_reachability(
                &field.ty,
                &included_schemas,
                &included_custom_types,
                &format!("schema.{}.field.{}", schema.id, field.id),
                &mut issues,
            );
        }
    }
    for custom in custom_types
        .iter()
        .filter(|custom| target_matches(&custom.target, target, audience))
    {
        validate_target_reachability(
            &custom.ty,
            &included_schemas,
            &included_custom_types,
            &format!("custom_type.{}", custom.id),
            &mut issues,
        );
    }

    if !issues.is_empty() {
        return Err(issues);
    }
    schemas
        .iter()
        .filter(|schema| target_matches(&schema.target, target, audience))
        .map(|schema| build_target_ir_internal(schema, target, audience))
        .collect()
}

fn target_matches(
    rule: &crate::TargetRule,
    target: CompilationTarget,
    audience: Option<Audience>,
) -> bool {
    rule.includes(target) && audience.is_none_or(|audience| rule.includes_audience(audience))
}

fn validate_target_reachability(
    ty: &TypeAst,
    schemas: &BTreeSet<SchemaId>,
    custom_types: &BTreeSet<CustomTypeId>,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    match ty {
        TypeAst::Reference {
            schema_id,
            mode: ReferenceMode::Hard,
        } if !schemas.contains(schema_id) => issues.push(ValidationIssue::new(
            ValidationCode::TargetLeak,
            path,
            "hard reference targets a schema excluded from this target",
        )),
        TypeAst::Custom { custom_type_id } if !custom_types.contains(custom_type_id) => {
            issues.push(ValidationIssue::new(
                ValidationCode::TargetLeak,
                path,
                "field uses a custom type excluded from this target",
            ));
        }
        TypeAst::Optional { item }
        | TypeAst::List { item, .. }
        | TypeAst::FixedArray { item, .. }
        | TypeAst::Set { item, .. } => {
            validate_target_reachability(item, schemas, custom_types, path, issues);
        }
        TypeAst::Map { key, value } => {
            validate_target_reachability(key, schemas, custom_types, path, issues);
            validate_target_reachability(value, schemas, custom_types, path, issues);
        }
        TypeAst::Struct { fields } => {
            for field in fields {
                validate_target_reachability(&field.ty, schemas, custom_types, path, issues);
            }
        }
        TypeAst::Union { variants } => {
            for variant in variants {
                validate_target_reachability(variant, schemas, custom_types, path, issues);
            }
        }
        TypeAst::Reference { .. }
        | TypeAst::Bool
        | TypeAst::Integer { .. }
        | TypeAst::Float { .. }
        | TypeAst::String { .. }
        | TypeAst::Bytes
        | TypeAst::Date
        | TypeAst::DateTime
        | TypeAst::Enum { .. }
        | TypeAst::Custom { .. } => {}
    }
}

fn lower_type(ty: &TypeAst, target: CompilationTarget) -> TargetType {
    match ty {
        TypeAst::Bool => TargetType::Bool,
        TypeAst::Integer { .. } => TargetType::I64,
        TypeAst::Float { .. } => TargetType::F64,
        TypeAst::String { .. } => TargetType::String,
        TypeAst::Bytes => TargetType::Bytes,
        TypeAst::Date => TargetType::Date,
        TypeAst::DateTime => TargetType::DateTime,
        TypeAst::Optional { item } => TargetType::Optional {
            item: Box::new(lower_type(item, target)),
        },
        TypeAst::List { item, .. } => TargetType::List {
            item: Box::new(lower_type(item, target)),
        },
        TypeAst::FixedArray { item, length } => TargetType::FixedArray {
            item: Box::new(lower_type(item, target)),
            length: *length,
        },
        TypeAst::Set { item, .. } => TargetType::Set {
            item: Box::new(lower_type(item, target)),
        },
        TypeAst::Map { key, value } => TargetType::Map {
            key: Box::new(lower_type(key, target)),
            value: Box::new(lower_type(value, target)),
        },
        TypeAst::Struct { fields } => {
            let mut fields = fields.clone();
            fields.sort_by_key(|field| field.id);
            TargetType::Struct {
                fields: fields
                    .into_iter()
                    .map(|field| TargetField {
                        id: field.id,
                        emitted_name: emitted_field_name(&field.name, target),
                        source_name: field.name,
                        ty: lower_type(&field.ty, target),
                    })
                    .collect(),
            }
        }
        TypeAst::Enum { variants } => {
            let mut variants = variants.clone();
            variants.sort_by_key(|variant| variant.id);
            TargetType::Enum {
                variants: variants
                    .into_iter()
                    .map(|variant| (variant.name, variant.value))
                    .collect(),
            }
        }
        TypeAst::Union { variants } => TargetType::Union {
            variants: variants
                .iter()
                .map(|variant| lower_type(variant, target))
                .collect(),
        },
        TypeAst::Reference { schema_id, .. } => TargetType::Reference {
            schema_id: *schema_id,
        },
        TypeAst::Custom { custom_type_id } => TargetType::Custom {
            custom_type_id: *custom_type_id,
        },
    }
}

fn emitted_type_name(name: &str, _target: CompilationTarget) -> String {
    to_pascal_case(name)
}

fn emitted_field_name(name: &str, target: CompilationTarget) -> String {
    match target {
        CompilationTarget::CSharp => to_pascal_case(name),
        CompilationTarget::Rust | CompilationTarget::TypeScript => to_camel_case(name),
    }
}

fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
}

fn to_pascal_case(name: &str) -> String {
    words(name)
        .map(|word| {
            let mut characters = word.chars();
            characters.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + characters.as_str()
            })
        })
        .collect()
}

fn to_camel_case(name: &str) -> String {
    let pascal = to_pascal_case(name);
    let mut characters = pascal.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_lowercase().collect::<String>() + characters.as_str()
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use uuid::Uuid;

    use crate::{
        CompilationTarget, FieldDefinition, FieldId, ProjectId, SchemaDefinition, SchemaId,
        TargetRule, TypeAst, ValidationCode, build_target_ir, build_target_ir_set,
    };

    fn field(id: FieldId, name: &str) -> FieldDefinition {
        FieldDefinition {
            id,
            name: name.into(),
            description: String::new(),
            ty: TypeAst::String {
                min_length: None,
                max_length: None,
            },
            default: None,
            target: TargetRule::default(),
        }
    }

    #[test]
    fn ir_is_deterministic_across_field_insertion_order() {
        let first_id = FieldId::new();
        let second_id = FieldId::new();
        let schema = SchemaDefinition {
            id: SchemaId::new(),
            project_id: ProjectId::new(),
            name: "monster table".into(),
            description: String::new(),
            fields: vec![field(first_id, "display name"), field(second_id, "icon")],
            target: TargetRule::default(),
        };
        let mut reversed = schema.clone();
        reversed.fields.reverse();

        let first = build_target_ir(&schema, CompilationTarget::Rust).expect("valid schema");
        let second = build_target_ir(&reversed, CompilationTarget::Rust).expect("valid schema");
        assert_eq!(first, second);
        assert_eq!(first.emitted_name, "MonsterTable");
        assert_eq!(
            first.fields[0].emitted_name,
            if first.fields[0].id == first_id {
                "displayName"
            } else {
                "icon"
            }
        );
    }

    #[test]
    fn hard_reference_cannot_leak_to_an_excluded_target() {
        let referenced_id = SchemaId::new();
        let source = SchemaDefinition {
            id: SchemaId::new(),
            project_id: ProjectId::new(),
            name: "Source".into(),
            description: String::new(),
            fields: vec![FieldDefinition {
                id: FieldId::new(),
                name: "target".into(),
                description: String::new(),
                ty: TypeAst::Reference {
                    schema_id: referenced_id,
                    mode: crate::ReferenceMode::Hard,
                },
                default: None,
                target: TargetRule::default(),
            }],
            target: TargetRule::default(),
        };
        let excluded = SchemaDefinition {
            id: referenced_id,
            project_id: source.project_id,
            name: "ServerOnly".into(),
            description: String::new(),
            fields: Vec::new(),
            target: TargetRule {
                include: vec![CompilationTarget::Rust],
                audiences: vec![crate::Audience::Server],
                rename: BTreeMap::default(),
            },
        };

        let issues = build_target_ir_set(&[source, excluded], &[], CompilationTarget::TypeScript)
            .expect_err("hard target leak must fail");
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == ValidationCode::TargetLeak)
        );
    }

    #[test]
    fn every_field_permutation_produces_the_same_ir() {
        let ids = [FieldId::new(), FieldId::new(), FieldId::new()];
        let base = SchemaDefinition {
            id: SchemaId::new(),
            project_id: ProjectId::new(),
            name: "Permutation".into(),
            description: String::new(),
            fields: vec![
                field(ids[0], "alpha"),
                field(ids[1], "beta"),
                field(ids[2], "gamma"),
            ],
            target: TargetRule::default(),
        };
        let expected = build_target_ir(&base, CompilationTarget::Rust).expect("valid schema");
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let mut permuted = base.clone();
            permuted.fields = order
                .into_iter()
                .map(|index| base.fields[index].clone())
                .collect();
            assert_eq!(
                build_target_ir(&permuted, CompilationTarget::Rust).expect("valid schema"),
                expected
            );
        }
    }

    #[test]
    fn target_ir_json_snapshot_is_stable() {
        let schema = SchemaDefinition {
            id: SchemaId::from_uuid(Uuid::from_u128(1)),
            project_id: ProjectId::from_uuid(Uuid::from_u128(2)),
            name: "monster table".into(),
            description: String::new(),
            fields: vec![field(
                FieldId::from_uuid(Uuid::from_u128(3)),
                "display name",
            )],
            target: TargetRule::default(),
        };
        let ir = build_target_ir(&schema, CompilationTarget::TypeScript).expect("valid schema");
        let actual = serde_json::to_string_pretty(&ir).expect("IR should serialize");
        assert_eq!(
            actual.trim(),
            include_str!("../tests/target_ir.json").trim()
        );
    }

    #[test]
    fn audience_filter_keeps_server_fields_out_of_client_artifacts() {
        let mut client_field = field(FieldId::new(), "display_name");
        client_field.target.audiences = vec![crate::Audience::Client];
        let mut server_field = field(FieldId::new(), "server_cost");
        server_field.target.audiences = vec![crate::Audience::Server];
        let schema = SchemaDefinition {
            id: SchemaId::new(),
            project_id: ProjectId::new(),
            name: "Item".into(),
            description: String::new(),
            fields: vec![client_field, server_field],
            target: TargetRule::default(),
        };

        let client = crate::build_target_ir_for_audience(
            &schema,
            CompilationTarget::TypeScript,
            crate::Audience::Client,
        )
        .expect("client IR");
        assert_eq!(client.fields.len(), 1);
        assert_eq!(client.fields[0].source_name, "display_name");
    }
}
