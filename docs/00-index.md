# Mongrel Documentation

A Mongoose-style MongoDB ODM for Rust.

## Guides

| # | Topic | Description |
|---|---|---|
| 1 | [Schema Definition](./01-schema.md) | `#[derive(Schema)]`, field attributes, custom validation |
| 2 | [Model CRUD](./02-model-crud.md) | create, find, update, delete, upsert, count |
| 3 | [Query Builder](./03-query-builder.md) | Chainable filters, sort, limit, skip, lean |
| 4 | [Population (Refs)](./04-population.md) | `Ref<T>`, `resolve_ref`, `Populate` trait |
| 5 | [Aggregation Pipeline](./05-aggregation.md) | `$match`, `$group`, `$lookup`, `$project`, and more |
| 6 | [Pagination](./06-pagination.md) | `paginate(page, per_page)` → `PaginatedResult<T>` |
| 7 | [Index Builder](./07-indexes.md) | Compound, sparse, TTL, text, partial indexes |
| 8 | [Hooks & Middleware](./08-hooks-middleware.md) | Lifecycle hooks, `MiddlewareRegistry<T>` |

## Quick links

- [README](../README.md) — project overview and quick start
- [examples/basic.rs](../mongrel/examples/basic.rs) — end-to-end runnable example
- [tests/integration.rs](../mongrel/tests/integration.rs) — integration test suite
