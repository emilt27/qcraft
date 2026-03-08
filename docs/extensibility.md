# Extensibility

qcraft provides three levels of extensibility, ordered from most common to least common.

## Level 1: Compose existing AST nodes (90% of cases)

The built-in AST is rich enough for most SQL. Combine `Expr`, `Conditions`, `QueryStmt`, and other nodes to express complex queries without writing any custom code. CTE-based patterns, window functions, subqueries, CASE expressions, and raw SQL fragments cover the vast majority of use cases.

## Level 2: Custom AST node + renderer override (9% of cases)

When the built-in AST lacks a specific construct (a vendor-specific function syntax, a custom operator, a proprietary clause), you can:

1. Define a custom struct implementing one of the `Custom*` traits.
2. Wrap an existing dialect renderer and override the relevant method.

### Custom traits

Each extensible AST enum has a corresponding trait:

| AST location | Custom trait | Used in |
|---|---|---|
| `Expr::Custom` | `CustomExpr` | Expressions |
| `ConditionNode::Custom` | `CustomCondition` | WHERE/HAVING conditions |
| `CompareOp::Custom` | `CustomCompareOp` | Comparison operators |
| `TableSource::Custom` | `CustomTableSource` | FROM clause sources |
| `SchemaMutationStmt::Custom` | `CustomSchemaMutation` | DDL statements |
| `TransactionStmt::Custom` | `CustomTransaction` | TCL statements |
| `FieldType::Custom` | `CustomFieldType` | Column types |
| `ConstraintDef::Custom` | `CustomConstraint` | Table constraints |
| `MutationStmt::Custom` | `CustomMutation` | DML statements |

All `Custom*` traits share the same shape:

```rust
pub trait CustomExpr: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomExpr>;
}
```

- `as_any()` enables downcasting to the concrete type in your renderer.
- `clone_box()` enables cloning the boxed trait object (the AST is `Clone`).

### Complete example

Suppose you need a PostgreSQL `AT TIME ZONE` expression that is not in the built-in AST.

**Step 1: Define the custom expression.**

```rust
use std::any::Any;
use qcraft_core::ast::custom::CustomExpr;
use qcraft_core::ast::expr::Expr;

#[derive(Debug, Clone)]
pub struct AtTimeZone {
    pub expr: Expr,
    pub zone: String,
}

impl CustomExpr for AtTimeZone {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn CustomExpr> {
        Box::new(self.clone())
    }
}
```

**Step 2: Wrap the renderer and override `render_expr`.**

```rust
use qcraft_core::render::renderer::Renderer;
use qcraft_core::render::ctx::RenderCtx;
use qcraft_core::ast::expr::Expr;
use qcraft_core::error::RenderResult;
use qcraft_core::delegate_renderer;
use qcraft_postgres::PostgresRenderer;

pub struct MyRenderer {
    inner: PostgresRenderer,
}

impl MyRenderer {
    pub fn new() -> Self {
        Self {
            inner: PostgresRenderer::new(),
        }
    }
}

impl Renderer for MyRenderer {
    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> RenderResult<()> {
        if let Expr::Custom(custom) = expr {
            if let Some(atz) = custom.as_any().downcast_ref::<AtTimeZone>() {
                self.inner.render_expr(&atz.expr, ctx)?;
                ctx.keyword("AT TIME ZONE");
                ctx.string_literal(&atz.zone);
                return Ok(());
            }
        }
        // Fall through to the default renderer for all other expressions
        self.inner.render_expr(expr, ctx)
    }

    // Delegate every other Renderer method to self.inner
    delegate_renderer!(self.inner);
}
```

**Step 3: Use it in a query.**

```rust
let expr = Expr::Custom(Box::new(AtTimeZone {
    expr: Expr::field("events", "created_at"),
    zone: "UTC".to_string(),
}));

// Use in a SelectColumn, WHERE clause, etc.
let col = SelectColumn::aliased(expr, "created_utc");
```

### The `as_any()` + `downcast_ref` pattern

Because `Custom*` traits are trait objects, you need runtime downcasting to access fields on your concrete type. The pattern is:

```rust
if let Some(my_node) = custom.as_any().downcast_ref::<MyConcreteType>() {
    // Access my_node.field1, my_node.field2, etc.
}
```

If the downcast fails, it means another custom type was passed. You can either return an error or delegate to the inner renderer.

### The `delegate_renderer!` macro

The `delegate_renderer!(self.inner)` macro generates forwarding implementations for all `Renderer` trait methods. You only write the methods you want to override; everything else delegates to `self.inner`.

## Level 3: Raw SQL escape hatch (1% of cases)

When you need a one-off SQL fragment that does not justify a custom AST node:

```rust
let expr = Expr::Raw {
    sql: "pg_advisory_lock(hashtext($1))".to_string(),
    params: vec![Value::Str("my_lock".to_string())],
};
```

Or using the convenience constructor (no parameters):

```rust
let expr = Expr::raw("CURRENT_TIMESTAMP");
```

Raw SQL is injected verbatim into the output. Parameters in `Expr::Raw` are appended to the parameter list and their placeholders must already be correct for your `ParamStyle`. Use this sparingly -- it bypasses dialect validation entirely.

## RenderCtx semantic API

When implementing custom renderers, you write SQL through `RenderCtx` rather than building strings manually. The context handles auto-spacing, quoting, and parameter collection.

| Method | Output | Notes |
|---|---|---|
| `ctx.keyword("SELECT")` | `SELECT` | Auto-spaces before the keyword if needed |
| `ctx.ident("users")` | `"users"` | Double-quoted identifier, escapes internal `"` |
| `ctx.param(Value::Int(42))` | `$1` / `?` / `%s` | Appends to the parameter list |
| `ctx.string_literal("hello")` | `'hello'` | Escapes internal single quotes |
| `ctx.paren_open()` | `(` | Auto-spaces before, suppresses space after |
| `ctx.paren_close()` | `)` | No leading space |
| `ctx.comma()` | `, ` | Comma plus space |
| `ctx.operator(" = ")` | ` = ` | Verbatim, no auto-spacing |
| `ctx.space()` | ` ` | Explicit space |
| `ctx.write("...")` | verbatim | Escape hatch for arbitrary text |

All methods return `&mut Self` for chaining:

```rust
ctx.keyword("CAST")
    .paren_open()
    .ident("col")
    .keyword("AS")
    .keyword("TEXT")
    .paren_close();
// CAST ("col" AS TEXT)
```

### Inspecting state

- `ctx.sql()` -- current SQL string (for debugging)
- `ctx.params()` -- current parameter list
- `ctx.parameterize()` -- whether values should be emitted as placeholders
- `ctx.finish()` -- consume the context and return `(String, Vec<Value>)`
