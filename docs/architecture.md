# rquery Architecture

## Overview

rquery is a universal, extensible query builder library for Rust. It generates SQL (and potentially other query languages) from a typed intermediate representation (IR/AST), with first-class support for external extensibility — users can add new features without modifying the library.

### Supported dialects

- PostgreSQL
- SQLite

## Three Layers

```
┌──────────────────────────────────────────────────────────┐
│  Layer 1: IR (AST)                                        │
│  - Enum-based types                                       │
│  - Every enum has Custom(Box<dyn Custom___>) variant      │
│  - No SQL strings, pure data structures                   │
│  rquery-core crate                                        │
└────────────────────────────┬─────────────────────────────┘
                             │
┌────────────────────────────▼─────────────────────────────┐
│  Layer 2: Renderer trait                                  │
│  - render_select(), render_expr(), render_condition()...  │
│  - Each method has default impl calling sub-methods       │
│  - User can override ANY granular piece                   │
│  rquery-core crate                                        │
└────────────────────────────┬─────────────────────────────┘
                             │
┌──────────────┬─────────────▼──────────────┐
│  PostgreSQL  │    SQLite Renderer          │
│  Renderer    │    (rquery-sqlite)          │
│ (rquery-pg)  │                             │
└──────────────┴────────────────────────────┘
```

## Extensibility Model

The key design goal: users of the library can add support for new database features **without waiting for a library release** and **without string manipulation hacks**.

This is achieved through two mechanisms:

### 1. Custom AST Variants

Every IR enum has a `Custom(Box<dyn Custom___>)` variant that allows injecting arbitrary AST nodes:

```rust
// User defines a custom AST node
#[derive(Debug, Clone)]
struct CountIf {
    field: Expr,
    condition: Expr,
}

impl CustomExpr for CountIf {
    fn as_any(&self) -> &dyn Any { self }
}

// User uses it in a query
let query = Query::select()
    .expr(Expr::custom(CountIf {
        field: Expr::field("id"),
        condition: Expr::field("age").gt(Expr::val(10)),
    }))
    .from("users")
    .build();
```

### 2. Renderer Wrapping (Composition)

Users create their own renderer by wrapping a standard one and overriding only the methods they need:

```rust
struct MyPostgresRenderer {
    inner: PostgresRenderer,
}

impl Renderer for MyPostgresRenderer {
    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> Result<()> {
        // Intercept only our custom node
        if let Expr::Custom(custom) = expr {
            if let Some(count_if) = custom.as_any().downcast_ref::<CountIf>() {
                ctx.keyword("COUNT").paren_open();
                self.render_expr(&count_if.field, ctx)?;
                ctx.paren_close().keyword("FILTER").paren_open();
                ctx.keyword("WHERE");
                self.render_expr(&count_if.condition, ctx)?;
                ctx.paren_close();
                return Ok(());
            }
        }
        // Everything else — delegate to standard renderer
        self.inner.render_expr(expr, ctx)
    }

    // All other methods delegate via macro
    delegate_renderer!(self.inner);
}
```

The `delegate_renderer!` macro generates delegation for all `Renderer` trait methods to the inner renderer, so the user only overrides what they need.

## IR Types

### Query

```rust
pub struct QueryStmt {
    pub table: TableSource,
    pub columns: Option<Vec<SelectColumn>>,
    pub distinct: Option<DistinctClause>,
    pub joins: Option<Vec<JoinDef>>,
    pub where_clause: Option<Conditions>,
    pub group_by: Option<Vec<Expr>>,
    pub having: Option<Conditions>,
    pub order_by: Option<Vec<OrderByDef>>,
    pub limit: Option<LimitDef>,
    pub ctes: Option<Vec<CteDef>>,
    pub lock: Option<SelectLockDef>,
}

pub enum TableSource {
    Table(SchemaRef),
    SubQuery(SubQueryDef),
    SetOp(Box<SetOpDef>),
    Custom(Box<dyn CustomTableSource>),
}

pub struct SchemaRef {
    pub name: String,
    pub alias: Option<String>,
    pub namespace: Option<String>,
}
```

