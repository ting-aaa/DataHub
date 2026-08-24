use std::collections::BTreeMap;
use std::fmt::Write as _;

use datahub_kernel::{
    Audience, CompilationTarget, ConfigRow, ConfigValue, FieldId, RevisionId, RowId,
    SchemaDefinition, SchemaId, TargetIr, TargetType, ValidationIssue, build_target_ir,
    build_target_ir_for_audience,
};
use quick_xml::escape::escape;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("target compilation failed")]
    Validation(Vec<ValidationIssue>),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("protobuf wire id {wire_id} collides for fields {first} and {second}")]
    ProtobufWireIdCollision {
        wire_id: u32,
        first: FieldId,
        second: FieldId,
    },
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

/// Serializes rows as deterministic XML with explicit field elements.
///
/// # Errors
/// Returns target validation errors.
pub fn generate_xml_for_audience(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
    audience: Audience,
) -> Result<Artifact, ExportError> {
    let ir = compile_ir(schema, target, Some(audience))?;
    let records = projected_records(&ir, rows);
    let mut content = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<datahub>\n");
    for record in records {
        content.push_str("  <row>\n");
        for field in &ir.fields {
            let value = record.get(&field.emitted_name).cloned().unwrap_or_default();
            let encoded = match value {
                serde_json::Value::String(value) => value,
                other => other.to_string(),
            };
            writeln!(
                content,
                "    <field name=\"{}\">{}</field>",
                escape(&field.emitted_name),
                escape(&encoded)
            )
            .expect("writing to a String is infallible");
        }
        content.push_str("  </row>\n");
    }
    content.push_str("</datahub>\n");
    Ok(Artifact::new(
        format!("data/{}/{}.xml", target_name(target), ir.emitted_name),
        "application/xml",
        content.into_bytes(),
    ))
}

#[derive(Serialize, Deserialize)]
struct BsonEnvelope {
    records: Vec<BTreeMap<String, serde_json::Value>>,
}

/// Serializes rows as a deterministic BSON document.
///
/// # Errors
/// Returns target or BSON serialization errors.
pub fn generate_bson_for_audience(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
    audience: Audience,
) -> Result<Artifact, ExportError> {
    let ir = compile_ir(schema, target, Some(audience))?;
    let content = bson::serialize_to_vec(&BsonEnvelope {
        records: projected_records(&ir, rows),
    })
    .map_err(|error| ExportError::Serialization(error.to_string()))?;
    Ok(Artifact::new(
        format!("data/{}/{}.bson", target_name(target), ir.emitted_name),
        "application/bson",
        content,
    ))
}

/// Serializes rows as a deterministic Lua table module.
///
/// # Errors
/// Returns target validation errors.
pub fn generate_lua_for_audience(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
    audience: Audience,
) -> Result<Artifact, ExportError> {
    let ir = compile_ir(schema, target, Some(audience))?;
    let mut content = String::from("-- Generated by DataHub. Do not edit.\nreturn {\n");
    for record in projected_records(&ir, rows) {
        content.push_str("  {\n");
        for field in &ir.fields {
            let value = record.get(&field.emitted_name).cloned().unwrap_or_default();
            writeln!(
                content,
                "    {} = {},",
                field.emitted_name,
                json_to_lua(&value)
            )
            .expect("writing to a String is infallible");
        }
        content.push_str("  },\n");
    }
    content.push_str("}\n");
    Ok(Artifact::new(
        format!("data/{}/{}.lua", target_name(target), ir.emitted_name),
        "text/x-lua",
        content.into_bytes(),
    ))
}

