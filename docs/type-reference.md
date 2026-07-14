# Type Reference

Quick reference of the key types in qcraft.

## Value

Enum representing a database value. Used in expressions, parameters, and defaults.

```rust
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    BigInt(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    Date(String),
    DateTime(String),
    Time(String),
    Decimal(String),
    Uuid(String),
    Json(String),
    Jsonb(String),
    IpNetwork(String),
    Array(Vec<Value>),
    Vector(Vec<f32>),
    TimeDelta {
        years: i32,
        months: i32,
        days: i64,
        seconds: i64,
        microseconds: i64,
    },
}
```

### From implementations

| Source type | Target variant |
|---|---|
| `i64` | `Value::Int` |
| `i32` | `Value::Int` (widened to i64) |
| `f64` | `Value::Float` |
| `bool` | `Value::Bool` |
| `String` | `Value::Str` |
| `&str` | `Value::Str` |
| `Vec<u8>` | `Value::Bytes` |

String-based variants (`Date`, `DateTime`, `Time`, `Decimal`, `Uuid`, `Json`, `Jsonb`, `IpNetwork`) must be constructed directly -- there are no `From` impls to avoid ambiguity.

### Int vs BigInt

Both store `i64`, but they map to different PostgreSQL wire types:

- `Value::Int` — maps to PG `int4` (`i32`) when the value fits, otherwise `int8`. Use for column values.
- `Value::BigInt` — always maps to PG `int8`. Used internally by the renderer for LIMIT/OFFSET parameters.

In SQLite both behave identically (SQLite has a single INTEGER type).

### NULL parameterization

In parameterized query mode (SELECT, INSERT, UPDATE, DELETE), `Value::Null` is sent as a bind parameter (`$1` / `?`), not inlined as the `NULL` keyword. This allows drivers to handle NULL correctly via the wire protocol.

The exception is `CompareOp::IsNull`: its boolean `right` selects the keyword (`Bool(true)` → `IS NULL`, `Bool(false)` → `IS NOT NULL`) and is never emitted as a bind parameter, since `IS $1` is not valid SQL syntax. A non-boolean `right` is a render error.

## Expr

Enum representing an expression in SQL.

```rust
pub enum Expr {
    Value(Value),
    Field(FieldRef),
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Unary { op: UnaryOp, expr: Box<Expr> },
    Func { name: String, args: Vec<Expr> },
    Aggregate(AggregationDef),
    Cast { expr: Box<Expr>, to_type: String },
    Case(CaseDef),
    Window(WindowDef),
    Exists(Box<QueryStmt>),
    SubQuery(Box<QueryStmt>),
    ArraySubQuery(Box<QueryStmt>),
    Collate { expr: Box<Expr>, collation: String },
    Raw { sql: String, params: Vec<Value> },
    JsonArray(Vec<Expr>),
    JsonObject(Vec<(String, Expr)>),
    JsonAgg { expr: Box<Expr>, distinct: bool, filter: Option<Conditions>, order_by: Option<Vec<OrderByDef>> },
    StringAgg { expr: Box<Expr>, delimiter: String, distinct: bool, filter: Option<Conditions>, order_by: Option<Vec<OrderByDef>> },
    Now,
    Custom(Box<dyn CustomExpr>),
}
```

### Constructors

| Constructor | Result |
|---|---|
| `Expr::field("t", "col")` | `Expr::Field(FieldRef::new("t", "col"))` |
| `Expr::value(42)` | `Expr::Value(Value::Int(42))` |
| `Expr::raw("NOW()")` | `Expr::Raw { sql: "NOW()", params: vec![] }` |
| `Expr::func("COALESCE", vec![...])` | `Expr::Func { name: "COALESCE", args: [...] }` |
| `Expr::cast(expr, "bigint")` | `Expr::Cast { expr, to_type: "bigint" }` |
| `Expr::count(expr)` | `Expr::Aggregate(...)` with name "COUNT" |
| `Expr::count_all()` | `Expr::Aggregate(...)` -- COUNT(*) |
| `Expr::sum(expr)` | `Expr::Aggregate(...)` with name "SUM" |
| `Expr::avg(expr)` | `Expr::Aggregate(...)` with name "AVG" |
| `Expr::min(expr)` | `Expr::Aggregate(...)` with name "MIN" |
| `Expr::max(expr)` | `Expr::Aggregate(...)` with name "MAX" |
| `Expr::exists(query)` | `Expr::Exists(Box::new(query))` |
| `Expr::subquery(query)` | `Expr::SubQuery(Box::new(query))` |
| `expr.collate("C")` | `Expr::Collate { expr, collation: "C" }` |
| `Expr::json_array(vec![...])` | `Expr::JsonArray(...)` — PG: `jsonb_build_array`, SQLite: `json_array` |
| `Expr::json_object(vec![...])` | `Expr::JsonObject(...)` — PG: `jsonb_build_object`, SQLite: `json_object` |
| `Expr::json_agg(expr)` | `Expr::JsonAgg { ... }` — PG: `jsonb_agg`, SQLite: `json_group_array` |
| `Expr::string_agg(expr, ",")` | `Expr::StringAgg { ... }` — PG: `string_agg`, SQLite: `group_concat` |
| `Expr::now()` | `Expr::Now` — PG: `now()`, SQLite: `datetime('now')` |
| `Expr::CurrentTimestamp` | `CURRENT_TIMESTAMP` keyword (both dialects) |
| `Expr::CurrentDate` | `CURRENT_DATE` keyword (both dialects) |
| `Expr::CurrentTime` | `CURRENT_TIME` keyword (both dialects) |

