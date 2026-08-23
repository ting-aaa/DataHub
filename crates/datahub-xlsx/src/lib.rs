//! Stable-ID XLSX interchange for `DataHub` configuration tables.

use std::{collections::BTreeMap, io::Cursor, str::FromStr};

use calamine::{Data, DataType, Reader, Xlsx};
use datahub_kernel::{
    ConfigRow, ConfigValue, FieldId, RevisionId, RowId, SchemaDefinition, SchemaId,
};
use rust_xlsxwriter::{Format, Workbook, Worksheet};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const FORMAT_MARKER: &str = "datahub-xlsx-v1";
const DATA_SHEET: &str = "Data";
const META_SHEET: &str = "__datahub";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionedRow {
    pub row: ConfigRow,
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedRow {
    pub row: ConfigRow,
    pub expected_version: Option<i64>,
    pub source_row: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XlsxImport {
    pub schema_id: SchemaId,
    pub revision_id: RevisionId,
    pub rows: Vec<ImportedRow>,
}

/// Exports a configuration table with stable field and row identities in a hidden sheet.
///
/// # Errors
/// Returns an error when values cannot be serialized or the workbook cannot be written.
pub fn export_workbook(
    schema: &SchemaDefinition,
    revision_id: RevisionId,
    rows: &[VersionedRow],
) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold();
    {
        let data = workbook.add_worksheet();
        data.set_name(DATA_SHEET)?;
        for (column, field) in schema.fields.iter().enumerate() {
            data.write_with_format(0, column_index(column)?, &field.name, &header)?;
        }
        for (row_index, versioned) in rows.iter().enumerate() {
            for (column, field) in schema.fields.iter().enumerate() {
                let value = versioned
                    .row
                    .values
                    .get(&field.id)
                    .unwrap_or(&ConfigValue::Null);
                write_value(data, data_row(row_index)?, column_index(column)?, value)?;
            }
        }
    }
    {
        let metadata = workbook.add_worksheet();
        metadata.set_name(META_SHEET)?;
        metadata.set_hidden(true);
        metadata.write_string(0, 0, FORMAT_MARKER)?;
        metadata.write_string(1, 0, "schema_id")?;
        metadata.write_string(1, 1, schema.id.to_string())?;
        metadata.write_string(2, 0, "revision_id")?;
        metadata.write_string(2, 1, revision_id.to_string())?;
        metadata.write_string(4, 0, "field_column")?;
        metadata.write_string(4, 1, "field_id")?;
        for (column, field) in schema.fields.iter().enumerate() {
            let row = u32_index(column, 5)?;
            metadata.write_string(row, 0, column.to_string())?;
            metadata.write_string(row, 1, field.id.to_string())?;
        }
        let row_start = u32_index(schema.fields.len(), 6)?;
        metadata.write_string(row_start, 0, "data_row")?;
        metadata.write_string(row_start, 1, "row_id")?;
        metadata.write_string(row_start, 2, "version")?;
        for (index, versioned) in rows.iter().enumerate() {
            let row = u32_index(
                index,
                usize::try_from(row_start).map_err(|_| XlsxError::Range)? + 1,
            )?;
            metadata.write_string(row, 0, data_row(index)?.to_string())?;
            metadata.write_string(row, 1, versioned.row.id.to_string())?;
            metadata.write_string(row, 2, versioned.version.to_string())?;
        }
    }
    workbook.save_to_buffer().map_err(XlsxError::Writer)
}

/// Imports a `DataHub` workbook using hidden stable-ID metadata rather than display headers.
///
/// Formula cells are accepted only when Excel has stored a cached result.
///
/// # Errors
/// Returns an error for foreign/stale metadata, missing formula caches, or invalid values.
pub fn import_workbook(
    bytes: &[u8],
    schema: &SchemaDefinition,
    current_revision: RevisionId,
) -> Result<XlsxImport, XlsxError> {
    let cursor = Cursor::new(bytes);
    let mut workbook: Xlsx<_> = Xlsx::new(cursor)?;
    let metadata = workbook.worksheet_range(META_SHEET)?;
    let marker = text_at(&metadata, 0, 0)?;
    if marker != FORMAT_MARKER {
        return Err(XlsxError::InvalidMetadata("format marker"));
    }
    let schema_id = SchemaId::from_str(text_at(&metadata, 1, 1)?)
        .map_err(|_| XlsxError::InvalidMetadata("schema id"))?;
    if schema_id != schema.id {
        return Err(XlsxError::ForeignSchema {
            expected: schema.id,
            actual: schema_id,
        });
    }
    let revision_id = RevisionId::from_str(text_at(&metadata, 2, 1)?)
        .map_err(|_| XlsxError::InvalidMetadata("revision id"))?;
    if revision_id != current_revision {
        return Err(XlsxError::StaleRevision {
            expected: current_revision,
            actual: revision_id,
        });
    }

    let mut columns = BTreeMap::new();
    let mut metadata_row = 5_usize;
    while let Some(column) = usize_at_optional(&metadata, metadata_row, 0) {
        let field_id = FieldId::from_str(text_at(&metadata, metadata_row, 1)?)
            .map_err(|_| XlsxError::InvalidMetadata("field id"))?;
        if !schema.fields.iter().any(|field| field.id == field_id) {
            return Err(XlsxError::InvalidMetadata("unknown field id"));
        }
        columns.insert(field_id, column);
        metadata_row += 1;
    }
    metadata_row += 2;

    let data = workbook.worksheet_range(DATA_SHEET)?;
    let formulas = workbook.worksheet_formula(DATA_SHEET)?;
    let mut identities = BTreeMap::new();
    while let Some(data_row_number) = usize_at_optional(&metadata, metadata_row, 0) {
        let row_id = RowId::from_str(text_at(&metadata, metadata_row, 1)?)
            .map_err(|_| XlsxError::InvalidMetadata("row id"))?;
        let version = integer_at(&metadata, metadata_row, 2)?;
        identities.insert(data_row_number, (row_id, version));
        metadata_row += 1;
    }

    let last_data_row = data
        .height()
        .max(formulas.height())
        .max(identities.keys().next_back().map_or(0, |row| row + 1));
    let mut rows = Vec::new();
    for source_row in 1..last_data_row {
        if !identities.contains_key(&source_row)
            && row_is_empty(&data, source_row, &columns)
            && row_is_empty_formula(&formulas, source_row, &columns)
        {
            continue;
        }
        let (row_id, expected_version) = identities.get(&source_row).map_or_else(
            || (RowId::new(), None),
            |(id, version)| (*id, Some(*version)),
        );
        let mut values = BTreeMap::new();
        for field in &schema.fields {
            let column = *columns
                .get(&field.id)
                .ok_or(XlsxError::InvalidMetadata("missing field mapping"))?;
            let has_formula =
                formula_at(&formulas, source_row, column).is_some_and(|value| !value.is_empty());
            let cell = data.get((source_row, column));
            if has_formula && cell.is_none_or(DataType::is_empty) {
                return Err(XlsxError::MissingFormulaCache {
                    row: source_row + 1,
                    column: column + 1,
                });
            }
            values.insert(
                field.id,
                parse_cell(cell.unwrap_or(&Data::Empty), &field.ty)?,
            );
        }
        rows.push(ImportedRow {
            row: ConfigRow {
                id: row_id,
                schema_id,
                revision_id,
                values,
            },
            expected_version,
            source_row: u32::try_from(source_row + 1).map_err(|_| XlsxError::Range)?,
        });
    }

    Ok(XlsxImport {
        schema_id,
        revision_id,
        rows,
    })
}

fn write_value(
    sheet: &mut Worksheet,
    row: u32,
    column: u16,
    value: &ConfigValue,
) -> Result<(), XlsxError> {
    match value {
        ConfigValue::Null => {
            sheet.write_blank(row, column, &Format::new())?;
        }
        ConfigValue::Bool(value) => {
            sheet.write_boolean(row, column, *value)?;
        }
        ConfigValue::Integer(value) => {
            sheet.write_string(row, column, value.to_string())?;
        }
        ConfigValue::Float(value) => {
            sheet.write_number(row, column, *value)?;
        }
        ConfigValue::String(value) | ConfigValue::Date(value) | ConfigValue::DateTime(value) => {
            sheet.write_string(row, column, value)?;
        }
        other => {
            sheet.write_string(row, column, serde_json::to_string(other)?)?;
        }
    }
    Ok(())
}

fn parse_cell(cell: &Data, ty: &datahub_kernel::TypeAst) -> Result<ConfigValue, XlsxError> {
    use datahub_kernel::TypeAst;
    if cell.is_empty() {
        return Ok(ConfigValue::Null);
    }
    match ty {
        TypeAst::Bool => cell.get_bool().map(ConfigValue::Bool),
        TypeAst::Integer { .. } => cell.as_i64().map(ConfigValue::Integer),
        TypeAst::Float { .. } => cell.as_f64().map(ConfigValue::Float),
        TypeAst::String { .. } => cell.as_string().map(ConfigValue::String),
        TypeAst::Date => cell.as_string().map(ConfigValue::Date),
        TypeAst::DateTime => cell.as_string().map(ConfigValue::DateTime),
        TypeAst::Optional { item } => return parse_cell(cell, item),
        _ => cell
            .as_string()
            .and_then(|value| serde_json::from_str(&value).ok()),
    }
    .ok_or_else(|| XlsxError::InvalidValue(cell.to_string()))
}

fn text_at(range: &calamine::Range<Data>, row: usize, column: usize) -> Result<&str, XlsxError> {
    range
        .get((row, column))
        .and_then(DataType::get_string)
        .ok_or(XlsxError::InvalidMetadata("text cell"))
}

fn usize_at_optional(range: &calamine::Range<Data>, row: usize, column: usize) -> Option<usize> {
    range
        .get((row, column))
        .and_then(DataType::as_string)?
        .parse()
        .ok()
}

fn integer_at(range: &calamine::Range<Data>, row: usize, column: usize) -> Result<i64, XlsxError> {
    range
        .get((row, column))
        .and_then(DataType::as_i64)
        .ok_or(XlsxError::InvalidMetadata("integer cell"))
}

fn row_is_empty(
    range: &calamine::Range<Data>,
    row: usize,
    columns: &BTreeMap<FieldId, usize>,
) -> bool {
    columns
        .values()
        .all(|column| range.get((row, *column)).is_none_or(DataType::is_empty))
}

fn row_is_empty_formula(
    range: &calamine::Range<String>,
    row: usize,
    columns: &BTreeMap<FieldId, usize>,
) -> bool {
    columns
        .values()
        .all(|column| formula_at(range, row, *column).is_none_or(String::is_empty))
}

fn formula_at(range: &calamine::Range<String>, row: usize, column: usize) -> Option<&String> {
    range.get_value((u32::try_from(row).ok()?, u32::try_from(column).ok()?))
}

fn column_index(index: usize) -> Result<u16, XlsxError> {
    u16::try_from(index).map_err(|_| XlsxError::Range)
}
fn data_row(index: usize) -> Result<u32, XlsxError> {
    u32_index(index, 1)
}
fn u32_index(index: usize, offset: usize) -> Result<u32, XlsxError> {
    u32::try_from(index.checked_add(offset).ok_or(XlsxError::Range)?).map_err(|_| XlsxError::Range)
}

#[derive(Debug, Error)]
pub enum XlsxError {
    #[error("XLSX writer error: {0}")]
    Writer(#[from] rust_xlsxwriter::XlsxError),
    #[error("XLSX reader error: {0}")]
    Reader(#[from] calamine::XlsxError),
    #[error("JSON value error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid DataHub XLSX metadata: {0}")]
    InvalidMetadata(&'static str),
    #[error("workbook belongs to schema {actual}, expected {expected}")]
    ForeignSchema {
        expected: SchemaId,
        actual: SchemaId,
    },
    #[error("workbook revision {actual} is stale; expected {expected}")]
    StaleRevision {
        expected: RevisionId,
        actual: RevisionId,
    },
    #[error("formula at row {row}, column {column} has no cached result")]
    MissingFormulaCache { row: usize, column: usize },
    #[error("invalid cell value: {0}")]
    InvalidValue(String),
    #[error("XLSX range exceeds supported limits")]
    Range,
}
