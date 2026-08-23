use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};

use datahub_kernel::{
    ConfigRow, ConfigValue, FieldDefinition, FieldId, ProjectId, RevisionId, RowId,
    SchemaDefinition, SchemaId, TargetRule, TypeAst,
};
use datahub_xlsx::{VersionedRow, XlsxError, export_workbook, import_workbook};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

fn schema() -> SchemaDefinition {
    SchemaDefinition {
        id: SchemaId::new(),
        project_id: ProjectId::new(),
        name: "Inventory".into(),
        description: String::new(),
        fields: vec![
            FieldDefinition {
                id: FieldId::new(),
                name: "quantity".into(),
                description: String::new(),
                ty: TypeAst::Integer {
                    min: None,
                    max: None,
                },
                default: None,
                target: TargetRule::default(),
            },
            FieldDefinition {
                id: FieldId::new(),
                name: "enabled".into(),
                description: String::new(),
                ty: TypeAst::Bool,
                default: None,
                target: TargetRule::default(),
            },
            FieldDefinition {
                id: FieldId::new(),
                name: "label".into(),
                description: String::new(),
                ty: TypeAst::String {
                    min_length: None,
                    max_length: None,
                },
                default: None,
                target: TargetRule::default(),
            },
        ],
        target: TargetRule::default(),
    }
}

fn versioned_row(schema: &SchemaDefinition, revision_id: RevisionId) -> VersionedRow {
    VersionedRow {
        row: ConfigRow {
            id: RowId::new(),
            schema_id: schema.id,
            revision_id,
            values: BTreeMap::from([
                (schema.fields[0].id, ConfigValue::Integer(i64::MAX)),
                (schema.fields[1].id, ConfigValue::Bool(true)),
                (schema.fields[2].id, ConfigValue::String("主数据".into())),
            ]),
        },
        version: 41,
    }
}

#[test]
fn round_trip_preserves_stable_identity_values_and_version() {
    let schema = schema();
    let revision_id = RevisionId::new();
    let source = versioned_row(&schema, revision_id);
    let bytes = export_workbook(&schema, revision_id, std::slice::from_ref(&source)).unwrap();
    let imported = import_workbook(&bytes, &schema, revision_id).unwrap();

    assert_eq!(imported.schema_id, schema.id);
    assert_eq!(imported.revision_id, revision_id);
    assert_eq!(imported.rows.len(), 1);
    assert_eq!(imported.rows[0].row, source.row);
    assert_eq!(imported.rows[0].expected_version, Some(41));
}

#[test]
fn display_header_rename_does_not_change_field_mapping() {
    let schema = schema();
    let revision_id = RevisionId::new();
    let source = versioned_row(&schema, revision_id);
    let mut renamed = schema.clone();
    renamed.fields[0].name = "库存数量（已重命名）".into();
    let bytes = export_workbook(&renamed, revision_id, std::slice::from_ref(&source)).unwrap();

    let imported = import_workbook(&bytes, &schema, revision_id).unwrap();
    assert_eq!(imported.rows[0].row.values, source.row.values);
}

#[test]
fn stale_and_foreign_workbooks_are_rejected() {
    let schema = schema();
    let revision_id = RevisionId::new();
    let bytes = export_workbook(&schema, revision_id, &[]).unwrap();

    assert!(matches!(
        import_workbook(&bytes, &schema, RevisionId::new()),
        Err(XlsxError::StaleRevision { .. })
    ));

    let mut foreign = schema.clone();
    foreign.id = SchemaId::new();
    assert!(matches!(
        import_workbook(&bytes, &foreign, revision_id),
        Err(XlsxError::ForeignSchema { .. })
    ));
}

#[test]
fn entirely_empty_existing_rows_keep_identity() {
    let schema = schema();
    let revision_id = RevisionId::new();
    let source = VersionedRow {
        row: ConfigRow {
            id: RowId::new(),
            schema_id: schema.id,
            revision_id,
            values: BTreeMap::new(),
        },
        version: 7,
    };
    let bytes = export_workbook(&schema, revision_id, std::slice::from_ref(&source)).unwrap();
    let imported = import_workbook(&bytes, &schema, revision_id).unwrap();

    assert_eq!(imported.rows.len(), 1);
    assert_eq!(imported.rows[0].row.id, source.row.id);
    assert_eq!(imported.rows[0].expected_version, Some(7));
}

#[test]
fn formula_without_cached_value_is_rejected() {
    let schema = schema();
    let revision_id = RevisionId::new();
    let source = versioned_row(&schema, revision_id);
    let bytes = export_workbook(&schema, revision_id, &[source]).unwrap();
    let bytes = replace_first_data_cell_with_uncached_formula(&bytes);

    let result = import_workbook(&bytes, &schema, revision_id);
    assert!(
        matches!(
            result,
            Err(XlsxError::MissingFormulaCache { row: 2, column: 1 })
        ),
        "unexpected import result: {result:?}"
    );
}

fn replace_first_data_cell_with_uncached_formula(bytes: &[u8]) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        entries.push((
            file.name().to_owned(),
            file.is_dir(),
            file.compression(),
            data,
        ));
    }

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, is_dir, compression, mut data) in entries {
        if name == "xl/worksheets/sheet1.xml" {
            let xml = String::from_utf8(data).unwrap();
            let marker = xml.find("r=\"A2\"").unwrap();
            let start = xml[..marker].rfind("<c").unwrap();
            let end = marker + xml[marker..].find("</c>").unwrap() + 4;
            data =
                format!("{}<c r=\"A2\"><f>1+1</f></c>{}", &xml[..start], &xml[end..]).into_bytes();
        }
        let options = SimpleFileOptions::default().compression_method(match compression {
            CompressionMethod::Stored => CompressionMethod::Stored,
            _ => CompressionMethod::Deflated,
        });
        if is_dir {
            writer.add_directory(name, options).unwrap();
        } else {
            writer.start_file(name, options).unwrap();
            writer.write_all(&data).unwrap();
        }
    }
    writer.finish().unwrap().into_inner()
}
