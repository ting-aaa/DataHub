use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    CompilationTarget, CustomTypeId, FieldId, ProjectId, RevisionId, RowId, SchemaId, VariantId,
};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    #[default]
    Client,
    Server,
    Editor,
}

fn default_audiences() -> Vec<Audience> {
    vec![Audience::Client, Audience::Server, Audience::Editor]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRule {
    pub include: Vec<CompilationTarget>,
    #[serde(default = "default_audiences")]
    pub audiences: Vec<Audience>,
    pub rename: BTreeMap<CompilationTarget, String>,
}

impl TargetRule {
    #[must_use]
    pub fn includes(&self, target: CompilationTarget) -> bool {
        self.include.is_empty() || self.include.contains(&target)
    }

    #[must_use]
    pub fn includes_audience(&self, audience: Audience) -> bool {
        self.audiences.is_empty() || self.audiences.contains(&audience)
    }

    #[must_use]
    pub fn emitted_name<'a>(&'a self, target: CompilationTarget, fallback: &'a str) -> &'a str {
        self.rename.get(&target).map_or(fallback, String::as_str)
    }
}

impl Default for TargetRule {
    fn default() -> Self {
        Self {
            include: vec![
                CompilationTarget::Rust,
                CompilationTarget::CSharp,
                CompilationTarget::TypeScript,
            ],
            audiences: default_audiences(),
            rename: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceMode {
    Hard,
    Soft,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeAst {
    Bool,
    Integer {
        min: Option<i64>,
        max: Option<i64>,
    },
    Float {
        min: Option<f64>,
        max: Option<f64>,
    },
    String {
        min_length: Option<usize>,
        max_length: Option<usize>,
    },
    Bytes,
    Date,
    DateTime,
    Optional {
        item: Box<Self>,
    },
    List {
        item: Box<Self>,
        min_items: Option<usize>,
        max_items: Option<usize>,
    },
    FixedArray {
        item: Box<Self>,
        length: usize,
    },
    Set {
        item: Box<Self>,
        min_items: Option<usize>,
        max_items: Option<usize>,
    },
    Map {
        key: Box<Self>,
        value: Box<Self>,
    },
    Struct {
        fields: Vec<FieldDefinition>,
    },
    Enum {
        variants: Vec<EnumVariant>,
    },
    Union {
        variants: Vec<Self>,
    },
    Reference {
        schema_id: SchemaId,
        mode: ReferenceMode,
    },
    Custom {
        custom_type_id: CustomTypeId,
    },
}

impl TypeAst {
    #[must_use]
    pub const fn is_optional(&self) -> bool {
        matches!(self, Self::Optional { .. })
    }

    pub(crate) fn canonicalize(&mut self) {
        match self {
            Self::Optional { item }
            | Self::List { item, .. }
            | Self::FixedArray { item, .. }
            | Self::Set { item, .. } => item.canonicalize(),
            Self::Map { key, value } => {
                key.canonicalize();
                value.canonicalize();
            }
            Self::Struct { fields } => {
                for field in fields.iter_mut() {
                    field.ty.canonicalize();
                }
                fields.sort_by_key(|field| field.id);
            }
            Self::Union { variants } => {
                for variant in variants {
                    variant.canonicalize();
                }
            }
            Self::Bool
            | Self::Integer { .. }
            | Self::Float { .. }
            | Self::String { .. }
            | Self::Bytes
            | Self::Date
            | Self::DateTime
            | Self::Enum { .. }
            | Self::Reference { .. }
            | Self::Custom { .. } => {}
        }
        if let Self::Enum { variants } = self {
            variants.sort_by_key(|variant| variant.id);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumVariant {
    pub id: VariantId,
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub id: FieldId,
    pub name: String,
    pub description: String,
    pub ty: TypeAst,
    pub default: Option<ConfigValue>,
    #[serde(default)]
    pub target: TargetRule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub id: SchemaId,
    pub project_id: ProjectId,
    pub name: String,
    pub description: String,
    pub fields: Vec<FieldDefinition>,
    #[serde(default)]
    pub target: TargetRule,
}

pub type TableDefinition = SchemaDefinition;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomTypeDefinition {
    pub id: CustomTypeId,
    pub project_id: ProjectId,
    pub name: String,
    pub description: String,
    pub ty: TypeAst,
    #[serde(default)]
    pub target: TargetRule,
}

impl SchemaDefinition {
    #[must_use]
    pub fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        for field in &mut canonical.fields {
            field.ty.canonicalize();
        }
        canonical.fields.sort_by_key(|field| field.id);
        canonical
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConfigValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Date(String),
    DateTime(String),
    List(Vec<Self>),
    Set(Vec<Self>),
    FixedArray(Vec<Self>),
    Map(BTreeMap<String, Self>),
    Struct(BTreeMap<FieldId, Self>),
    Enum(VariantId),
    Union {
        variant: usize,
        value: Box<Self>,
    },
    Reference {
        schema_id: SchemaId,
        row_id: RowId,
    },
    Custom {
        custom_type_id: CustomTypeId,
        value: Box<Self>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigRow {
    pub id: RowId,
    pub schema_id: SchemaId,
    pub revision_id: RevisionId,
    pub values: BTreeMap<FieldId, ConfigValue>,
}
