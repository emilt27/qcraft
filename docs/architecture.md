# Architecture

This document explains the design decisions behind rquery and how the pieces fit together.

## AST-first design

rquery is built around a typed Abstract Syntax Tree (AST), not a builder pattern. Every SQL construct -- a SELECT query, an INSERT statement, a CREATE TABLE -- is represented as a Rust data structure. You construct the AST directly, then hand it to a renderer to produce SQL.

Why not a builder? Builders mix construction and rendering, making it hard to inspect, transform, or re-render a query. With an AST, you can:

- Build a query once, render it to multiple dialects.
- Inspect or transform the query programmatically before rendering.
- Serialize the AST for logging, caching, or transport.
- Test the AST structure independently of the SQL output.

The AST is the **single source of truth** for what the query means. The renderer only decides **how** to express it in a specific SQL dialect.

## Three layers

```
+--------------------------------------------+
|  rquery  (umbrella crate, re-exports)      |
+--------------------------------------------+
         |                    |
+------------------+  +------------------+
| rquery-postgres  |  | rquery-sqlite    |
| (PostgresRenderer)|  | (SqliteRenderer) |
+------------------+  +------------------+
         |                    |
+--------------------------------------------+
|  rquery-core                               |
|  - AST types (query, dml, ddl, tcl, expr)  |
|  - Renderer trait                          |
|  - RenderCtx                               |
|  - Value, Conditions, Custom traits        |
+--------------------------------------------+
```

### rquery-core

The foundation. Contains:

- **AST modules**: `query` (DQL), `dml`, `ddl`, `tcl`, `expr`, `conditions`, `value`, `common`, `custom`
- **Renderer trait**: Defines the interface that all dialect renderers implement
- **RenderCtx**: The context object that accumulates SQL output and parameters
- **Error types**: `RenderError` and `RenderResult`
- **Policy types**: `UnsupportedPolicy`, `Feature`, `Warning`

rquery-core has no dialect-specific logic. It defines _what_ can be expressed, not _how_ it renders.

### rquery-postgres

Implements `Renderer` for PostgreSQL via `PostgresRenderer`. Handles PG-specific syntax: `$1` parameters, `DISTINCT ON`, `FOR UPDATE`, `TABLESAMPLE`, JSONB/trigram/range/FTS operators, two-phase commit, `LOCK TABLE`, extensions, partitioning, and more.

Configurable parameter style via `PostgresRenderer::new().with_param_style(ParamStyle::Percent)`.

### rquery-sqlite

Implements `Renderer` for SQLite via `SqliteRenderer`. Handles SQLite-specific syntax: `?` parameters, `INDEXED BY`, `WITHOUT ROWID`, `STRICT`, conflict resolution (`OR REPLACE`, `OR IGNORE`), and SQLite transaction lock types (`DEFERRED`, `IMMEDIATE`, `EXCLUSIVE`).

### rquery (umbrella)

Re-exports from all three crates for convenience.

## AST = WHAT, Renderer = HOW

The same `QueryStmt` renders differently depending on the renderer:

- A `LimitDef` with `LimitKind::FetchFirst` renders as `FETCH FIRST n ROWS ONLY` on PostgreSQL but converts to `LIMIT n` on SQLite.
- A `DistinctDef::DistinctOn` renders as `DISTINCT ON (...)` on PostgreSQL but returns `RenderError::Unsupported` on SQLite.
- A `BeginStmt` with `lock_type: Some(SqliteLockType::Immediate)` renders as `BEGIN IMMEDIATE` on SQLite but the lock type is ignored on PostgreSQL.

This separation means you can build your AST once in shared code, then render it per-database at the edges of your application.

## RenderCtx

`RenderCtx` is the semantic writing API that renderers use to produce SQL. Instead of raw string concatenation, renderers call methods like `keyword`, `ident`, `param`, `string_literal`, `paren_open`, `paren_close`, `comma`, `operator`, and `write`.

Key behaviors:

