# Population (Refs)

Mongrel's `Ref<T>` type lets you store a typed reference to another collection's document. It serializes as a plain `ObjectId` in MongoDB and can be resolved ("populated") into the full document on demand.

---

## Defining a reference field

Use `Ref<T>` anywhere you'd normally store a foreign-key `ObjectId`. The referenced type must implement `MongooseSchema`.

```rust
use mongrel::Ref;

#[derive(Debug, Serialize, Deserialize, Schema, Model)]
#[schema(collection = "posts")]
pub struct Post {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,

    pub title: String,

    /// Stored as ObjectId in MongoDB; can be populated to a full User.
    pub author: Ref<User>,

    /// Optional reference — None means "no organization".
    pub organization: Option<Ref<Organization>>,
}
```

---

## Creating a document with a reference

```rust
let post = posts.create(Post {
    id: None,
    title: "Hello world".into(),
    author: Ref::Id(user.id.unwrap()),          // store the ObjectId
    organization: Some(Ref::Id(org_id)),
}).await?;
```

---

## Implementing `Populate`

`Populate` is a trait you implement to define _which_ refs get resolved and in what order:

```rust
use mongrel::populate::{Populate, resolve_ref, resolve_opt_ref};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
impl Populate for Post {
    async fn populate(mut self, db: Arc<mongodb::Database>) -> mongrel::Result<Self> {
        // Required ref
        self.author = resolve_ref(self.author, Arc::clone(&db)).await?;

        // Optional ref
        self.organization = resolve_opt_ref(self.organization, Arc::clone(&db)).await?;

        Ok(self)
    }
}
```

---

## Using `populate` on fetched documents

```rust
let db = Mongrel::db()?;

let post = posts.find_by_id("64b1f...").await?.unwrap();
let post = post.populate(Arc::clone(&db)).await?;

// Access the populated author
match &post.author {
    Ref::Populated(user) => println!("Author: {}", user.name),
    Ref::Id(id)          => println!("Unpopulated id: {id}"),  // shouldn't happen
}

// Access an optional populated ref
if let Some(Ref::Populated(org)) = &post.organization {
    println!("Organization: {}", org.name);
}
```

---

## Populating a list of documents

```rust
let posts: Vec<Post> = posts.find().exec().await?;

let populated: Vec<Post> = futures_util::future::try_join_all(
    posts.into_iter().map(|p| p.populate(Arc::clone(&db)))
).await?;
```

---

## `Ref<T>` API reference

| Method | Description |
|---|---|
| `Ref::Id(oid)` | Construct an unpopulated ref |
| `Ref::Populated(doc)` | Construct a pre-populated ref (rarely needed manually) |
| `.id()` | `Option<&ObjectId>` — `Some` if unpopulated |
| `.populated()` | `Option<&T>` — `Some` if populated |
| `.is_populated()` | `true` if the document has been loaded |

---

## Helper functions

| Function | Description |
|---|---|
| `resolve_ref(r, db)` | Populate a `Ref<T>`. No-op if already populated. Errors if not found. |
| `resolve_opt_ref(r, db)` | Populate an `Option<Ref<T>>`. Returns `Ok(None)` if the field is `None`. |

---

## Serde behaviour

`Ref<T>` always **deserializes** from a BSON `ObjectId`. It always **serializes** as a BSON `ObjectId` (from the `Id` variant). Attempting to serialize the `Populated` variant returns a serde error — this design prevents accidentally writing a nested subdocument when you intended to write a reference.

If you need to embed a subdocument, use a nested struct directly rather than `Ref<T>`.
