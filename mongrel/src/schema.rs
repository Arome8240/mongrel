use crate::error::MongooseError;

/// Core trait that every schema struct must implement.
///
/// You almost never implement this by hand — use `#[derive(Schema)]` and
/// configure it with `#[schema(...)]` / `#[field(...)]` attributes instead.
///
/// # Manual implementation
///
/// ```rust,ignore
/// use mongrel::schema::MongooseSchema;
/// use mongrel::error::MongooseError;
///
/// impl MongooseSchema for MyStruct {
///     fn collection_name() -> &'static str { "my_collection" }
///
///     fn validate(&self) -> std::result::Result<(), MongooseError> {
///         if self.name.is_empty() {
///             return Err(MongooseError::Validation("name is required".into()));
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait MongooseSchema: Sized + Send + Sync {
    /// The MongoDB collection name for this schema.
    fn collection_name() -> &'static str;

    /// Whether `created_at` / `updated_at` are automatically managed.
    /// Set via `#[schema(timestamps)]`. Defaults to `false`.
    fn timestamps() -> bool { false }

    /// Fields that require a unique index, declared via `#[field(unique)]`.
    /// `Model::ensure_indexes` creates these automatically at startup.
    fn unique_fields() -> &'static [&'static str] { &[] }

    /// Field-level and custom validation. Called during `Model::create` before
    /// any data reaches MongoDB. Return `Err(MongooseError::Validation(...))` to
    /// abort the operation. The derive macro generates this from `#[field(...)]`
    /// attributes; override it for logic that attributes can't express.
    fn validate(&self) -> std::result::Result<(), MongooseError> { Ok(()) }
}

/// Helper trait used internally by the derive macro to extract `&str` from
/// `String` and `Option<String>` fields during validation.
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
