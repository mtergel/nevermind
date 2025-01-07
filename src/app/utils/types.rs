use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::de;
use serde::{Serialize, Serializer};
use std::fmt::Formatter;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::Type)]
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

impl<'de> de::Deserialize<'de> for Timestamptz {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct StrVisitor;

        impl de::Visitor<'_> for StrVisitor {
            type Value = Timestamptz;

            fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
                f.write_str("expected a valid RFC 3339 date string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
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

#[derive(Debug)]
pub struct CPagination {
    pub id: Uuid,
    pub created_at: Timestamptz,
}

impl Serialize for CPagination {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // TODO: Duplicate impl
        let formatted = self
            .created_at
            .0
            .format(&Rfc3339)
            .map_err(serde::ser::Error::custom)?;

        // Order is important, match with deserializer
        let input = format!("{},{}", self.id, formatted);
        let encoded = URL_SAFE.encode(input);

        serializer.collect_str(&encoded)
    }
}

impl<'de> de::Deserialize<'de> for CPagination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct StrVisitor;

        impl de::Visitor<'_> for StrVisitor {
            type Value = CPagination;

            fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
                f.write_str("expected a valid cursor string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match URL_SAFE.decode(v) {
                    Ok(decoded_bytes) => {
                        let param_str = String::from_utf8(decoded_bytes).map_err(E::custom)?;

                        let parts: Vec<&str> = param_str.split(",").collect();
                        if parts.len() != 2 {
                            return Err(E::custom("malformed cursor"));
                        }

                        let id = Uuid::try_parse(parts[0]).map_err(E::custom)?;
                        let created_at =
                            OffsetDateTime::parse(parts[1], &Rfc3339).map_err(E::custom)?;
                        let created_at = Timestamptz(created_at);

                        Ok(CPagination { id, created_at })
                    }
                    Err(e) => Err(E::custom(e)),
                }
            }
        }

        deserializer.deserialize_str(StrVisitor)
    }
}