### Expressions

```rust
pub enum Expr {
    Value(Value),
    Field(FieldRef),
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Func { name: String, args: Vec<Expr> },
    Aggregate(AggregationDef),
    Cast { expr: Box<Expr>, to_type: String },
    Case(CaseDef),
    Window(WindowDef),
    Exists(Box<QueryStmt>),
    SubQuery(Box<QueryStmt>),
    Raw { sql: String, params: Vec<Value> },
    Custom(Box<dyn CustomExpr>),
}

pub struct AggregationDef {
    pub name: String,
    pub expression: Option<Box<Expr>>,
    pub distinct: bool,
    pub filter: Option<Conditions>,
    pub args: Option<Vec<Expr>>,
    pub order_by: Option<Vec<OrderByDef>>,
}

pub struct WindowDef {
    pub expression: Box<Expr>,
    pub partition_by: Option<Vec<Expr>>,
    pub order_by: Option<Vec<OrderByDef>>,
    pub frame: Option<WindowFrameDef>,
}

pub struct WindowFrameDef {
    pub frame_type: WindowFrameType,  // Rows | Range | Groups
    pub start: Option<i64>,
    pub end: Option<i64>,
}

pub struct CaseDef {
    pub cases: Vec<WhenClause>,
    pub default: Option<Box<Expr>>,
}

pub struct WhenClause {
    pub condition: Conditions,
    pub result: Expr,
}
```

### Field References

```rust
pub struct FieldDef {
    pub name: String,
    pub child: Option<Box<FieldDef>>,  // For nested/JSON path access
}

pub struct FieldRef {
    pub field: FieldDef,
    pub table_name: String,
    pub namespace: Option<String>,
}
```

### Conditions

```rust
pub struct Conditions {
    pub children: Vec<ConditionNode>,
    pub connector: Connector,  // And | Or
    pub negated: bool,
}

pub enum ConditionNode {
    Comparison(Comparison),
    Group(Conditions),
    Exists(Box<QueryStmt>),
    Custom(Box<dyn CustomCondition>),
}

pub struct Comparison {
    pub left: Expr,
    pub op: CompareOp,
    pub right: Expr,
    pub negate: bool,
}

pub enum CompareOp {
    Eq, Neq, Gt, Gte, Lt, Lte,
    In, Like, ILike, Between, IsNull,
    // PostgreSQL-specific
    JsonbContains, JsonbContainedBy, JsonbHasKey, JsonbHasAnyKey, JsonbHasAllKeys,
    FtsMatch,
    TrigramSimilar, TrigramWordSimilar, TrigramStrictWordSimilar,
    RangeContains, RangeContainedBy, RangeOverlap,
    Similar, Regex, IRegex,
    Custom(Box<dyn CustomCompareOp>),
}
```

### Values

```rust
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    Date(String),
    DateTime(String),
    Time(String),
    List(Vec<Value>),
    Json(serde_json::Value),
    Decimal(String),
    Uuid(String),
    TimeDelta { days: i64, seconds: i64, microseconds: i64 },
}
```

### DML (Data Manipulation)

```rust
pub enum MutationStmt {
    Insert(InsertStmt),
    InsertFromSelect(InsertFromSelectStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    Truncate(SchemaRef),
    Custom(Box<dyn CustomMutation>),
}

pub struct InsertStmt {
    pub schema: SchemaRef,
    pub rows: Vec<DataRow>,
    pub on_conflict: Option<OnConflictDef>,
    pub returning: Option<Vec<FieldRef>>,
}

pub struct DataRow {
    pub data: Vec<(String, Value)>,
}

pub struct OnConflictDef {
    pub fields: Vec<FieldRef>,
    pub action: ConflictAction,  // Nothing | Update
    pub update_fields: Option<Vec<FieldRef>>,
    pub where_clause: Option<Conditions>,
}

pub struct InsertFromSelectStmt {
    pub schema: SchemaRef,
    pub query: QueryStmt,
    pub columns: Option<Vec<FieldRef>>,
    pub returning: Option<Vec<FieldRef>>,
}

pub struct UpdateStmt {
    pub schema: SchemaRef,
    pub assignments: Vec<(String, Expr)>,
    pub where_clause: Option<Conditions>,
    pub from_tables: Option<Vec<TableSource>>,
    pub returning: Option<Vec<FieldRef>>,
}

pub struct DeleteStmt {
    pub schema: SchemaRef,
    pub where_clause: Option<Conditions>,
    pub returning: Option<Vec<FieldRef>>,
}
```

