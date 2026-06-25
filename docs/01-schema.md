# Schema Definition

A Mongrel schema is a plain Rust struct annotated with `#[derive(Schema, Model)]`. The derive macros generate all the boilerplate — collection name, validation, timestamps, and index declarations — from attributes you write directly on the struct and its fields.

---

## Minimal schema

```rust
use mongrel::{Schema, Model};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Schema, Model)]
pub struct Article {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,

    pub title: String,
    pub body:  String,
}
```

This is enough to use `ArticleModel`. The collection name defaults to `articles` (snake_case plural of the struct name).

---

## `#[schema(...)]` — struct-level options

### `collection = "name"`

Override the default collection name:

```rust
#[schema(collection = "blog_articles")]
pub struct Article { /* ... */ }
```

### `timestamps`

Automatically inject `created_at` and `updated_at` (`bson::DateTime`) on every create and update:

```rust
#[schema(collection = "articles", timestamps)]
pub struct Article { /* ... */ }
```

When enabled:
- `create()` writes both fields to the document before insertion.
- `find_one_and_update()` / `find_by_id_and_update()` overwrite `updated_at` inside your `$set`.

The fields are added at the BSON level, not to the Rust struct — so you don't need to declare them unless you want to read them back:

```rust
pub struct Article {
    // Declare these if you want to deserialize them:
    pub created_at: Option<bson::DateTime>,
    pub updated_at: Option<bson::DateTime>,
}
```

---

## `#[field(...)]` — field-level options

### `required`

Marks the field as required. Currently serves as documentation; pair it with `min_length = 1` to actively enforce non-emptiness for strings.

```rust
#[field(required)]
pub title: String,
```

### `unique`

Creates a unique index on this field when you call `model.ensure_indexes()`.

```rust
#[field(unique)]
pub email: String,
```

Multiple fields can be marked unique — each gets its own single-field unique index. For **compound** unique indexes, use [`MongooseIndexes`](./07-indexes.md).

### `min_length = N` / `max_length = N`

Enforce character-count bounds on `String` and `Option<String>` fields. Validation runs inside `create()` before any write reaches MongoDB.

```rust
#[field(min_length = 3, max_length = 100)]
pub username: String,

#[field(max_length = 500)]
pub bio: Option<String>,
```

### `enum_values = "value1, value2, value3"`

Restrict the field to a comma-separated list of allowed string values.

```rust
#[field(enum_values = "draft, published, archived")]
pub status: String,
```

### `rename = "mongo_field_name"`

Store the field under a different key in MongoDB. Useful when your Rust naming convention differs from your database convention.

```rust
#[field(rename = "firstName")]
pub first_name: String,
```

---

## Custom validation

`#[field(...)]` attributes cover the most common rules. For anything more complex, implement `MongooseSchema` manually and override `validate()`. You can import and call the derived implementation first:

```rust
use mongrel::schema::MongooseSchema;
use mongrel::error::MongooseError;

// NOTE: you can only manually implement MongooseSchema if you remove
// #[derive(Schema)] — otherwise there will be conflicting impls.
// The pattern below uses a separate validation method instead:

impl Article {
    fn custom_validate(&self) -> std::result::Result<(), MongooseError> {
        if self.title.contains('<') || self.title.contains('>') {
            return Err(MongooseError::Validation(
                "title must not contain HTML tags".into()
            ));
        }
        if self.body.len() < self.title.len() {
            return Err(MongooseError::Validation(
                "body must be longer than the title".into()
            ));
        }
        Ok(())
    }
}
```

Then call it from your `pre_validate` hook:

```rust
use async_trait::async_trait;
use mongrel::hooks::Hooks;
use mongrel::error::Result;

#[async_trait]
impl Hooks for Article {
    async fn pre_validate(&self) -> Result<()> {
        self.custom_validate()?;
        Ok(())
    }
}
```

---

## Full example

```rust
use mongrel::{Schema, Model};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, Schema, Model)]
#[schema(collection = "products", timestamps)]
pub struct Product {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,

    #[field(required, min_length = 2, max_length = 120)]
    pub name: String,

    #[field(required, unique)]
    pub sku: String,

    #[field(enum_values = "electronics, clothing, food, other")]
    pub category: String,

    #[field(min_length = 0, max_length = 2000)]
    pub description: Option<String>,

    pub price_cents: u64,
    pub stock: u32,

    pub created_at: Option<bson::DateTime>,
    pub updated_at: Option<bson::DateTime>,
}
```