/// Generates a deterministic `.proto` schema and binary data artifact.
///
/// Stable wire IDs are derived solely from immutable `FieldId` values. A rare collision
/// is rejected instead of silently renumbering either field.
///
/// # Errors
/// Returns target validation or wire-ID collision errors.
pub fn generate_protobuf_for_audience(
    schema: &SchemaDefinition,
    rows: &[ConfigRow],
    target: CompilationTarget,
    audience: Audience,
) -> Result<Vec<Artifact>, ExportError> {
    let ir = compile_ir(schema, target, Some(audience))?;
    let mut wire_ids = BTreeMap::new();
    for field in &ir.fields {
        let wire_id = protobuf_wire_id(field.id);
        if let Some(first) = wire_ids.insert(wire_id, field.id) {
            return Err(ExportError::ProtobufWireIdCollision {
                wire_id,
                first,
                second: field.id,
            });
        }
    }

    let mut schema_text = String::from(
        "// Generated by DataHub. Stable tags derive from immutable FieldIds.\nsyntax = \"proto3\";\npackage datahub.generated;\n\n",
    );
    writeln!(schema_text, "message {} {{", ir.emitted_name)
        .expect("writing to a String is infallible");
    for field in &ir.fields {
        writeln!(
            schema_text,
            "  {} {} = {}; // FieldId {}",
            protobuf_type(&field.ty),
            field.emitted_name,
            protobuf_wire_id(field.id),
            field.id
        )
        .expect("writing to a String is infallible");
    }
    write!(
        schema_text,
        "}}\n\nmessage {}Data {{\n  repeated {} rows = 1;\n}}\n",
        ir.emitted_name, ir.emitted_name
    )
    .expect("writing to a String is infallible");

    let mut binary = Vec::new();
    let mut sorted_rows = rows.to_vec();
    sorted_rows.sort_by_key(|row| row.id);
    for row in &sorted_rows {
        let mut encoded_row = Vec::new();
        for field in &ir.fields {
            if let Some(value) = row.values.get(&field.id) {
                encode_protobuf_field(
                    protobuf_wire_id(field.id),
                    &field.ty,
                    value,
                    &mut encoded_row,
                );
            }
        }
        encode_length_delimited(1, &encoded_row, &mut binary);
    }
    let base = format!("data/{}/{}", target_name(target), ir.emitted_name);
    Ok(vec![
        Artifact::new(
            format!("{base}.proto"),
            "text/x-protobuf",
            schema_text.into_bytes(),
        ),
        Artifact::new(format!("{base}.pb"), "application/x-protobuf", binary),
    ])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRowInput {
    pub row_id: RowId,
    pub row_revision_id: RevisionId,
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildSchemaInput {
    pub schema_id: SchemaId,
    pub schema_revision_id: RevisionId,
    pub schema_version: i64,
    pub data_revision_id: Option<RevisionId>,
    pub rows: Vec<BuildRowInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildArtifactDigest {
    pub path: String,
    pub media_type: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildManifest {
    pub format: String,
    pub target: CompilationTarget,
    pub audience: Audience,
    pub schemas: Vec<BuildSchemaInput>,
    pub plugin_versions: BTreeMap<String, String>,
    pub artifacts: Vec<BuildArtifactDigest>,
}

/// Produces a timestamp-free manifest whose bytes are a deterministic build fingerprint.
///
/// # Panics
/// Panics only if serializing the manifest's JSON-compatible values fails.
#[must_use]
pub fn generate_build_manifest(
    target: CompilationTarget,
    audience: Audience,
    mut schemas: Vec<BuildSchemaInput>,
    plugin_versions: BTreeMap<String, String>,
    artifacts: &[Artifact],
) -> Artifact {
    schemas.sort_by_key(|schema| schema.schema_id);
    for schema in &mut schemas {
        schema.rows.sort_by_key(|row| row.row_id);
    }
    let mut artifact_digests = artifacts
        .iter()
        .map(|artifact| BuildArtifactDigest {
            path: artifact.path.clone(),
            media_type: artifact.media_type.clone(),
            sha256: artifact.sha256.clone(),
        })
        .collect::<Vec<_>>();
    artifact_digests.sort_by(|left, right| left.path.cmp(&right.path));
    let manifest = BuildManifest {
        format: "datahub-build-v1".into(),
        target,
        audience,
        schemas,
        plugin_versions,
        artifacts: artifact_digests,
    };
    let content =
        serde_json::to_vec_pretty(&manifest).expect("manifest serialization is infallible");
    Artifact::new("manifest.json", "application/json", content)
}

#[must_use]
pub fn protobuf_wire_id(field_id: FieldId) -> u32 {
    let uuid = field_id.as_uuid();
    let bytes = uuid.as_bytes();
    let raw = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let mut wire_id = raw % 536_870_911 + 1;
    if (19_000..=19_999).contains(&wire_id) {
        wire_id += 1_000;
    }
    wire_id
}

fn json_to_lua(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "nil".into(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => format!("{value:?}"),
        serde_json::Value::Array(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(json_to_lua)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        serde_json::Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!("[{:?}] = {}", key, json_to_lua(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn protobuf_type(ty: &TargetType) -> &'static str {
    match ty {
        TargetType::Bool => "bool",
        TargetType::I64 => "int64",
        TargetType::F64 => "double",
        TargetType::String | TargetType::Date | TargetType::DateTime | TargetType::Enum { .. } => {
            "string"
        }
        TargetType::Optional { item } => protobuf_type(item),
        TargetType::Bytes
        | TargetType::Reference { .. }
        | TargetType::Custom { .. }
        | TargetType::List { .. }
        | TargetType::Set { .. }
        | TargetType::FixedArray { .. }
        | TargetType::Map { .. }
        | TargetType::Struct { .. }
        | TargetType::Union { .. } => "bytes",
    }
}

fn encode_protobuf_field(
    field_number: u32,
    ty: &TargetType,
    value: &ConfigValue,
    output: &mut Vec<u8>,
) {
    match (ty, value) {
        (_, ConfigValue::Null) => {}
        (TargetType::Optional { item }, value) => {
            encode_protobuf_field(field_number, item, value, output);
        }
        (TargetType::Bool, ConfigValue::Bool(value)) => {
            encode_key(field_number, 0, output);
            encode_varint(u64::from(*value), output);
        }
        (TargetType::I64, ConfigValue::Integer(value)) => {
            encode_key(field_number, 0, output);
            encode_varint(value.cast_unsigned(), output);
        }
        (TargetType::F64, ConfigValue::Float(value)) => {
            encode_key(field_number, 1, output);
            output.extend_from_slice(&value.to_le_bytes());
        }
        (
            TargetType::String | TargetType::Date | TargetType::DateTime,
            ConfigValue::String(value) | ConfigValue::Date(value) | ConfigValue::DateTime(value),
        ) => encode_length_delimited(field_number, value.as_bytes(), output),
        (TargetType::Enum { .. }, ConfigValue::Enum(value)) => {
            encode_length_delimited(field_number, value.to_string().as_bytes(), output);
        }
        (TargetType::Bytes, ConfigValue::Bytes(value)) => {
            encode_length_delimited(field_number, value, output);
        }
        (_, value) => {
            let encoded = serde_json::to_vec(&config_to_json(value)).unwrap_or_default();
            encode_length_delimited(field_number, &encoded, output);
        }
    }
}

fn encode_length_delimited(field_number: u32, value: &[u8], output: &mut Vec<u8>) {
    encode_key(field_number, 2, output);
    encode_varint(value.len() as u64, output);
    output.extend_from_slice(value);
}

fn encode_key(field_number: u32, wire_type: u8, output: &mut Vec<u8>) {
    encode_varint(
        (u64::from(field_number) << 3) | u64::from(wire_type),
        output,
    );
}

fn encode_varint(mut value: u64, output: &mut Vec<u8>) {
    while value >= 0x80 {
        let low_bits = u8::try_from(value & 0x7f).expect("masked varint byte fits in u8");
        output.push(low_bits | 0x80);
        value >>= 7;
    }
    output.push(u8::try_from(value).expect("final varint byte fits in u8"));
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
                "    public required {} {} {{ get; init; }}",
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
        Audience, CompilationTarget, ConfigRow, ConfigValue, FieldDefinition, FieldId, ProjectId,
        RevisionId, RowId, SchemaDefinition, SchemaId, TargetRule, TypeAst,
    };
    use quick_xml::{Reader, events::Event};
    use uuid::Uuid;

    use super::{
        Artifact, BsonEnvelope, BuildManifest, BuildRowInput, BuildSchemaInput, ExportError,
        generate_bson_for_audience, generate_build_manifest, generate_code, generate_csv,
        generate_json, generate_lua_for_audience, generate_protobuf_for_audience,
        generate_xml_for_audience, protobuf_wire_id,
    };

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

    #[test]
    fn emits_parseable_xml_bson_protobuf_and_lua() {
        let (schema, rows, field_id) = fixture();
        let xml =
            generate_xml_for_audience(&schema, &rows, CompilationTarget::Rust, Audience::Client)
                .expect("xml");
        let mut reader = Reader::from_reader(xml.content.as_slice());
        let mut row_elements = 0;
        loop {
            match reader.read_event().expect("valid XML") {
                Event::Start(element) if element.name().as_ref() == b"row" => row_elements += 1,
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(row_elements, 1);

        let bson =
            generate_bson_for_audience(&schema, &rows, CompilationTarget::Rust, Audience::Client)
                .expect("bson");
        let envelope: BsonEnvelope =
            bson::deserialize_from_slice(&bson.content).expect("valid BSON");
        assert_eq!(
            envelope.records[0].get("itemId"),
            Some(&serde_json::json!(1001))
        );

        let protobuf = generate_protobuf_for_audience(
            &schema,
            &rows,
            CompilationTarget::Rust,
            Audience::Client,
        )
        .expect("protobuf");
        let schema_text = String::from_utf8(protobuf[0].content.clone()).expect("UTF-8 proto");
        assert!(schema_text.contains(&format!("itemId = {}", protobuf_wire_id(field_id))));
        let (outer_key, used) = decode_varint(&protobuf[1].content);
        assert_eq!(outer_key, 10);
        let (row_length, length_used) = decode_varint(&protobuf[1].content[used..]);
        let row_start = used + length_used;
        assert_eq!(
            usize::try_from(row_length).expect("test row length fits in usize"),
            protobuf[1].content.len() - row_start
        );
        let (field_key, _) = decode_varint(&protobuf[1].content[row_start..]);
        assert_eq!(field_key, u64::from(protobuf_wire_id(field_id)) << 3);

        let lua =
            generate_lua_for_audience(&schema, &rows, CompilationTarget::Rust, Audience::Client)
                .expect("lua");
        assert!(
            String::from_utf8(lua.content)
                .expect("UTF-8 Lua")
                .contains("itemId = 1001")
        );
    }

    #[test]
    fn protobuf_wire_ids_survive_renames_and_reject_collisions() {
        let (mut schema, rows, field_id) = fixture();
        let original = generate_protobuf_for_audience(
            &schema,
            &rows,
            CompilationTarget::Rust,
            Audience::Client,
        )
        .expect("original protobuf");
        schema.fields[0].name = "renamed item id".into();
        let renamed = generate_protobuf_for_audience(
            &schema,
            &rows,
            CompilationTarget::Rust,
            Audience::Client,
        )
        .expect("renamed protobuf");
        assert_eq!(original[1].content, renamed[1].content);
        assert!(
            String::from_utf8(renamed[0].content.clone())
                .expect("UTF-8 proto")
                .contains(&format!("= {}", protobuf_wire_id(field_id)))
        );

        let first = FieldId::from_uuid(Uuid::from_bytes([1; 16]));
        let mut second_bytes = [1; 16];
        second_bytes[15] = 2;
        let second = FieldId::from_uuid(Uuid::from_bytes(second_bytes));
        assert_eq!(protobuf_wire_id(first), protobuf_wire_id(second));
        let field_template = schema.fields[0].clone();
        schema.fields = vec![
            FieldDefinition {
                id: first,
                name: "first".into(),
                ..field_template.clone()
            },
            FieldDefinition {
                id: second,
                name: "second".into(),
                ..field_template
            },
        ];
        let error =
            generate_protobuf_for_audience(&schema, &[], CompilationTarget::Rust, Audience::Client)
                .expect_err("collision must be rejected");
        assert!(matches!(error, ExportError::ProtobufWireIdCollision { .. }));
    }

    #[test]
    fn build_manifest_is_order_independent_and_input_sensitive() {
        let (schema, rows, _) = fixture();
        let artifacts = vec![
            generate_json(&schema, &rows, CompilationTarget::Rust).expect("json"),
            generate_csv(&schema, &rows, CompilationTarget::Rust).expect("csv"),
        ];
        let input = BuildSchemaInput {
            schema_id: schema.id,
            schema_revision_id: RevisionId::new(),
            schema_version: 3,
            data_revision_id: Some(RevisionId::new()),
            rows: vec![BuildRowInput {
                row_id: rows[0].id,
                row_revision_id: rows[0].revision_id,
                version: 7,
            }],
        };
        let plugins = BTreeMap::from([("built-in".into(), "0.1.0".into())]);
        let first = generate_build_manifest(
            CompilationTarget::Rust,
            Audience::Client,
            vec![input.clone()],
            plugins.clone(),
            &artifacts,
        );
        let second = generate_build_manifest(
            CompilationTarget::Rust,
            Audience::Client,
            vec![input.clone()],
            plugins.clone(),
            &artifacts.iter().cloned().rev().collect::<Vec<Artifact>>(),
        );
        assert_eq!(first, second);
        let decoded: BuildManifest = serde_json::from_slice(&first.content).expect("manifest JSON");
        assert_eq!(decoded.schemas, vec![input.clone()]);

        let mut changed = input;
        changed.schema_version += 1;
        let third = generate_build_manifest(
            CompilationTarget::Rust,
            Audience::Client,
            vec![changed],
            plugins,
            &artifacts,
        );
        assert_ne!(first.sha256, third.sha256);
    }

    fn decode_varint(bytes: &[u8]) -> (u64, usize) {
        let mut value = 0_u64;
        for (index, byte) in bytes.iter().copied().enumerate() {
            value |= u64::from(byte & 0x7f) << (index * 7);
            if byte & 0x80 == 0 {
                return (value, index + 1);
            }
        }
        panic!("unterminated varint");
    }
}