### DDL (Schema Mutation)

```rust
pub enum SchemaMutationStmt {
    CreateTable { schema: SchemaDef, if_not_exists: bool },
    DropTable { schema_ref: SchemaRef, if_exists: bool, cascade: bool },
    RenameTable { schema_ref: SchemaRef, new_name: String },
    AddColumn { schema_ref: SchemaRef, column: ColumnDef },
    DropColumn { schema_ref: SchemaRef, name: String },
    RenameColumn { schema_ref: SchemaRef, old_name: String, new_name: String },
    UpdateColumn { schema_ref: SchemaRef, column: ColumnDef },
    AddConstraint { schema_ref: SchemaRef, constraint: ConstraintDef, not_valid: bool },
    DropConstraint { schema_ref: SchemaRef, name: String },
    ValidateConstraint { schema_ref: SchemaRef, name: String },
    CreateIndex { schema_ref: SchemaRef, index: IndexDef, if_not_exists: bool, concurrent: bool },
    DropIndex { schema_ref: SchemaRef, name: String, if_exists: bool, concurrent: bool },
    CreateExtension { name: String, if_not_exists: bool, schema: Option<String>, version: Option<String> },
    DropExtension { name: String, if_exists: bool, cascade: bool },
    Custom(Box<dyn CustomSchemaMutation>),
}

pub struct SchemaDef {
    pub name: String,
    pub namespace: Option<String>,
    pub columns: Vec<ColumnDef>,
    pub constraints: Option<Vec<ConstraintDef>>,
    pub indexes: Option<Vec<IndexDef>>,
}

pub struct ColumnDef {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<Expr>,
    pub generated: Option<Expr>,
    pub collation: Option<String>,
}

pub enum FieldType {
    Scalar(String),              // text, integer, bigint, boolean, etc.
    Custom { name: String, params: Option<Vec<String>> },
    Array(Box<FieldType>),
    Vector(i64),                 // pgvector
}

pub enum ConstraintDef {
    PrimaryKey { name: Option<String>, fields: Vec<String> },
    ForeignKey {
        name: Option<String>,
        fields: Vec<String>,
        ref_table: SchemaRef,
        ref_fields: Vec<String>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
    },
    Unique { name: Option<String>, fields: Vec<String>, condition: Option<Conditions> },
    Check { name: Option<String>, condition: Conditions },
    Exclusion {
        name: Option<String>,
        elements: Vec<(String, String)>,
        index_method: String,
        condition: Option<Conditions>,
    },
}

pub enum ReferentialAction {
    NoAction, Restrict, Cascade, SetNull, SetDefault,
}

pub struct IndexDef {
    pub name: String,
    pub fields: Vec<IndexFieldDef>,
    pub unique: bool,
    pub index_type: Option<String>,     // btree, hash, gist, gin, brin
    pub include: Option<Vec<String>>,   // INCLUDE columns (PostgreSQL)
    pub condition: Option<Conditions>,  // Partial index WHERE
    pub parameters: Option<Vec<(String, String)>>,  // WITH parameters
}

pub struct IndexFieldDef {
    pub name: String,
    pub direction: OrderDir,
    pub op_class: Option<String>,
}
```

## Renderer Trait

