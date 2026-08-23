use std::collections::BTreeMap;

use datahub_kernel::{
    Audience, CompilationTarget, ConfigRow, ConfigValue, SchemaDefinition, TargetIr, TargetType,
    ValidationIssue, build_target_ir, build_target_ir_for_audience,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("target compilation failed")]
    Validation(Vec<ValidationIssue>),
    #[error("serialization failed: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Artifact {
    pub path: String,
    pub media_type: String,
    pub sha256: String,
    pub content: Vec<u8>,
}

impl Artifact {
    #[must_use]
    pub fn new(path: impl Into<String>, media_type: impl Into<String>, content: Vec<u8>) -> Self {
        let sha256 = format!("{:x}", Sha256::digest(&content));
        Self {
            path: path.into(),
            media_type: media_type.into(),
            sha256,
            content,
        }
    }
}

/// Generates a deterministic source-code artifact from target-aware IR.
///
/// # Errors
/// Returns validation errors when the schema cannot be compiled for the target.
pub fn generate_code(
    schema: &SchemaDefinition,
    target: CompilationTarget,
) -> Result<Artifact, ExportError> {
    generate_code_internal(schema, target, None)
}

/// Generates source code filtered for a C/S/E audience.
///
/// # Errors
/// Returns target validation diagnostics.
pub fn generate_code_for_audience(
    schema: &SchemaDefinition,
    target: CompilationTarget,
    audience: Audience,
) -> Result<Artifact, ExportError> {
    generate_code_internal(schema, target, Some(audience))
}

fn generate_code_internal(
    schema: &SchemaDefinition,
    target: CompilationTarget,
    audience: Option<Audience>,
) -> Result<Artifact, ExportError> {
    let ir = compile_ir(schema, target, audience)?;
    let (extension, media_type, content) = match target {
        CompilationTarget::Rust => ("rs", "text/x-rust", render_rust(&ir)),
        CompilationTarget::CSharp => ("cs", "text/x-csharp", render_csharp(&ir)),
        CompilationTarget::TypeScript => ("ts", "text/typescript", render_typescript(&ir)),
    };
    Ok(Artifact::new(
        format!(
            "code/{}/{}.{}",
            target_name(target),
            ir.emitted_name,
            extension
        ),
        media_type,
        content.into_bytes(),
    ))
}

/// Serializes rows as deterministic JSON using emitted field names.
///
/// # Errors
/// Returns validation or JSON serialization errors.
pub fn generate_json(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
) -> Result<Artifact, ExportError> {
    generate_json_internal(schema, rows, target, None)
}

/// Serializes rows as audience-filtered deterministic JSON.
///
/// # Errors
/// Returns target or serialization diagnostics.
pub fn generate_json_for_audience(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
    audience: Audience,
) -> Result<Artifact, ExportError> {
    generate_json_internal(schema, rows, target, Some(audience))
}

fn generate_json_internal(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
    audience: Option<Audience>,
) -> Result<Artifact, ExportError> {
    let ir = compile_ir(schema, target, audience)?;
    let records = projected_records(&ir, rows);
    let content = serde_json::to_vec_pretty(&records)
        .map_err(|error| ExportError::Serialization(error.to_string()))?;
    Ok(Artifact::new(
        format!("data/{}/{}.json", target_name(target), ir.emitted_name),
        "application/json",
        content,
    ))
}

/// Serializes rows as deterministic CSV. Nested values use JSON cells.
///
/// # Errors
/// Returns validation or CSV serialization errors.
pub fn generate_csv(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
) -> Result<Artifact, ExportError> {
    generate_csv_internal(schema, rows, target, None)
}

/// Serializes rows as audience-filtered deterministic CSV.
///
/// # Errors
/// Returns target or serialization diagnostics.
pub fn generate_csv_for_audience(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
    audience: Audience,
) -> Result<Artifact, ExportError> {
    generate_csv_internal(schema, rows, target, Some(audience))
}

fn generate_csv_internal(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
    audience: Option<Audience>,
) -> Result<Artifact, ExportError> {
    let ir = compile_ir(schema, target, audience)?;
    let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
    writer
        .write_record(ir.fields.iter().map(|field| field.emitted_name.as_str()))
        .map_err(|error| ExportError::Serialization(error.to_string()))?;
    for record in projected_records(&ir, rows) {
        writer
            .write_record(ir.fields.iter().map(|field| {
                let value = record.get(&field.emitted_name).cloned().unwrap_or_default();
                match value {
                    serde_json::Value::Null => String::new(),
                    serde_json::Value::String(value) => value,
                    other => other.to_string(),
                }
            }))
            .map_err(|error| ExportError::Serialization(error.to_string()))?;
    }
    let content = writer
        .into_inner()
        .map_err(|error| ExportError::Serialization(error.to_string()))?;
    Ok(Artifact::new(
        format!("data/{}/{}.csv", target_name(target), ir.emitted_name),
        "text/csv",
        content,
    ))
}

