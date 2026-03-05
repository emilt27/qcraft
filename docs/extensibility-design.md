# Extensibility Design

## Core Principle: AST = WHAT, Renderer = HOW

AST describes **semantics** (what we want to do) without any hint about order, keywords, or syntax.
Renderer for each dialect decides **how** it looks in SQL.

```
AST = WHAT (semantics, data, no order)
     ↓
Renderer = HOW (syntax, order, keywords — different per dialect)
     ↓
SQL string
```

## How It Works: Same AST, Different SQL

AST node describes intent, not syntax:

```rust
pub struct AggregationDef {
    pub name: String,                   // "COUNT"
    pub expression: Option<Box<Expr>>,  // id
    pub distinct: bool,
    pub filter: Option<Conditions>,     // age > 10
    pub order_by: Option<Vec<OrderByDef>>,
}
```

Each dialect renders it differently:

```rust
// PostgresRenderer
fn render_aggregate(&self, agg: &AggregationDef, ctx: &mut RenderCtx) -> Result<()> {
    // → COUNT(id) FILTER (WHERE age > 10)
    ctx.keyword(&agg.name).paren_open();
    if let Some(expr) = &agg.expression {
        self.render_expr(expr, ctx)?;
    }
    ctx.paren_close();
    if let Some(filter) = &agg.filter {
        ctx.keyword("FILTER").paren_open().keyword("WHERE");
        self.render_condition(filter, ctx)?;
        ctx.paren_close();
    }
    Ok(())
}

// SqliteRenderer — filter inside parens
fn render_aggregate(&self, agg: &AggregationDef, ctx: &mut RenderCtx) -> Result<()> {
    // → COUNT(id, WHERE age > 10)
    ctx.keyword(&agg.name).paren_open();
    if let Some(expr) = &agg.expression {
        self.render_expr(expr, ctx)?;
    }
    if let Some(filter) = &agg.filter {
        ctx.comma().keyword("WHERE");
        self.render_condition(filter, ctx)?;
    }
    ctx.paren_close();
    Ok(())
}
```

Same AST, different SQL. Order, keywords, structure — all decided by the renderer, not by AST.

## Three Levels of Extensibility

### Level 1: Composition of Existing AST Nodes (90% of cases)

The AST is rich enough to express the feature natively. No custom code needed.

Example: `COUNT(DISTINCT id) FILTER (WHERE age > 10)` — already works if `AggregationDef` has `distinct: bool` and `filter: Option<Conditions>`:

```rust
Expr::Aggregate(AggregationDef {
    name: "COUNT".into(),
    expression: Some(Box::new(Expr::Field(field_ref("id")))),
    distinct: true,
    filter: Some(conditions![Field("age").gt(Value::Int(10))]),
    ..Default::default()
})
```

Example: `COUNT(id::text) FILTER (WHERE age > 10)` — works if expression accepts `Expr::Cast`:

```rust
Expr::Aggregate(AggregationDef {
    name: "COUNT".into(),
    expression: Some(Box::new(Expr::Cast {
        expr: Box::new(Expr::Field(field_ref("id"))),
        to_type: "text".into(),
    })),
    filter: Some(conditions![Field("age").gt(Value::Int(10))]),
    ..Default::default()
})
```

No custom nodes, no renderer overrides. The AST is expressive enough.

### Level 2: Custom AST Node + Renderer Override (9% of cases)

When a database adds a feature that the AST cannot express, the user extends both the AST and the renderer **from outside the library**.

Example: PostgreSQL adds `COUNT(id) FILTER (WHERE age > 10 GROUP BY age)` — GROUP BY inside FILTER doesn't exist in our AST.

**Step 1: Define a new AST node (semantics, not syntax)**

```rust
#[derive(Debug, Clone)]
struct AggregateWithFilterGroupBy {
    pub name: String,
    pub expression: Expr,
    pub filter: Conditions,
    pub filter_group_by: Vec<Expr>,  // ← new field not in standard AST
}

impl CustomExpr for AggregateWithFilterGroupBy {
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn CustomExpr> { Box::new(self.clone()) }
}
```

**Step 2: Wrap the renderer and handle the new node**

```rust
struct MyRenderer {
    inner: PostgresRenderer,
}

impl Renderer for MyRenderer {
    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> Result<()> {
        if let Expr::Custom(c) = expr {
            if let Some(agg) = c.as_any().downcast_ref::<AggregateWithFilterGroupBy>() {
                ctx.keyword(&agg.name).paren_open();
                self.render_expr(&agg.expression, ctx)?;
                ctx.paren_close().keyword("FILTER").paren_open().keyword("WHERE");
                self.render_condition(&agg.filter, ctx)?;
                ctx.keyword("GROUP BY");
                for (i, g) in agg.filter_group_by.iter().enumerate() {
                    if i > 0 { ctx.comma(); }
                    self.render_expr(g, ctx)?;
                }
                ctx.paren_close();
                return Ok(());
            }
        }
        self.inner.render_expr(expr, ctx)
    }

    // All other methods delegate to inner
    delegate_renderer!(self.inner);
}
```

**Step 3: Use it**