```rust
pub trait Renderer {
    // ── Top-level ──
    fn render_query(&self, stmt: &QueryStmt, ctx: &mut RenderCtx) -> Result<()>;
    fn render_mutation(&self, stmt: &MutationStmt, ctx: &mut RenderCtx) -> Result<()>;
    fn render_schema_mutation(&self, stmt: &SchemaMutationStmt, ctx: &mut RenderCtx) -> Result<()>;

    // ── SELECT parts ──
    fn render_select_columns(&self, cols: &[SelectColumn], ctx: &mut RenderCtx) -> Result<()>;
    fn render_from(&self, source: &TableSource, ctx: &mut RenderCtx) -> Result<()>;
    fn render_joins(&self, joins: &[JoinDef], ctx: &mut RenderCtx) -> Result<()>;
    fn render_where(&self, cond: &Conditions, ctx: &mut RenderCtx) -> Result<()>;
    fn render_order_by(&self, order: &[OrderByDef], ctx: &mut RenderCtx) -> Result<()>;
    fn render_limit(&self, limit: &LimitDef, ctx: &mut RenderCtx) -> Result<()>;
    fn render_ctes(&self, ctes: &[CteDef], ctx: &mut RenderCtx) -> Result<()>;
    fn render_lock(&self, lock: &SelectLockDef, ctx: &mut RenderCtx) -> Result<()>;

    // ── Expressions ──
    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> Result<()>;
    fn render_aggregate(&self, agg: &AggregationDef, ctx: &mut RenderCtx) -> Result<()>;
    fn render_window(&self, win: &WindowDef, ctx: &mut RenderCtx) -> Result<()>;
    fn render_case(&self, case: &CaseDef, ctx: &mut RenderCtx) -> Result<()>;

    // ── Conditions ──
    fn render_condition(&self, cond: &Conditions, ctx: &mut RenderCtx) -> Result<()>;
    fn render_compare_op(&self, op: &CompareOp, left: &Expr, right: &Expr, ctx: &mut RenderCtx) -> Result<()>;

    // ── DML ──
    fn render_insert(&self, stmt: &InsertStmt, ctx: &mut RenderCtx) -> Result<()>;
    fn render_update(&self, stmt: &UpdateStmt, ctx: &mut RenderCtx) -> Result<()>;
    fn render_delete(&self, stmt: &DeleteStmt, ctx: &mut RenderCtx) -> Result<()>;
    fn render_on_conflict(&self, oc: &OnConflictDef, ctx: &mut RenderCtx) -> Result<()>;
    fn render_returning(&self, fields: &[FieldRef], ctx: &mut RenderCtx) -> Result<()>;

    // ── DDL ──
    fn render_column_type(&self, ty: &FieldType, ctx: &mut RenderCtx) -> Result<()>;
    fn render_constraint(&self, c: &ConstraintDef, ctx: &mut RenderCtx) -> Result<()>;
    fn render_index(&self, idx: &IndexDef, ctx: &mut RenderCtx) -> Result<()>;

    // ── Identifiers ──
    fn quote_ident(&self, name: &str) -> String;
}
```

## RenderCtx

```rust
pub struct RenderCtx {
    sql: String,
    params: Vec<Value>,
    param_style: ParamStyle,  // Dollar ($1) | QMark (?) | Format (%s)
}

impl RenderCtx {
    pub fn new(param_style: ParamStyle) -> Self {
        Self {
            sql: String::with_capacity(256),
            params: Vec::new(),
            param_style,
        }
    }

    // ── Semantic writing methods (chainable) ──

    /// Write a SQL keyword (e.g. SELECT, FROM, CAST, AS).
    /// Automatically adds a leading space when needed.
    pub fn keyword(&mut self, kw: &str) -> &mut Self;

    /// Write a quoted identifier (e.g. "users", "age").
    pub fn ident(&mut self, name: &str) -> &mut Self;

    /// Write a parameterized value. Returns the placeholder ($1, ?, %s).
    pub fn param(&mut self, val: Value) -> &mut Self;

    /// Write a string literal with proper escaping and quoting.
    pub fn string_literal(&mut self, s: &str) -> &mut Self;

    /// Write an operator (e.g. ::, =, >, ||).
    pub fn operator(&mut self, op: &str) -> &mut Self;

    /// Write opening parenthesis.
    pub fn paren_open(&mut self) -> &mut Self;

    /// Write closing parenthesis.
    pub fn paren_close(&mut self) -> &mut Self;

    /// Write a comma separator.
    pub fn comma(&mut self) -> &mut Self;

    /// Write a space.
    pub fn space(&mut self) -> &mut Self;

    /// Write arbitrary text (escape hatch, use sparingly).
    pub fn write(&mut self, s: &str) -> &mut Self;
}
```

