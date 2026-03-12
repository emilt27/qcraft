# Type Reference

Quick reference of the key types in qcraft.

## Value

Enum representing a database value. Used in expressions, parameters, and defaults.

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
    Raw { sql: String, params: Vec<Value> },
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

### From implementations

| Source | Target |
|---|---|
| `Value` | `Expr::Value(v)` |
| `FieldRef` | `Expr::Field(f)` |

## CompareOp

Enum of comparison operators used in `Comparison` nodes inside `Conditions`.

```
Eq              =
Neq             != / <>
Gt              >
Gte             >=
Lt              <
Lte             <=
In              IN
Like            LIKE (raw pattern, caller provides wildcards)
ILike           ILIKE (raw pattern, PG only)
Contains        LIKE '%val%' (auto-escaped)
StartsWith      LIKE 'val%' (auto-escaped)
EndsWith        LIKE '%val' (auto-escaped)
IContains       ILIKE '%val%' (PG) / LOWER(col) LIKE LOWER(?) (SQLite)
IStartsWith     ILIKE 'val%' (PG) / LOWER(col) LIKE LOWER(?) (SQLite)
IEndsWith       ILIKE '%val' (PG) / LOWER(col) LIKE LOWER(?) (SQLite)
Between         BETWEEN
IsNull          IS NULL
Similar         SIMILAR TO (PG)
Regex           ~ (PG)
IRegex          ~* (PG)
JsonbContains       @> (PG JSONB)
JsonbContainedBy    <@ (PG JSONB)
JsonbHasKey         ? (PG JSONB)
JsonbHasAnyKey      ?| (PG JSONB)
JsonbHasAllKeys     ?& (PG JSONB)
FtsMatch            @@ (PG full-text search)
TrigramSimilar          % (PG trigram)
TrigramWordSimilar      <% (PG trigram)
TrigramStrictWordSimilar    <<% (PG trigram)
RangeContains       @> (PG range)
RangeContainedBy    <@ (PG range)
RangeOverlap        && (PG range)
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
