# Parameterized Queries

## What is parameterization?

Parameterized queries separate SQL structure from data values. Instead of interpolating values directly into the SQL string, the renderer emits placeholders (`$1`, `?`, or `%s`) and collects the actual values in a separate `Vec<Value>`. The database driver then binds these values at execution time.

This gives you two things:

1. **SQL injection prevention** -- user-supplied data never becomes part of the SQL syntax.
2. **Prepared statement caching** -- the database can parse and plan the query once, then re-execute it with different parameter values.

## Which statements are parameterized?

| Statement category | Parameterized | Reason |
|---|---|---|
| DQL (SELECT) | Yes | Values in WHERE, HAVING, expressions |
| DML (INSERT, UPDATE, DELETE) | Yes | Values in SET, VALUES, WHERE |
| DDL (CREATE TABLE, ALTER, etc.) | No | Identifiers and type names cannot be parameterized |
| TCL (BEGIN, COMMIT, etc.) | No | No user data; all literals are structural |

The renderer controls this via the `parameterize` flag on `RenderCtx`. When `parameterize` is `true`, `Value` nodes are emitted as placeholders. When `false`, values are rendered as inline SQL literals.

## Parameter styles

rquery supports three placeholder styles via the `ParamStyle` enum:

| Style | Placeholder | Typical drivers |
|---|---|---|
| `ParamStyle::Dollar` | `$1`, `$2`, `$3` | asyncpg, rust-postgres, tokio-postgres |
| `ParamStyle::QMark` | `?` | SQLite, rusqlite, MySQL connectors |
| `ParamStyle::Percent` | `%s` | psycopg, DB-API 2.0 (Python) |

### Configuring the parameter style

PostgreSQL defaults to `Dollar`. To switch:

```rust
let renderer = PostgresRenderer::new()
    .with_param_style(ParamStyle::Percent);

let (sql, params) = renderer.render_query_stmt(&query)?;
// sql uses %s placeholders instead of $1
```

SQLite always uses `QMark` -- this is not configurable on `SqliteRenderer`.

## NULL handling

`Value::Null` is **always** rendered as the SQL keyword `NULL`, regardless of the `parameterize` flag. It is never emitted as a parameter placeholder. This matches database semantics: `WHERE col = NULL` is different from `WHERE col IS NULL`, and most drivers do not handle NULL parameters consistently.

## Binding parameters to drivers

The `render_*_stmt` methods return `(String, Vec<Value>)`. You convert `Value` variants to your driver's native types.

### rust-postgres / tokio-postgres

```rust
use qcraft_postgres::PostgresRenderer;
use qcraft_core::ast::value::Value;

let renderer = PostgresRenderer::new();
let (sql, params) = renderer.render_query_stmt(&query)?;

// Convert Vec<Value> to &[&dyn ToSql]
let pg_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> = params
    .iter()
    .map(|v| match v {
        Value::Int(i) => Box::new(*i) as Box<dyn tokio_postgres::types::ToSql + Sync>,
        Value::Str(s) => Box::new(s.clone()) as _,
        Value::Bool(b) => Box::new(*b) as _,
        Value::Float(f) => Box::new(*f) as _,
        Value::Null => unreachable!("NULL is rendered inline"),
        // ... handle other variants
        _ => todo!(),
    })
    .collect();

let refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
    pg_params.iter().map(|p| p.as_ref()).collect();
client.query(&sql, &refs).await?;
```

### rusqlite

```rust
use qcraft_sqlite::SqliteRenderer;

let renderer = SqliteRenderer::new();
let (sql, params) = renderer.render_query_stmt(&query)?;

let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> = params
    .iter()
    .map(|v| match v {
        Value::Int(i) => Box::new(*i) as Box<dyn rusqlite::types::ToSql>,
        Value::Str(s) => Box::new(s.clone()) as _,
        Value::Bool(b) => Box::new(*b) as _,
        Value::Float(f) => Box::new(*f) as _,
        _ => todo!(),
    })
    .collect();

let refs: Vec<&dyn rusqlite::types::ToSql> =
    rusqlite_params.iter().map(|p| p.as_ref()).collect();
conn.execute(&sql, rusqlite::params_from_iter(refs))?;
```

### psycopg (Python, via Percent style)

```rust
let renderer = PostgresRenderer::new()
    .with_param_style(ParamStyle::Percent);
let (sql, params) = renderer.render_query_stmt(&query)?;
// Pass sql and params to Python side:
// cursor.execute(sql, params)
```

## The `%` escape

When using `ParamStyle::Percent`, the `%` character has special meaning to drivers like psycopg. rquery automatically escapes operators that contain `%` by doubling it to `%%`:

- **Modulo operator** (`BinaryOp::Mod`): rendered as `%%` instead of `%`
- **Trigram operators** (`CompareOp::TrigramSimilar`, `TrigramWordSimilar`, `TrigramStrictWordSimilar`): the `%` in `%`, `<%`, and `<<%` is escaped as `%%`, `<%%`, and `<<%%`

This escaping only applies when `ParamStyle::Percent` is active. With `Dollar` or `QMark` styles, operators render normally.

## Type casts

rquery does not add automatic type casts to parameter placeholders. PostgreSQL infers parameter types from column context in most cases (e.g., `WHERE age > $1` infers `$1` is the same type as `age`).

When the database cannot infer the type, use `Expr::Cast` explicitly:

```rust
let expr = Expr::cast(Expr::value(42), "bigint");
// CAST($1 AS bigint)  -- with Dollar style
```

This is especially relevant for ambiguous expressions like `$1 || $2` where PostgreSQL cannot determine whether `||` means string concatenation or array concatenation without knowing the parameter types.