fn compile_ir(
    schema: &SchemaDefinition,
    target: CompilationTarget,
    audience: Option<Audience>,
) -> Result<TargetIr, ExportError> {
    audience
        .map_or_else(
            || build_target_ir(schema, target),
            |audience| build_target_ir_for_audience(schema, target, audience),
        )
        .map_err(ExportError::Validation)
}

fn projected_records(
    ir: &TargetIr,
    rows: &[ConfigRow],
) -> Vec<BTreeMap<String, serde_json::Value>> {
    let mut rows = rows.to_vec();
    rows.sort_by_key(|row| row.id);
    rows.into_iter()
        .map(|row| {
            ir.fields
                .iter()
                .map(|field| {
                    let value = row
                        .values
                        .get(&field.id)
                        .map_or(serde_json::Value::Null, config_to_json);
                    (field.emitted_name.clone(), value)
                })
                .collect()
        })
        .collect()
}

fn config_to_json(value: &ConfigValue) -> serde_json::Value {
    match value {
        ConfigValue::Null => serde_json::Value::Null,
        ConfigValue::Bool(value) => (*value).into(),
        ConfigValue::Integer(value) => (*value).into(),
        ConfigValue::Float(value) => serde_json::Number::from_f64(*value)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        ConfigValue::String(value) | ConfigValue::Date(value) | ConfigValue::DateTime(value) => {
            value.clone().into()
        }
        ConfigValue::Bytes(value) => value.iter().copied().map(serde_json::Value::from).collect(),
        ConfigValue::List(values) | ConfigValue::Set(values) | ConfigValue::FixedArray(values) => {
            values.iter().map(config_to_json).collect()
        }
        ConfigValue::Map(values) => values
            .iter()
            .map(|(key, value)| (key.clone(), config_to_json(value)))
            .collect(),
        ConfigValue::Struct(values) => values
            .iter()
            .map(|(key, value)| (key.to_string(), config_to_json(value)))
            .collect(),
        ConfigValue::Enum(value) => value.to_string().into(),
        ConfigValue::Union { variant, value } => serde_json::json!({
            "variant": variant,
            "value": config_to_json(value),
        }),
        ConfigValue::Reference { schema_id, row_id } => serde_json::json!({
            "schema_id": schema_id,
            "row_id": row_id,
        }),
        ConfigValue::Custom {
            custom_type_id,
            value,
        } => serde_json::json!({
            "custom_type_id": custom_type_id,
            "value": config_to_json(value),
        }),
    }
}

