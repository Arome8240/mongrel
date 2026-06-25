/// Mongoose-style virtuals: computed properties derived from document fields.
/// Implement this trait on your schema struct and define any computed values.
///
/// Example:
/// ```ignore
/// impl Virtuals for User {
///     fn full_name(&self) -> String {
///         format!("{} {}", self.first_name, self.last_name)
///     }
/// }
/// ```
///
/// Virtuals are never serialized to MongoDB — they are computed on the fly.
pub trait Virtuals {}

// ── VirtualField helper ───────────────────────────────────────────────────────
// Wraps a T plus a set of computed string virtuals by name.
// Used when you need to serialize a document + its virtuals as one JSON object
// (e.g. for an API response).

use std::collections::HashMap;
use serde::{Serialize, Serializer};

pub struct WithVirtuals<T: Serialize> {
    inner: T,
    virtuals: HashMap<String, serde_json::Value>,
}

impl<T: Serialize> WithVirtuals<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            virtuals: HashMap::new(),
        }
    }

    pub fn add(mut self, name: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.virtuals.insert(name.into(), value.into());
        self
    }
}

impl<T: Serialize> Serialize for WithVirtuals<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let inner_value = serde_json::to_value(&self.inner).map_err(serde::ser::Error::custom)?;

        let mut obj = match inner_value {
            serde_json::Value::Object(m) => m,
            _ => return Err(serde::ser::Error::custom("WithVirtuals: inner must be an object")),
        };

        for (k, v) in &self.virtuals {
            obj.insert(k.clone(), v.clone());
        }

        serde_json::Value::Object(obj).serialize(ser)
    }
}
