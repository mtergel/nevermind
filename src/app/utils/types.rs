use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, sqlx::Type)]
pub struct Timestamptz(pub OffsetDateTime);

impl Serialize for Timestamptz {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = self.0.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
        serializer.collect_str(&formatted)
    }
}

impl Deserialize<'_> for Timestamptz {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'_>,
    {
        struct StrVisitor;

        impl<'de> Visitor<'de> for StrVisitor {
            type Value = Timestamptz;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("expected a valid RFC 3339 date string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                OffsetDateTime::parse(v, &Rfc3339)
                    .map(Timestamptz)
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_str(StrVisitor)
    }
}

impl From<OffsetDateTime> for Timestamptz {
    fn from(offset_datetime: OffsetDateTime) -> Self {
        Timestamptz(offset_datetime)
    }
}
