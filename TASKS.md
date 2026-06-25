# mongoose-rust — Implementation Tasks

## In Progress
_All tasks complete._

## Pending
_None._

## Completed
- [x] Workspace scaffold (mongoose + mongoose-macros crates)
- [x] `#[derive(Schema)]` proc macro with field attributes (`required`, `min_length`, `max_length`, `enum_values`, `unique`, `rename`)
- [x] `#[derive(Model)]` proc macro — generates `XModel` struct
- [x] `MongooseSchema` trait
- [x] `Hooks` trait — `pre_save`, `post_save`, `pre_delete`, `post_delete`, `pre_validate`, `post_validate`
- [x] `Model<T>` — `create`, `find`, `find_by_id`, `find_one_and_update`, `find_by_id_and_update`, `find_by_id_and_delete`, `delete_many`, `update_many`, `find_one_and_upsert`, `count_documents`, `ensure_indexes`
- [x] `QueryBuilder` — chainable `where_field`, `eq`, `ne`, `gt`, `gte`, `lt`, `lte`, `in_list`, `nin`, `regex`, `field_exists`, `sort`, `limit`, `skip`, `select`, `exec`, `exec_one`, `count`, `any`
- [x] `Mongoose::connect()` + global `Arc<Database>` handle
- [x] `MongooseError` — `Driver`, `Validation`, `NotFound`, `Serialization`, `InvalidId`
- [x] Timestamps support (`created_at` / `updated_at` auto-inject)
- [x] Basic example (`examples/basic.rs`)
- [x] Population / refs — `Ref<T>`, `resolve_ref()`, `Populate` trait
- [x] Virtuals — `Virtuals` trait + `WithVirtuals<T>` serialization wrapper
- [x] Aggregation pipeline builder — `Model::aggregate()` with `$match`, `$group`, `$lookup`, `$project`, `$sort`, `$limit`, `$unwind`, `$count`, `$addFields`, `$replaceRoot`, raw stage escape hatch
- [x] Pagination — `Model::paginate(page, per_page)` → `PaginatedResult<T>` (total, total_pages, has_next, has_prev)
- [x] Middleware chaining — `MiddlewareRegistry<T>` with ordered async hooks per event; `ModelWithMiddleware<T>`
- [x] Lean queries — `.lean()` / `.lean_one()` on `QueryBuilder` returning raw `Document`
- [x] Index builder — `IndexBuilder` (field, field_desc, text, unique, sparse, ttl, partial_filter); `MongooseIndexes` trait; `Model::ensure_custom_indexes()`
- [x] Integration tests — 6 test cases with `testcontainers` + real MongoDB (create, validation, query builder, update/delete, pagination, lean, count/delete_many)
