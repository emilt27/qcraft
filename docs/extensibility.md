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

    // The node renders itself. It is handed the renderer, so it recurses back through
    // it for any sub-expression it holds — nested expressions, identifier quoting and
    // parameter numbering all go through the same dialect and the same RenderCtx.
    fn render(&self, renderer: &dyn Renderer, ctx: &mut RenderCtx) -> RenderResult<()> {
        // AT TIME ZONE is an operator, so its left-hand side is an operand:
        // `a + b AT TIME ZONE 'UTC'` would bind the zone to `b`. render_operand adds
        // brackets when the sub-expression's own structure needs them.
        renderer.render_operand(&self.expr, ctx)?;
        ctx.keyword("AT TIME ZONE").string_literal(&self.zone);
        Ok(())
    }

    // This node renders an infix operator, so it is not self-delimiting: as the operand
    // of `::` it must be bracketed, or `x AT TIME ZONE 'UTC'::date` casts the zone
    // literal instead. A node rendering `my_func(x)` would return false here.
    fn needs_operand_parens(&self) -> bool {
        true
    }
}
```

**Step 2: Use it in a query — no renderer wrapping needed.**

```rust
let expr = Expr::Custom(Box::new(AtTimeZone {
    expr: Expr::field("events", "created_at"),
    zone: "UTC".to_string(),
}));

let stmt = QueryStmt {
    columns: vec![SelectColumn::Expr {
        expr: Expr::cast(expr, "date"),   // brackets are added for you
        alias: Some("day".into()),
    }],
    from: Some(vec![FromItem::table(SchemaRef::new("events"))]),
    ..Default::default()
};

let (sql, params) = PostgresRenderer::new().render_query_stmt(&stmt).unwrap();
// SELECT ("events"."created_at" AT TIME ZONE 'UTC')::date AS "day" FROM "events"
```

The stock renderer handles it — the knowledge lives in the node, not in a renderer
wrapped around the dialect. The same applies to `CustomCondition`, `CustomBinaryOp` and
the other `Custom*` traits: each has a `render` method with the same contract. qcraft's
own `PgVectorOp` (`<->`, `<#>`, …) is implemented exactly this way, as a `CustomBinaryOp`.

A `Custom*` node that does not implement `render` is a `RenderError`, never silently
dropped SQL.

### Wrapping a renderer (`delegate_renderer!`)

`delegate_renderer!(self.inner)` forwards **every** `Renderer` method to an inner
renderer. Because it emits all of them, it **cannot be combined with overriding one** —
that is a duplicate definition (`error[E0201]`). Use it to wrap a dialect wholesale; to
add a node the renderer does not know, implement `CustomExpr::render` as above instead.

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
let expr = Expr::raw("my_custom_func()");
```

> **Note:** For standard SQL datetime keywords, prefer the built-in variants
> (`Expr::CurrentTimestamp`, `Expr::CurrentDate`, `Expr::CurrentTime`) over `Expr::raw`.

Raw SQL is injected verbatim into the output. Use this sparingly -- it bypasses dialect validation entirely.

When `params` is non-empty, use Django-style `%s` placeholders in the SQL string. The renderer replaces each `%s` with the correct dialect placeholder (`$1`, `?`, or `%s`) and appends the value to the parameter list. Use `%%` to emit a literal `%`, and `%%s` for a literal `%s`:

```rust
let expr = Expr::Raw {
    sql: "my_func(%s, %s)".to_string(),
    params: vec![Value::Int(1), Value::Str("hello".to_string())],
};
// PostgreSQL: my_func($1, $2)
// SQLite:     my_func(?, ?)
```

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
    .write("(")
    .ident("col")
    .keyword("AS")
    .keyword("TEXT")
    .paren_close();
// CAST("col" AS TEXT)
```

### Inspecting state

- `ctx.sql()` -- current SQL string (for debugging)
- `ctx.params()` -- current parameter list
- `ctx.parameterize()` -- whether values should be emitted as placeholders
- `ctx.finish()` -- consume the context and return `(String, Vec<Value>)`
