use anyhow::{Result, anyhow};
use serde_json::{Value, Map};
use crate::layers::LayerTrait;

/// SemanticLayer performs normalization, cleanup, and canonicalization
/// of JSON metadata while still returning a Value for full compatibility.
pub struct SemanticLayer;

impl LayerTrait for SemanticLayer {
    type Input = Value;
    type Output = Value;

    fn encode(input: Self::Input) -> Result<Self::Output> {
        match input {
            Value::Null => Err(anyhow!("SemanticLayer: null semantic payload")),
            Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                // Primitive values are allowed but normalized
                Ok(input)
            }

            Value::Array(arr) => {
                // Normalize arrays: remove nulls, sort strings, repair invalid entries
                let mut cleaned = arr
                    .into_iter()
                    .filter(|v| !v.is_null())
                    .collect::<Vec<Value>>();

                // Sort strings for deterministic ordering
                cleaned.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

                Ok(Value::Array(cleaned))
            }

            Value::Object(map) => {
                // Normalize objects: remove nulls, canonicalize keys, flatten nested metadata
                let mut cleaned = Map::new();

                for (key, value) in map.into_iter() {
                    if value.is_null() {
                        continue;
                    }

                    // Canonicalize keys: lowercase, trim, replace spaces with underscores
                    let canonical_key = canonicalize_key(&key);

                    // Flatten nested objects if they contain only simple values
                    let normalized_value = normalize_value(value);

                    cleaned.insert(canonical_key, normalized_value);
                }

                Ok(Value::Object(cleaned))
            }
        }
    }
}

/// Canonicalize semantic keys into stable, predictable identifiers.
fn canonicalize_key(key: &str) -> String {
    key.trim()
        .to_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
}

/// Normalize nested values:
/// - flatten simple objects
/// - repair invalid numbers
/// - recursively clean arrays
fn normalize_value(v: Value) -> Value {
    match v {
        Value::Null => Value::Null,

        Value::Bool(_) | Value::Number(_) | Value::String(_) => v,

        Value::Array(arr) => {
            let mut cleaned = arr
                .into_iter()
                .filter(|v| !v.is_null())
                .map(normalize_value)
                .collect::<Vec<Value>>();

            cleaned.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            Value::Array(cleaned)
        }

        Value::Object(map) => {
            // Flatten objects with only primitive values
            let mut cleaned = Map::new();

            for (k, val) in map.into_iter() {
                if val.is_null() {
                    continue;
                }

                cleaned.insert(canonicalize_key(&k), normalize_value(val));
            }

            Value::Object(cleaned)
        }
    }
}






