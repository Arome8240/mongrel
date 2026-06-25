use crate::error::MongooseError;

/// Core trait every schema struct must implement.
/// Derived automatically via `#[derive(Schema)]`.
pub trait MongooseSchema: Sized + Send + Sync {
    fn collection_name() -> &'static str;
    fn timestamps() -> bool { false }
    fn unique_fields() -> &'static [&'static str] { &[] }
    fn validate(&self) -> std::result::Result<(), MongooseError> { Ok(()) }
}

/// Helper trait used by the macro to extract `&str` from String / Option<String>.
pub trait AsStr {
    fn as_str_opt(&self) -> Option<&str>;
}

impl AsStr for String {
    fn as_str_opt(&self) -> Option<&str> { Some(self.as_str()) }
}

impl AsStr for Option<String> {
    fn as_str_opt(&self) -> Option<&str> { self.as_deref() }
}

impl AsStr for str {
    fn as_str_opt(&self) -> Option<&str> { Some(self) }
}
