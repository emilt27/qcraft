# rquery

Universal, extensible SQL query builder for Rust.

Build SQL statements as typed AST nodes, then render them to parameterized SQL
for any supported dialect. One AST, many backends.

## Features

- **AST-first design** -- queries are data structures, not string templates
- **PostgreSQL + SQLite** support out of the box, more dialects planned
- **Parameterized queries** -- every render returns `(String, Vec<Value>)`, never interpolates user data
- **Extensible via Custom traits** -- add custom expressions, conditions, compare operators, table sources, and mutations without forking
- **Convenience API** -- helpers like `SelectColumn::field()`, `Conditions::gte()`, `InsertStmt::values()` to build statements in a few lines
- **Full SQL coverage** -- SELECT, INSERT, UPDATE, DELETE, DDL (CREATE/ALTER/DROP), transactions, CTEs, window functions, set operations, locking, upsert

## Installation

```sh
cargo add rquery
```

Both `postgres` and `sqlite` features are enabled by default. To use only one:

```sh
cargo add rquery --no-default-features --features postgres
cargo add rquery --no-default-features --features sqlite
```

## Quick Start

### SELECT with WHERE

```rust
use rquery::ast::common::{FieldRef, SchemaRef};
use rquery::ast::conditions::Conditions;
use rquery::ast::query::{FromItem, QueryStmt, SelectColumn};

let stmt = QueryStmt {
    columns: vec![SelectColumn::field("users", "name")],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    where_clause: Some(Conditions::gte(FieldRef::new("users", "age"), 18)),
    ..Default::default()
};
```

### Rendering -- PostgreSQL

```rust
use rquery_postgres::PostgresRenderer;

let (sql, params) = PostgresRenderer::new().render_query_stmt(&stmt).unwrap();
// sql:    SELECT "users"."name" FROM "users" WHERE "users"."age" >= $1
// params: [Int(18)]
```

### Rendering -- SQLite

```rust
use rquery_sqlite::SqliteRenderer;

let (sql, params) = SqliteRenderer::new().render_query_stmt(&stmt).unwrap();
// sql:    SELECT "users"."name" FROM "users" WHERE "users"."age" >= ?
// params: [Int(18)]
```

Same AST, different output. Parameters are always separated from the SQL string.

### INSERT with values

```rust
use rquery::ast::dml::{InsertStmt, MutationStmt};
use rquery::ast::expr::Expr;

let insert = InsertStmt::values(
    "users",
    vec!["name", "email"],
    vec![vec![Expr::value("Alice"), Expr::value("alice@test.com")]],
);
let stmt = MutationStmt::Insert(insert);

let (sql, params) = PostgresRenderer::new().render_mutation_stmt(&stmt).unwrap();
// sql:    INSERT INTO "users" ("name", "email") VALUES ($1, $2)
// params: [Str("Alice"), Str("alice@test.com")]

let (sql, params) = SqliteRenderer::new().render_mutation_stmt(&stmt).unwrap();
// sql:    INSERT INTO "users" ("name", "email") VALUES (?, ?)
// params: [Str("Alice"), Str("alice@test.com")]
```

## Crate Structure

| Crate              | Description                                      |
|--------------------|--------------------------------------------------|
| `rquery`           | Umbrella crate -- re-exports core + dialect crates |
| `rquery-core`      | AST types, `Renderer` trait, render context       |
| `rquery-postgres`  | `PostgresRenderer` -- PG-specific SQL generation  |
| `rquery-sqlite`    | `SqliteRenderer` -- SQLite-specific SQL generation |

## Documentation

- [Getting Started](docs/getting-started.md)
- [SELECT Queries](docs/select-queries.md)
- [INSERT / UPDATE / DELETE](docs/insert-update-delete.md)
- [Schema Management (DDL)](docs/schema-management.md)
- [Transactions](docs/transactions.md)
- [Parameterized Queries](docs/parameterized-queries.md)
- [Multi-Dialect Rendering](docs/multi-dialect.md)
- [Extensibility](docs/extensibility.md)
- [Type Reference](docs/type-reference.md)
- [Architecture](docs/architecture.md)

## License

MIT OR Apache-2.0