Usage example:

```rust
// PostgreSQL CAST: field::type
fn render_cast(&self, expr: &Expr, to_type: &str, ctx: &mut RenderCtx) -> Result<()> {
    self.render_expr(expr, ctx)?;
    ctx.operator("::");
    ctx.write(to_type);
    Ok(())
}

// SQLite CAST: CAST(field AS type)
fn render_cast(&self, expr: &Expr, to_type: &str, ctx: &mut RenderCtx) -> Result<()> {
    ctx.keyword("CAST").paren_open();
    self.render_expr(expr, ctx)?;
    ctx.keyword("AS").write(to_type).paren_close();
    Ok(())
}
```

## Crate Structure

```
rquery/
├── rquery-core/        # IR types + Renderer trait + RenderCtx + Value + Custom* traits
├── rquery-postgres/    # PostgresRenderer implementation
├── rquery-sqlite/      # SqliteRenderer implementation
└── rquery/             # Umbrella crate, re-exports everything
```

## Custom Extension Traits

All extension traits follow the same pattern:

```rust
pub trait CustomExpr: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomExpr>;
}

pub trait CustomCondition: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomCondition>;
}

pub trait CustomCompareOp: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomCompareOp>;
}

pub trait CustomTableSource: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomTableSource>;
}

pub trait CustomMutation: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomMutation>;
}

pub trait CustomSchemaMutation: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomSchemaMutation>;
}
```

## Full Extensibility Example

Scenario: PostgreSQL releases a new feature — `COUNT(id) FILTER (WHERE age > 10)` with some new syntax. A user of rquery can add support immediately:

```rust
use rquery_core::{Expr, CustomExpr, Renderer, RenderCtx};
use rquery_postgres::PostgresRenderer;

// 1. Define the custom AST node
#[derive(Debug, Clone)]
struct CountIf {
    field: Expr,
    condition: Expr,
}

impl CustomExpr for CountIf {
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn CustomExpr> { Box::new(self.clone()) }
}

// 2. Wrap the standard renderer
struct MyPostgresRenderer {
    inner: PostgresRenderer,
}

impl Renderer for MyPostgresRenderer {
    fn render_expr(&self, expr: &Expr, ctx: &mut RenderCtx) -> Result<()> {
        if let Expr::Custom(custom) = expr {
            if let Some(count_if) = custom.as_any().downcast_ref::<CountIf>() {
                ctx.write("COUNT(");
                self.render_expr(&count_if.field, ctx)?;
                ctx.write(") FILTER (WHERE ");
                self.render_expr(&count_if.condition, ctx)?;
                ctx.write(")");
                return Ok(());
            }
        }
        self.inner.render_expr(expr, ctx)
    }

    // Delegate all other methods
    delegate_renderer!(self.inner);
}

// 3. Use it
let query = Query::select()
    .expr(Expr::custom(CountIf {
        field: Expr::field("id"),
        condition: Expr::field("age").gt(Expr::val(10)),
    }))
    .from("users")
    .build();

let renderer = MyPostgresRenderer { inner: PostgresRenderer::new() };
let (sql, params) = renderer.render(&query)?;
// sql = SELECT COUNT("id") FILTER (WHERE "age" > $1) FROM "users"
// params = [Value::Int(10)]
```