### From implementations

| Source | Target |
|---|---|
| `Value` | `Expr::Value(v)` |
| `FieldRef` | `Expr::Field(f)` |

### Parenthesization

The tree carries the grouping, so the renderer brackets an operand whenever its own
structure would otherwise be re-associated by the engine's operator precedence:

```rust
// (1 + 2) * 3 — flat `1 + 2 * 3` would evaluate to 7
Expr::Binary {
    left: Box::new(Expr::Binary { left: one, op: BinaryOp::Add, right: two }),
    op: BinaryOp::Mul,
    right: three,
}
// PG and SQLite: (1 + 2) * 3
```

Operands that are `Binary`, `Unary`, `Collate`, `JsonPathText`, `Window`, or a `Field`
whose `FieldDef` carries a JSON child get brackets; self-delimiting forms (literals,
fields, function calls, `CAST(…)`, `CASE … END`, subqueries, tuples) render bare.
Bracketing is structural rather than driven by a precedence table, because precedence
differs per dialect — SQLite binds `||` tighter than `*`, PostgreSQL binds it looser
than `+`.

`Raw` and `Custom` are **never** bracketed automatically: their contents are opaque and
need not be an expression at all. Write the grouping into the fragment itself:

```rust
Expr::cast(Expr::raw("price * qty"), "numeric")     // price * qty::numeric  ← cast binds to qty
Expr::cast(Expr::raw("(price * qty)"), "numeric")   // (price * qty)::numeric
```

## FieldRef / FieldDef

A field reference with optional schema namespace and JSON child path.

```rust
pub struct FieldRef {
    pub field: FieldDef,
    pub table_name: String,
    pub namespace: Option<String>,  // schema prefix: "public"."table"."col"
}

pub struct FieldDef {
    pub name: String,
    pub child: Option<Box<FieldDef>>,  // JSON path: "col"->'key'->'nested'
}
```

Constructor: `FieldRef::new("table", "column")` — no namespace, no child.

With namespace: renders as `"public"."users"."id"`.

With child chain: renders as `"users"."data"->'address'->'city'` (always `->`, use `Expr::Cast` for text extraction).

## CompareOp

Enum of comparison operators used in `Comparison` nodes inside `Conditions`.

```
Eq              =
Neq             != / <>
Gt              >
Gte             >=
Lt              <
Lte             <=
In              IN (Value::Array expands to $1, $2, ...)
Like            LIKE (raw pattern, caller provides wildcards)
ILike           ILIKE (PG) / LOWER(col) LIKE LOWER(?) (SQLite)
Contains        LIKE '%val%' (auto-escaped)
StartsWith      LIKE 'val%' (auto-escaped)
EndsWith        LIKE '%val' (auto-escaped)
IContains       ILIKE '%val%' (PG) / LOWER(col) LIKE LOWER(?) (SQLite)
IStartsWith     ILIKE 'val%' (PG) / LOWER(col) LIKE LOWER(?) (SQLite)
IEndsWith       ILIKE '%val' (PG) / LOWER(col) LIKE LOWER(?) (SQLite)
Between         BETWEEN $1 AND $2 (Value::Array with 2 items)
IsNull          IS NULL / IS NOT NULL   (selected by boolean `right`: true → IS NULL, false → IS NOT NULL)
Similar         SIMILAR TO (PG)
Regex           ~ (PG)
IRegex          ~* (PG) / REGEXP '(?i)' || pattern (SQLite)
JsonbContains       @> (PG JSONB)
JsonbContainedBy    <@ (PG JSONB)
JsonbHasKey         ? (PG JSONB)
JsonbHasAnyKey      ?| (PG JSONB, auto-appends ::text[])
JsonbHasAllKeys     ?& (PG JSONB, auto-appends ::text[])
FtsMatch            @@ (PG full-text search)
TrigramSimilar          % (PG trigram)
TrigramWordSimilar      <% (PG trigram)
TrigramStrictWordSimilar    <<% (PG trigram)
RangeContains       @> (PG range)
RangeContainedBy    <@ (PG range)
RangeOverlap        && (PG range)
RangeStrictlyLeft   << (PG range)
RangeStrictlyRight  >> (PG range)
RangeNotLeft        &> (PG range)
RangeNotRight       &< (PG range)
RangeAdjacent       -|- (PG range)
Custom(Box<dyn CustomCompareOp>)
```

## BinaryOp

Enum of binary arithmetic/string operators used in `Expr::Binary`.

```
Add             +
Sub             -
Mul             *
Div             /
Mod             %
BitwiseAnd      &
BitwiseOr       |
ShiftLeft       <<
ShiftRight      >>
Concat          ||
Custom(Box<dyn CustomBinaryOp>)
```

