# mongrel-macros

Internal proc-macro crate for [Mongrel](https://crates.io/crates/mongrel) — a Mongoose-style MongoDB ODM for Rust.

**Do not depend on this crate directly.** It is an implementation detail of `mongrel` and re-exported automatically when you add `mongrel` to your dependencies.

```toml
[dependencies]
mongrel = "0.1"   # this is all you need
```

---

## What this crate provides

Two derive macros, used via the `mongrel` crate:

### `#[derive(Schema)]`

Generates an implementation of `MongooseSchema` from struct and field attributes.

```rust
#[derive(Serialize, Deserialize, Schema)]
#[schema(collection = "users", timestamps)]
pub struct User {
    #[field(required, min_length = 2, unique)]
    pub name: String,

    #[field(required, unique)]
    pub email: String,

    #[field(enum_values = "admin, user, moderator")]
    pub role: String,
}
```

Supported `#[schema(...)]` options:

| Option | Description |
|---|---|
| `collection = "name"` | MongoDB collection name (defaults to snake_case plural) |
| `timestamps` | Auto-manage `created_at` / `updated_at` fields |

Supported `#[field(...)]` options:

| Option | Description |
|---|---|
| `required` | Field must be non-empty at validation time |
| `unique` | Creates a unique index via `ensure_indexes()` |
| `min_length = N` | Minimum string length |
| `max_length = N` | Maximum string length |
| `enum_values = "a, b, c"` | Comma-separated list of allowed values |
| `rename = "mongo_name"` | Override the MongoDB field name |

### `#[derive(Model)]`

Generates a `{Name}Model` struct that wraps `Model<T>` and exposes all CRUD, query, aggregation, and pagination methods.

```rust
#[derive(Serialize, Deserialize, Schema, Model)]
pub struct User { /* ... */ }

// Generated:
// pub struct UserModel { /* ... */ }
// impl UserModel { pub fn new(db: Arc<Database>) -> Self { ... } }
```

---

## License

MIT — see the [main repository](https://github.com/Arome8240/mongrel).