- **Auto-spacing**: `keyword` and `ident` insert a space before themselves when needed (unless the buffer ends with `(`, `.`, or whitespace). This eliminates manual space management.
- **Parameter collection**: `ctx.param(value)` emits the correct placeholder for the configured `ParamStyle` and appends the value to the internal parameter list.
- **Quoting**: `ctx.ident("name")` wraps identifiers in double quotes and escapes embedded quotes. `ctx.string_literal("text")` wraps in single quotes with proper escaping.
- **Finish**: `ctx.finish()` consumes the context and returns `(String, Vec<Value>)`.

Usage example:

```rust
// PostgreSQL CAST: expr::type
ctx.operator("::");
ctx.write(to_type);

// SQLite CAST: CAST(expr AS type)
ctx.keyword("CAST").paren_open();
self.render_expr(expr, ctx)?;
ctx.keyword("AS").write(to_type).paren_close();
```

## Parameterization architecture

The `RenderCtx` carries a `parameterize: bool` flag that controls whether `Value` nodes are emitted as placeholders or inline literals.

| Statement type | `parameterize` | Reason |
|---|---|---|
| DQL (SELECT) | `true` | User data in WHERE, expressions |
| DML (INSERT, UPDATE, DELETE) | `true` | User data in VALUES, SET, WHERE |
| DDL (CREATE TABLE, ALTER, etc.) | `false` | Only identifiers and type names |
| TCL (BEGIN, COMMIT, etc.) | `false` | Structural commands, no user data |

Each top-level render method (`render_query_stmt`, `render_mutation_stmt`, `render_schema_stmt`, `render_transaction_stmt`) creates its own `RenderCtx` with the appropriate flag. This is not something callers need to manage.

`Value::Null` is always rendered as the `NULL` keyword, never as a parameter, regardless of the flag.

## Unsupported features policy

When a feature in the AST has no equivalent in the target dialect, the renderer follows one of three strategies:

### Ignore (safe to skip)

The feature is decoration or an optimization hint. Skipping it does not change query results. Examples: CTE `MATERIALIZED` hints on SQLite, `ONLY` keyword on SQLite.

### Error (changes semantics)

Skipping the feature would produce a query with different behavior. The renderer returns `RenderError::Unsupported` with a description. Examples: `DISTINCT ON` on SQLite, `FOR UPDATE` on SQLite, `LATERAL` on SQLite.

### Workaround (transforms syntax)

The feature has no direct equivalent, but can be expressed differently. The renderer silently transforms it. Examples: `TOP(n)` to `LIMIT n`, `FETCH FIRST n ROWS` to `LIMIT n` on SQLite.

The policy is hardcoded per feature per renderer. The renderer authors made these decisions based on semantic correctness -- whether silently dropping a feature would change the query's meaning.

## Extensibility model

Every major AST enum has a `Custom(Box<dyn Custom*>)` variant. This covers expressions, conditions, comparison operators, table sources, DML, DDL, TCL, field types, and constraints. All custom traits follow the same shape:

```rust
pub trait CustomExpr: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomExpr>;
}
```

There are three levels of extensibility:

1. **Compose existing AST** (90% of cases) -- the AST is rich enough for most SQL.
2. **Custom AST node + renderer override** (9%) -- define a custom struct implementing the appropriate `Custom*` trait, wrap an existing renderer, override the relevant `render_*` method, and use `as_any().downcast_ref()` to access your concrete type.
3. **Raw SQL escape hatch** (1%) -- `Expr::Raw { sql, params }` for one-off fragments.

To handle custom variants, you wrap an existing renderer via composition and override the relevant method. The `delegate_renderer!` macro forwards all other `Renderer` methods to the inner renderer:

```rust
struct MyRenderer {
    inner: PostgresRenderer,
}

impl Renderer for MyRenderer {
    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> RenderResult<()> {
        if let Expr::Custom(custom) = expr {
            if let Some(my_node) = custom.as_any().downcast_ref::<MyCustomNode>() {
                // Render your custom syntax
                return Ok(());
            }
        }
        self.inner.render_expr(expr, ctx)
    }

    delegate_renderer!(self.inner);
}
```

This three-level model covers the full spectrum from simple queries to exotic vendor-specific syntax, while keeping the core library focused and maintainable.
