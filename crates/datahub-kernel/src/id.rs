use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            #[must_use]
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value).map(Self)
            }
        }
    };
}

typed_id!(ProjectId);
typed_id!(UserId);
typed_id!(SessionId);
typed_id!(SchemaId);
typed_id!(CustomTypeId);
typed_id!(FieldId);
typed_id!(VariantId);
typed_id!(RowId);
typed_id!(RevisionId);
typed_id!(AuditEventId);
typed_id!(OutboxEventId);
typed_id!(BuildId);
typed_id!(TableViewId);

#[cfg(test)]
mod tests {
    use super::FieldId;
    use uuid::Version;

    #[test]
    fn typed_ids_round_trip_as_strings() {
        let id = FieldId::new();
        let parsed: FieldId = id.to_string().parse().expect("id should parse");
        assert_eq!(parsed, id);
        assert_eq!(id.as_uuid().get_version(), Some(Version::SortRand));
    }
}