```rust
let query = Query::select()
    .expr(Expr::custom(AggregateWithFilterGroupBy {
        name: "COUNT".into(),
        expression: Expr::field("id"),
        filter: conditions![Field("age").gt(Value::Int(10))],
        filter_group_by: vec![Expr::field("age")],
    }))
    .from("users")
    .build();

let renderer = MyRenderer { inner: PostgresRenderer::new() };
let (sql, params) = renderer.render(&query)?;
// → SELECT COUNT("id") FILTER (WHERE "age" > $1 GROUP BY "age") FROM "users"
// → params = [Value::Int(10)]
```

### Level 3: Raw SQL Escape Hatch (1% of cases)

For truly one-off cases where even a custom node is overkill:

```rust
Expr::Raw {
    sql: "COUNT(id) FILTER (WHERE age > $1 GROUP BY age)".into(),
    params: vec![Value::Int(10)],
}
```

## RenderCtx Semantic API

Instead of raw `ctx.write("CAST(")`, `RenderCtx` provides semantic methods that are chainable and self-documenting:

```rust
impl RenderCtx {
    pub fn keyword(&mut self, kw: &str) -> &mut Self;       // SQL keyword, auto-spaced
    pub fn ident(&mut self, name: &str) -> &mut Self;        // Quoted identifier
    pub fn param(&mut self, val: Value) -> &mut Self;        // Parameterized value → $1 or ?
    pub fn string_literal(&mut self, s: &str) -> &mut Self;  // Escaped string literal
    pub fn operator(&mut self, op: &str) -> &mut Self;       // Operator (::, =, >, ||)
    pub fn paren_open(&mut self) -> &mut Self;               // (
    pub fn paren_close(&mut self) -> &mut Self;              // )
    pub fn comma(&mut self) -> &mut Self;                    // ,
    pub fn space(&mut self) -> &mut Self;                    // explicit space
    pub fn write(&mut self, s: &str) -> &mut Self;           // escape hatch
}
```

Usage:

```rust
// Instead of: ctx.write("CAST("); ... ctx.write(" AS "); ... ctx.write(")");
// Write:
ctx.keyword("CAST").paren_open();
self.render_expr(expr, ctx)?;
ctx.keyword("AS").write(to_type).paren_close();
```

## Why This Is Not String Hacking

In Level 2, the renderer override uses `ctx.keyword("FILTER")`, `ctx.keyword("GROUP BY")` etc. This is **not** the same problem as sea-query string hacking:

| | sea-query + string hacking | rquery custom renderer |
|---|---|---|
| Where | After rendering, on finished SQL | In renderer, with access to AST |
| Parameters | Break (string manipulation) | Work (`self.render_expr` correctly adds `$1`) |
| Type safety | No (regex on SQL) | Yes (`downcast_ref` to concrete type) |
| Nested expressions | Not re-rendered | `self.render_expr()` recursively renders |
| Composability | None | Full (can call any renderer method) |

Keywords via `ctx.keyword()` are static strings (`"FILTER"`, `"GROUP BY"`). The problem with sea-query is manipulating **already-rendered expressions** — substituting values, reordering parts of finished SQL. In rquery, expressions and conditions are rendered through the standard pipeline with proper parameter handling.

## Multi-Dialect Custom Nodes

A custom AST node can have different renderers for different dialects:

```rust
// Same custom AST node
struct MySpecialAgg { /* ... */ }

// PostgreSQL rendering
struct MyPgRenderer { inner: PostgresRenderer }
impl Renderer for MyPgRenderer {
    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> Result<()> {
        if let Some(agg) = try_downcast::<MySpecialAgg>(expr) {
            // PostgreSQL syntax: COUNT(id) FILTER (WHERE age > 10)
            // ...
        }
        self.inner.render_expr(expr, ctx)
    }
    delegate_renderer!(self.inner);
}

// SQLite rendering
struct MySqliteRenderer { inner: SqliteRenderer }
impl Renderer for MySqliteRenderer {
    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> Result<()> {
        if let Some(agg) = try_downcast::<MySpecialAgg>(expr) {
            // SQLite syntax: completely different order/keywords
            // ...
        }
        self.inner.render_expr(expr, ctx)
    }
    delegate_renderer!(self.inner);
}
```

Same AST node, different renderers — different SQL output per dialect.

## Rejected Alternative: expand() -> Vec<AstToken>

An earlier design had custom nodes return a sequence of tokens:

```rust
fn expand(&self) -> Vec<AstToken> {
    vec![
        AstToken::Keyword("MERGE INTO"),
        AstToken::TableRef(self.target.clone()),
        AstToken::Keyword("USING"),
        AstToken::SubQuery(self.source.clone()),
        // ...
    ]
}
```

This was rejected because:
- **Fixes the order** — different dialects cannot reorder parts
- **Fixes the keywords** — different dialects cannot use different keywords
- **One expand() per node** — cannot have different expansions per dialect
- Essentially the same as writing a template string, just with typed pieces

The renderer override approach (Level 2) gives full control per dialect.

## Summary

| Level | When | What user does | String manipulation? |
|---|---|---|---|
| 1. Compose existing AST | Feature fits existing AST fields | Nothing — just use the API | No |
| 2. Custom node + renderer | New syntax not in AST | Define struct + override render method | Only SQL keywords |
| 3. Raw SQL | One-off, throwaway | `Expr::Raw { sql, params }` | Yes (escape hatch) |

The goal: make Level 1 cover as many cases as possible by having a rich, granular AST. Level 2 is the structured escape hatch. Level 3 exists but should rarely be needed.
