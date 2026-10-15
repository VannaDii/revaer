use std::fmt;
use std::marker::PhantomData;

use serde::de::{IntoDeserializer, MapAccess, Visitor, value::MapAccessDeserializer};
use serde::{Deserialize, Deserializer};

// Derived structs and unit enums also admit arrays and tagged objects in JSON.
// Restrict their entry points while retaining Serde's duplicate-field checks.
#[derive(Debug)]
pub(super) struct JsonObject<T>(pub(super) T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for JsonObject<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
            type Value = JsonObject<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                T::deserialize(MapAccessDeserializer::new(map)).map(JsonObject)
            }
        }

        deserializer.deserialize_map(ObjectVisitor(PhantomData))
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct JsonString<T>(pub(super) T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for JsonString<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        T::deserialize(value.into_deserializer()).map(Self)
    }
}