fn render_rust(ir: &TargetIr) -> String {
    let fields = ir
        .fields
        .iter()
        .map(|field| format!("    pub {}: {},", field.emitted_name, rust_type(&field.ty)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "// Generated by DataHub. Do not edit.\n#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub struct {} {{\n{}\n}}\n",
        ir.emitted_name, fields
    )
}

fn render_csharp(ir: &TargetIr) -> String {
    let fields = ir
        .fields
        .iter()
        .map(|field| {
            format!(
                "    public {} {} {{ get; init; }}",
                csharp_type(&field.ty),
                field.emitted_name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "// Generated by DataHub. Do not edit.\nnamespace DataHub.Generated;\n\npublic sealed record {}\n{{\n{}\n}}\n",
        ir.emitted_name, fields
    )
}

fn render_typescript(ir: &TargetIr) -> String {
    let fields = ir
        .fields
        .iter()
        .map(|field| format!("  {}: {};", field.emitted_name, typescript_type(&field.ty)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "// Generated by DataHub. Do not edit.\nexport interface {} {{\n{}\n}}\n",
        ir.emitted_name, fields
    )
}

fn rust_type(ty: &TargetType) -> String {
    type_name(ty, Language::Rust)
}

fn csharp_type(ty: &TargetType) -> String {
    type_name(ty, Language::CSharp)
}

fn typescript_type(ty: &TargetType) -> String {
    type_name(ty, Language::TypeScript)
}

#[derive(Clone, Copy)]
enum Language {
    Rust,
    CSharp,
    TypeScript,
}

fn type_name(ty: &TargetType, language: Language) -> String {
    match ty {
        TargetType::Bool => match language {
            Language::TypeScript => "boolean",
            Language::Rust | Language::CSharp => "bool",
        }
        .into(),
        TargetType::I64 | TargetType::Enum { .. } => match language {
            Language::Rust => "i64",
            Language::CSharp => "long",
            Language::TypeScript => "number",
        }
        .into(),
        TargetType::F64 => match language {
            Language::Rust => "f64",
            Language::CSharp => "double",
            Language::TypeScript => "number",
        }
        .into(),
        TargetType::String
        | TargetType::Date
        | TargetType::DateTime
        | TargetType::Reference { .. }
        | TargetType::Custom { .. } => match language {
            Language::Rust => "String",
            Language::CSharp | Language::TypeScript => "string",
        }
        .into(),
        TargetType::Bytes => match language {
            Language::Rust => "Vec<u8>",
            Language::CSharp => "byte[]",
            Language::TypeScript => "number[]",
        }
        .into(),
        TargetType::Optional { item } => match language {
            Language::Rust => format!("Option<{}>", type_name(item, language)),
            Language::CSharp => format!("{}?", type_name(item, language)),
            Language::TypeScript => format!("{} | null", type_name(item, language)),
        },
        TargetType::List { item } | TargetType::Set { item } => match language {
            Language::Rust => format!("Vec<{}>", type_name(item, language)),
            Language::CSharp => format!("IReadOnlyList<{}>", type_name(item, language)),
            Language::TypeScript => format!("Array<{}>", type_name(item, language)),
        },
        TargetType::FixedArray { item, length } => match language {
            Language::Rust => format!("[{}; {length}]", type_name(item, language)),
            Language::CSharp => format!(
                "IReadOnlyList<{}> /* fixed {length} */",
                type_name(item, language)
            ),
            Language::TypeScript => {
                format!("Array<{}> /* fixed {length} */", type_name(item, language))
            }
        },
        TargetType::Map { .. } | TargetType::Struct { .. } | TargetType::Union { .. } => {
            match language {
                Language::Rust => "serde_json::Value",
                Language::CSharp => "object",
                Language::TypeScript => "unknown",
            }
            .into()
        }
    }
}

const fn target_name(target: CompilationTarget) -> &'static str {
    match target {
        CompilationTarget::Rust => "rust",
        CompilationTarget::CSharp => "csharp",
        CompilationTarget::TypeScript => "typescript",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use datahub_kernel::{
        CompilationTarget, ConfigRow, ConfigValue, FieldDefinition, FieldId, ProjectId, RevisionId,
        RowId, SchemaDefinition, SchemaId, TargetRule, TypeAst,
    };

    use super::{generate_code, generate_csv, generate_json};

    fn fixture() -> (SchemaDefinition, Vec<ConfigRow>, FieldId) {
        let field_id = FieldId::new();
        let schema = SchemaDefinition {
            id: SchemaId::new(),
            project_id: ProjectId::new(),
            name: "item table".into(),
            description: String::new(),
            fields: vec![FieldDefinition {
                id: field_id,
                name: "item id".into(),
                description: String::new(),
                ty: TypeAst::Integer {
                    min: Some(1),
                    max: None,
                },
                default: None,
                target: TargetRule::default(),
            }],
            target: TargetRule::default(),
        };
        let row = ConfigRow {
            id: RowId::new(),
            schema_id: schema.id,
            revision_id: RevisionId::new(),
            values: BTreeMap::from([(field_id, ConfigValue::Integer(1001))]),
        };
        (schema, vec![row], field_id)
    }

    #[test]
    fn emits_all_three_language_shapes() {
        let (schema, _, _) = fixture();
        let rust = generate_code(&schema, CompilationTarget::Rust).expect("rust");
        let csharp = generate_code(&schema, CompilationTarget::CSharp).expect("csharp");
        let typescript = generate_code(&schema, CompilationTarget::TypeScript).expect("ts");
        assert!(
            String::from_utf8(rust.content)
                .expect("utf8")
                .contains("pub struct ItemTable")
        );
        assert!(
            String::from_utf8(csharp.content)
                .expect("utf8")
                .contains("record ItemTable")
        );
        assert!(
            String::from_utf8(typescript.content)
                .expect("utf8")
                .contains("interface ItemTable")
        );
    }

    #[test]
    fn emits_json_and_csv_values() {
        let (schema, rows, _) = fixture();
        let json = generate_json(&schema, &rows, CompilationTarget::Rust).expect("json");
        let csv = generate_csv(&schema, &rows, CompilationTarget::Rust).expect("csv");
        assert!(
            String::from_utf8(json.content)
                .expect("utf8")
                .contains("1001")
        );
        assert_eq!(
            String::from_utf8(csv.content).expect("utf8"),
            "itemId\n1001\n"
        );
    }

    #[test]
    fn identical_inputs_have_identical_hashes() {
        let (schema, rows, _) = fixture();
        let first = generate_json(&schema, &rows, CompilationTarget::Rust).expect("json");
        let second = generate_json(&schema, &rows, CompilationTarget::Rust).expect("json");
        assert_eq!(first.sha256, second.sha256);
    }
}
