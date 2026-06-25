use async_trait::async_trait;
use mongrel::{
    bson::doc,
    hooks::Hooks,
    error::Result,
    populate::{Populate, Ref, resolve_ref},
    Model, Mongrel, Schema, SortDir,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Company schema ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Schema, Model)]
#[schema(collection = "companies", timestamps)]
pub struct Company {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub name: String,
}

#[async_trait]
impl Hooks for Company {}

// ── User schema ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Schema, Model)]
#[schema(collection = "users", timestamps)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,

    #[field(required, min_length = 2, max_length = 50)]
    pub name: String,

    #[field(required, unique)]
    pub email: String,

    #[field(enum_values = "admin, user, moderator")]
    pub role: String,

    pub age: Option<i32>,

    /// Reference to a Company document
    pub company: Option<Ref<Company>>,
}

#[async_trait]
impl Hooks for User {
    async fn pre_save(&mut self) -> Result<()> {
        println!("[hook] pre_save: {}", self.name);
        Ok(())
    }
    async fn post_save(&self) -> Result<()> {
        println!("[hook] post_save: saved");
        Ok(())
    }
}

// ── Populate impl ─────────────────────────────────────────────────────────────

#[async_trait]
impl Populate for User {
    async fn populate(mut self, db: Arc<mongodb::Database>) -> Result<Self> {
        self.company = resolve_ref(
            self.company.unwrap_or(mongrel::Ref::Id(Default::default())),
            Arc::clone(&db),
        )
        .await
        .ok()
        .map(Some)
        .unwrap_or(None);
        Ok(self)
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let db = Mongrel::connect("mongodb://localhost:27017", "mongrel_example").await?;

    let company_model = CompanyModel::new(Arc::clone(&db));
    let user_model = UserModel::new(Arc::clone(&db));

    user_model.ensure_indexes().await?;

    // Create a company
    let company = company_model.create(Company {
        id: None,
        name: "Acme Corp".into(),
    }).await?;

    // Create a user referencing that company
    let user = user_model.create(User {
        id: None,
        name: "Alice".into(),
        email: "alice@example.com".into(),
        role: "admin".into(),
        age: Some(30),
        company: company.id.map(mongrel::Ref::Id),
    }).await?;
    println!("Created: {:?}", user);

    // Find with builder
    let adults = user_model
        .find()
        .where_field("age").gte(18)
        .sort("name", SortDir::Asc)
        .limit(10)
        .exec()
        .await?;
    println!("Adults: {} found", adults.len());

    // Populate company reference
    for u in adults {
        let populated = u.populate(Arc::clone(&db)).await?;
        println!("User {} works at {:?}", populated.name, populated.company);
    }

    // Find by id
    if let Some(id) = &user.id {
        let updated = user_model.find_by_id_and_update(
            &id.to_hex(),
            doc! { "$set": { "name": "Alice Updated" } },
        ).await?;
        println!("Updated: {:?}", updated);

        user_model.find_by_id_and_delete(&id.to_hex()).await?;
        println!("Deleted");
    }

    Ok(())
}
