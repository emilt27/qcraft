# qcraft

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
cargo add qcraft
```

Both `postgres` and `sqlite` features are enabled by default. To use only one:

```sh
cargo add qcraft --no-default-features --features postgres
cargo add qcraft --no-default-features --features sqlite
```

## Quick Start

### SELECT with WHERE

```rust
use qcraft::ast::common::{FieldRef, SchemaRef};
use qcraft::ast::conditions::Conditions;
use qcraft::ast::query::{FromItem, QueryStmt, SelectColumn};

let stmt = QueryStmt {
    columns: vec![SelectColumn::field("users", "name")],
    from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
    where_clause: Some(Conditions::gte(FieldRef::new("users", "age"), 18)),
    ..Default::default()
};
```

### Rendering -- PostgreSQL

```rust
use qcraft_postgres::PostgresRenderer;

let (sql, params) = PostgresRenderer::new().render_query_stmt(&stmt).unwrap();
// sql:    SELECT "users"."name" FROM "users" WHERE "users"."age" >= $1
// params: [Int(18)]
```

### Rendering -- SQLite

```rust
use qcraft_sqlite::SqliteRenderer;

let (sql, params) = SqliteRenderer::new().render_query_stmt(&stmt).unwrap();
// sql:    SELECT "users"."name" FROM "users" WHERE "users"."age" >= ?
// params: [Int(18)]
```

Same AST, different output. Parameters are always separated from the SQL string.

### INSERT with values

```rust
use qcraft::ast::dml::{InsertStmt, MutationStmt};
use qcraft::ast::expr::Expr;

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

## Performance

qcraft is **3.5x–14.6x faster** than [sea-query](https://crates.io/crates/sea-query) and uses **up to 8.9x less memory** per query. Benchmarks measure parameterized SQL rendering to PostgreSQL:

| Scenario | qcraft | sea-query | Speedup |
|---|---|---|---|
| Simple SELECT + WHERE | 201 ns | 1,345 ns | **6.7x** |
| JOIN + GROUP BY + ORDER BY + LIMIT | 362 ns | 3,168 ns | **8.8x** |
| INSERT (3 rows) | 479 ns | 1,662 ns | **3.5x** |
| Complex CTE + JOIN + GROUP BY + HAVING | 489 ns | 7,152 ns | **14.6x** |

See [full benchmark results](docs/benchmarks.md) for memory allocation details and methodology.

## Crate Structure

| Crate              | Description                                      |
|--------------------|--------------------------------------------------|
| `qcraft`           | Umbrella crate -- re-exports core + dialect crates |
| `qcraft-core`      | AST types, `Renderer` trait, render context       |
| `qcraft-postgres`  | `PostgresRenderer` -- PG-specific SQL generation  |
| `qcraft-sqlite`    | `SqliteRenderer` -- SQLite-specific SQL generation |

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
- [Benchmarks](docs/benchmarks.md)

## License

MIT OR Apache-2.0
