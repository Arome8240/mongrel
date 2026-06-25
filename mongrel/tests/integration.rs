use async_trait::async_trait;
use mongrel::{
    bson::doc,
    hooks::Hooks,
    index::{IndexBuilder, MongooseIndexes},
    Model, Schema, SortDir,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mongo::Mongo;

// ── Test schema ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Schema, Model)]
#[schema(collection = "test_users", timestamps)]
struct TestUser {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<bson::oid::ObjectId>,

    #[field(required, min_length = 2)]
    name: String,

    #[field(required, unique)]
    email: String,

    age: Option<i32>,
}

#[async_trait]
impl Hooks for TestUser {}

impl MongooseIndexes for TestUser {
    fn indexes() -> Vec<mongrel::IndexDef> {
        vec![
            IndexBuilder::new()
                .field("name")
                .field_desc("age")
                .name("name_age_idx")
                .build(),
        ]
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn setup() -> (
    testcontainers::ContainerAsync<Mongo>,
    mongrel::model::Model<TestUser>,
) {
    let container = Mongo::default().start().await.expect("mongo container");
    let port = container
        .get_host_port_ipv4(27017)
        .await
        .expect("port");
    let uri = format!("mongodb://127.0.0.1:{}", port);

    let client = mongodb::Client::with_uri_str(&uri)
        .await
        .expect("client");
    let db = Arc::new(client.database("test_db"));
    let model = TestUserModel::new(Arc::clone(&db));
    model.ensure_indexes().await.expect("indexes");
    model.ensure_custom_indexes().await.expect("custom indexes");
    (container, model)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_and_find_by_id() {
    let (_c, model) = setup().await;

    let user = model
        .create(TestUser {
            id: None,
            name: "Alice".into(),
            email: "alice@test.com".into(),
            age: Some(25),
        })
        .await
        .expect("create");

    let id = user.id.expect("has id").to_hex();
    let found = model.find_by_id(&id).await.expect("find").expect("exists");
    assert_eq!(found.name, "Alice");
    assert_eq!(found.email, "alice@test.com");
}

#[tokio::test]
async fn test_validation_min_length() {
    let (_c, model) = setup().await;

    let err = model
        .create(TestUser {
            id: None,
            name: "A".into(), // too short
            email: "x@test.com".into(),
            age: None,
        })
        .await;

    assert!(err.is_err());
    let msg = err.unwrap_err().to_string();
    assert!(msg.contains("name"), "error should mention the field: {msg}");
}

#[tokio::test]
async fn test_query_builder() {
    let (_c, model) = setup().await;

    for (name, email, age) in [
        ("Bob", "bob@test.com", 30),
        ("Carol", "carol@test.com", 17),
        ("Dave", "dave@test.com", 45),
    ] {
        model
            .create(TestUser { id: None, name: name.into(), email: email.into(), age: Some(age) })
            .await
            .expect("create");
    }

    let adults = model
        .find()
        .where_field("age")
        .gte(18)
        .sort("name", SortDir::Asc)
        .exec()
        .await
        .expect("find");

    assert_eq!(adults.len(), 2);
    assert_eq!(adults[0].name, "Bob");
    assert_eq!(adults[1].name, "Dave");
}

#[tokio::test]
async fn test_update_and_delete() {
    let (_c, model) = setup().await;

    let user = model
        .create(TestUser {
            id: None,
            name: "Eve".into(),
            email: "eve@test.com".into(),
            age: Some(22),
        })
        .await
        .expect("create");

    let id = user.id.expect("id").to_hex();

    let updated = model
        .find_by_id_and_update(&id, doc! { "$set": { "name": "Eve Updated" } })
        .await
        .expect("update")
        .expect("exists");
    assert_eq!(updated.name, "Eve Updated");

    model.find_by_id_and_delete(&id).await.expect("delete");
    let gone = model.find_by_id(&id).await.expect("find after delete");
    assert!(gone.is_none());
}

#[tokio::test]
async fn test_pagination() {
    let (_c, model) = setup().await;

    for i in 0..10u32 {
        model
            .create(TestUser {
                id: None,
                name: format!("User{i:02}"),
                email: format!("user{i}@test.com"),
                age: Some(i as i32),
            })
            .await
            .expect("create");
    }

    let page = model
        .paginate(2, 3)
        .sort(doc! { "name": 1 })
        .exec()
        .await
        .expect("paginate");

    assert_eq!(page.total, 10);
    assert_eq!(page.total_pages, 4);
    assert_eq!(page.docs.len(), 3);
    assert!(page.has_prev);
    assert!(page.has_next);
}

#[tokio::test]
async fn test_lean_query() {
    let (_c, model) = setup().await;

    model
        .create(TestUser {
            id: None,
            name: "Frank".into(),
            email: "frank@test.com".into(),
            age: Some(33),
        })
        .await
        .expect("create");

    let docs = model.find().lean().await.expect("lean");
    assert!(!docs.is_empty());
    assert!(docs[0].contains_key("name"));
}

#[tokio::test]
async fn test_count_and_delete_many() {
    let (_c, model) = setup().await;

    for i in 0..5u32 {
        model
            .create(TestUser {
                id: None,
                name: format!("Bulk{i}"),
                email: format!("bulk{i}@test.com"),
                age: Some(i as i32),
            })
            .await
            .expect("create");
    }

    let count = model.count_documents(doc! {}).await.expect("count");
    assert_eq!(count, 5);

    let deleted = model
        .delete_many(doc! { "age": { "$lt": 3 } })
        .await
        .expect("delete_many");
    assert_eq!(deleted, 3);

    let remaining = model.count_documents(doc! {}).await.expect("count");
    assert_eq!(remaining, 2);
}