### PgVectorOp (qcraft-postgres)

Ready-to-use pgvector distance operators, implements `CustomBinaryOp`:

```
PgVectorOp::L2Distance       <->
PgVectorOp::InnerProduct      <#>
PgVectorOp::CosineDistance     <=>
PgVectorOp::L1Distance        <+>
```

Usage:
```rust
Expr::Binary {
    left: Box::new(Expr::field("items", "embedding")),
    op: PgVectorOp::L2Distance.into(),
    right: Box::new(Expr::Value(Value::Vector(vec![1.0, 2.0, 3.0]))),
}
```

## UnaryOp

```
Neg             - (negation)
Not             NOT
BitwiseNot      ~
```

## FieldType

Enum representing a column type in DDL.

```rust
pub enum FieldType {
    Scalar(String),                                // e.g. "bigint", "text", "boolean"
    Parameterized { name: String, params: Vec<String> },  // e.g. VARCHAR(255), NUMERIC(10,2)
    Array(Box<FieldType>),                         // e.g. INTEGER[], TEXT[]
    Vector(i64),                                   // pgvector: VECTOR(1536)
    Custom(Box<dyn CustomFieldType>),
}
```

Constructors: `FieldType::scalar("bigint")`, `FieldType::parameterized("varchar", vec!["255"])`.

### `FieldType::Decimal { precision, scale }`

Fixed-point decimal, rendered per dialect:

| Variant | PostgreSQL | SQLite |
| --- | --- | --- |
| `Decimal { None, None }` | `NUMERIC` | `DECIMAL_TEXT` |
| `Decimal { Some(p), None }` | `NUMERIC(p)` | `DECIMAL_TEXT(p)` |
| `Decimal { Some(p), Some(s) }` | `NUMERIC(p, s)` | `DECIMAL_TEXT(p, s)` |
| `Decimal { None, Some(s) }` | render error | render error |

Convenience constructor: `FieldType::decimal(p, s)` for the `NUMERIC(p, s)` form.

On SQLite the type name is deliberately `DECIMAL_TEXT` so the column gets **TEXT
affinity** (the name contains `TEXT`), which stores decimal strings without the
precision loss that `NUMERIC`/`DECIMAL` (NUMERIC affinity → float) would cause. SQLite
does **not** enforce the `(p, s)` — they are documentary. Bind decimal values as
`Value::Decimal` (carried as a string) so they store as text.

## Conditions

A tree of conditions connected by AND/OR. Core type for WHERE and HAVING clauses.

### Constructors

| Constructor | SQL equivalent |
|---|---|
| `Conditions::eq(field, expr)` | `field = expr` |
| `Conditions::neq(field, expr)` | `field <> expr` |
| `Conditions::gt(field, expr)` | `field > expr` |
| `Conditions::gte(field, expr)` | `field >= expr` |
| `Conditions::lt(field, expr)` | `field < expr` |
| `Conditions::lte(field, expr)` | `field <= expr` |
| `Conditions::is_null(field)` | `field IS NULL` |
| `Conditions::is_not_null(field)` | `field IS NOT NULL` |
| `Conditions::like(field, pattern)` | `field LIKE pattern` |
| `Conditions::in_subquery(field, query)` | `field IN (subquery)` |

### Combinators

| Method | SQL equivalent |
|---|---|
| `cond1.and_also(cond2)` | `(cond1) AND (cond2)` |
| `cond1.or_else(cond2)` | `(cond1) OR (cond2)` |
| `cond.negated()` | `NOT (cond)` |

### Building with AND/OR groups

```rust
Conditions::and(vec![
    ConditionNode::Comparison(Box::new(Comparison::new(left, CompareOp::Eq, right))),
    ConditionNode::Group(other_conditions),
    ConditionNode::Exists(Box::new(subquery)),
])
```

## AggregationDef

Aggregate function definition used in `Expr::Aggregate`.

```rust
pub struct AggregationDef {
    pub name: String,              // "COUNT", "SUM", "AVG", etc.
    pub expression: Option<Box<Expr>>,  // None for COUNT(*)
    pub distinct: bool,            // SUM(DISTINCT expr)
    pub filter: Option<Conditions>,     // FILTER (WHERE ...) -- PG
    pub args: Option<Vec<Expr>>,        // Additional arguments
    pub order_by: Option<Vec<OrderByDef>>,  // ORDER BY inside aggregate -- PG
}
```

Builder methods: `AggregationDef::new("SUM", expr).distinct().filter(cond).order_by(order)`.

## SelectColumn

Represents a column in the SELECT clause.

| Constructor | SQL |
|---|---|
| `SelectColumn::all()` | `*` |
| `SelectColumn::all_from("t")` | `t.*` |
| `SelectColumn::field("t", "col")` | `"t"."col"` |
| `SelectColumn::expr(expr)` | expression without alias |
| `SelectColumn::aliased(expr, "name")` | `expr AS "name"` |
| `SelectColumn::field_aliased("t", "col", "alias")` | `"t"."col" AS "alias"` |
