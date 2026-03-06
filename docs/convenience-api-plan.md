# Convenience API Plan

Мета: зробити AST зручним публічним API без Builder pattern.
Підхід: `Default` для великих структур + конструктори/хелпери на типах.
Принцип: нуль обмежень — хелпери для типових кейсів, повний AST для складних.

## Що вже є

- `SchemaRef::new()`, `with_namespace()`, `with_alias()`
- `FieldDef::new()`, `FieldRef::new()`
- `SchemaDef::new()`
- `ColumnDef::new()`, `not_null()`, `default()`
- `IndexDef::new()`, `unique()`
- `FieldType::scalar()`, `parameterized()`
- `Conditions::and()`, `or()`, `negated()`
- `FromItem::table()`, `subquery()`
- `IdentityColumn: Default`

---

## Plan

### 1. Default для великих структур

Всі структури з купою `Option<...>` полів мають отримати `Default`.

```rust
// query.rs
QueryStmt: Default          // всі поля None, columns = vec![]
LimitDef: —                 // немає сенсу, маленька структура
CteDef: —                   // обов'язкові поля (name, query)
SelectLockDef: —            // обов'язкове поле (strength)
WindowNameDef: —            // обов'язкове поле (name)

// dml.rs
InsertStmt: Default         // table = SchemaRef::new(""), source = DefaultValues
UpdateStmt: Default         // table = SchemaRef::new(""), assignments = vec![]
DeleteStmt: Default         // table = SchemaRef::new("")

// ddl.rs
ColumnDef: —                // обов'язкові name + field_type, вже є new()
IndexColumnDef: —           // обов'язкове expr

// tcl.rs
BeginStmt: Default          // всі поля None
CommitStmt: Default         // всі false/None
RollbackStmt: Default       // всі None/false

// expr.rs
AggregationDef: —           // обов'язкове name
WindowDef: —                // обов'язкове expression
```

### 2. Expr — конструктори

Найчастіше використовуваний тип. Потрібні короткі шляхи.

```rust
impl Expr {
    fn field(table: &str, name: &str) -> Self
    fn value(val: impl Into<Value>) -> Self
    fn raw(sql: &str) -> Self
    fn func(name: &str, args: Vec<Expr>) -> Self
    fn cast(expr: Expr, to_type: &str) -> Self
    fn count(expr: Expr) -> Self           // Aggregate shortcut
    fn sum(expr: Expr) -> Self
    fn avg(expr: Expr) -> Self
    fn min(expr: Expr) -> Self
    fn max(expr: Expr) -> Self
    fn count_all() -> Self                 // COUNT(*)
    fn exists(query: QueryStmt) -> Self
    fn subquery(query: QueryStmt) -> Self
}
```

### 3. Value — From implementations

```rust
impl From<i64> for Value        // Value::Int
impl From<i32> for Value        // Value::Int (as i64)
impl From<f64> for Value        // Value::Float
impl From<bool> for Value       // Value::Bool
impl From<String> for Value     // Value::Str
impl From<&str> for Value       // Value::Str
impl From<Vec<u8>> for Value    // Value::Bytes
```

### 4. Conditions — зручні конструктори

```rust
impl Conditions {
    // Прості порівняння (повертають Conditions з одним Comparison)
    fn eq(field: FieldRef, val: impl Into<Expr>) -> Self
    fn neq(field: FieldRef, val: impl Into<Expr>) -> Self
    fn gt(field: FieldRef, val: impl Into<Expr>) -> Self
    fn gte(field: FieldRef, val: impl Into<Expr>) -> Self
    fn lt(field: FieldRef, val: impl Into<Expr>) -> Self
    fn lte(field: FieldRef, val: impl Into<Expr>) -> Self
    fn is_null(field: FieldRef) -> Self
    fn is_not_null(field: FieldRef) -> Self
    fn like(field: FieldRef, pattern: &str) -> Self
    fn between(field: FieldRef, low: Expr, high: Expr) -> Self
    fn in_list(field: FieldRef, values: Vec<Expr>) -> Self
    fn in_subquery(field: FieldRef, query: QueryStmt) -> Self

    // Комбінування
    fn and_also(self, other: Conditions) -> Self   // self AND other
    fn or_else(self, other: Conditions) -> Self    // self OR other
}

impl Comparison {
    fn new(left: Expr, op: CompareOp, right: Expr) -> Self
}
```

### 5. SelectColumn — конструктори

```rust
impl SelectColumn {
    fn all() -> Self                                    // *
    fn all_from(table: &str) -> Self                    // table.*
    fn field(table: &str, name: &str) -> Self           // table.field
    fn expr(expr: Expr) -> Self                         // вираз без alias
    fn aliased(expr: Expr, alias: &str) -> Self         // вираз AS alias
    fn field_aliased(table: &str, name: &str, alias: &str) -> Self
}
```

### 6. FromItem — додаткові конструктори

```rust
impl FromItem {
    // Вже є: table(), subquery()
    fn lateral(inner: FromItem) -> Self
    fn function(name: &str, args: Vec<Expr>, alias: &str) -> Self
    fn values(rows: Vec<Vec<Expr>>, alias: &str) -> Self
}
```

### 7. JoinDef — конструктори

```rust
impl JoinDef {
    fn inner(source: FromItem, on: Conditions) -> Self
    fn left(source: FromItem, on: Conditions) -> Self
    fn right(source: FromItem, on: Conditions) -> Self
    fn full(source: FromItem, on: Conditions) -> Self
    fn cross(source: FromItem) -> Self
    fn using(join_type: JoinType, source: FromItem, columns: Vec<String>) -> Self
    fn natural(self) -> Self      // fluent: додає natural = true
}
```

### 8. OrderByDef — конструктори

```rust
impl OrderByDef {
    fn asc(expr: Expr) -> Self
    fn desc(expr: Expr) -> Self
    fn nulls_first(self) -> Self    // fluent
    fn nulls_last(self) -> Self     // fluent
}
```

### 9. LimitDef — конструктори

```rust
impl LimitDef {
    fn limit(count: u64) -> Self
    fn limit_offset(count: u64, offset: u64) -> Self
    fn fetch_first(count: u64) -> Self
    fn fetch_first_with_ties(count: u64) -> Self
    fn top(count: u64) -> Self
    fn offset(self, offset: u64) -> Self   // fluent
}
```

### 10. CteDef — конструктори

```rust
impl CteDef {
    fn new(name: &str, query: QueryStmt) -> Self
    fn recursive(name: &str, query: QueryStmt) -> Self
    fn columns(self, cols: Vec<&str>) -> Self            // fluent
    fn materialized(self) -> Self                         // fluent
    fn not_materialized(self) -> Self                     // fluent
}
```

### 11. DML — конструктори

```rust
impl InsertStmt {
    fn values(table: &str, columns: Vec<&str>, rows: Vec<Vec<Expr>>) -> Self
    fn from_select(table: &str, columns: Vec<&str>, query: QueryStmt) -> Self
    fn default_values(table: &str) -> Self
    fn returning(self, cols: Vec<SelectColumn>) -> Self   // fluent
    fn on_conflict(self, def: OnConflictDef) -> Self      // fluent
}

impl UpdateStmt {
    fn new(table: &str, assignments: Vec<(&str, Expr)>) -> Self
    fn where_clause(self, cond: Conditions) -> Self       // fluent
    fn returning(self, cols: Vec<SelectColumn>) -> Self   // fluent
}

impl DeleteStmt {
    fn new(table: &str) -> Self
    fn where_clause(self, cond: Conditions) -> Self       // fluent
    fn returning(self, cols: Vec<SelectColumn>) -> Self   // fluent
}

impl OnConflictDef {
    fn do_nothing() -> Self
    fn do_update(columns: Vec<&str>, assignments: Vec<(&str, Expr)>) -> Self
}
```

### 12. DDL — додаткові хелпери

```rust
impl ColumnDef {
    // Вже є: new(), not_null(), default()
    fn generated(self, sql: &str, stored: bool) -> Self
    fn collation(self, name: &str) -> Self
}

impl ConstraintDef {
    fn primary_key(columns: Vec<&str>) -> Self
    fn foreign_key(columns: Vec<&str>, ref_table: &str, ref_columns: Vec<&str>) -> Self
    fn unique(columns: Vec<&str>) -> Self
    fn check(condition: Conditions) -> Self
}

impl IndexColumnDef {
    fn column(name: &str) -> Self
    fn expression(expr: Expr) -> Self
    fn asc(self) -> Self            // fluent
    fn desc(self) -> Self           // fluent
    fn nulls_first(self) -> Self    // fluent
    fn nulls_last(self) -> Self     // fluent
}

impl SchemaMutationStmt {
    fn create_table(schema: SchemaDef) -> Self
    fn drop_table(name: &str) -> Self
    fn drop_table_if_exists(name: &str) -> Self
    fn create_index(table: &str, index: IndexDef) -> Self
    fn drop_index(table: &str, name: &str) -> Self
    fn add_column(table: &str, column: ColumnDef) -> Self
    fn drop_column(table: &str, name: &str) -> Self
    fn rename_table(old: &str, new: &str) -> Self
    fn rename_column(table: &str, old: &str, new: &str) -> Self
    fn truncate(table: &str) -> Self
}
```

### 13. TCL — конструктори

```rust
impl TransactionStmt {
    fn begin() -> Self
    fn commit() -> Self
    fn rollback() -> Self
    fn savepoint(name: &str) -> Self
    fn release(name: &str) -> Self
    fn rollback_to(name: &str) -> Self
}

impl BeginStmt {
    fn with_isolation(level: IsolationLevel) -> Self
    fn read_only() -> Self
    fn sqlite_deferred() -> Self
    fn sqlite_immediate() -> Self
    fn sqlite_exclusive() -> Self
}
```

### 14. SetOpDef — конструктори

```rust
impl SetOpDef {
    fn union(left: QueryStmt, right: QueryStmt) -> Self
    fn union_all(left: QueryStmt, right: QueryStmt) -> Self
    fn intersect(left: QueryStmt, right: QueryStmt) -> Self
    fn except(left: QueryStmt, right: QueryStmt) -> Self
}
```

### 15. AggregationDef — конструктори

```rust
impl AggregationDef {
    fn new(name: &str, expr: Expr) -> Self
    fn count_all() -> Self
    fn distinct(self) -> Self       // fluent
    fn filter(self, cond: Conditions) -> Self  // fluent
    fn order_by(self, order: Vec<OrderByDef>) -> Self  // fluent (для string_agg тощо)
}
```

---

## Пріоритет реалізації

1. **Value: From impls** — найпростіше, одразу скорочує код всюди
2. **Expr конструктори** — найчастіше використовується
3. **Conditions конструктори** — другий за частотою
4. **Default для QueryStmt, InsertStmt, UpdateStmt, DeleteStmt, BeginStmt, CommitStmt, RollbackStmt**
5. **SelectColumn, OrderByDef, LimitDef** — часто в DQL
6. **JoinDef, CteDef, FromItem** — DQL розширення
7. **DDL хелпери** — ConstraintDef, IndexColumnDef, SchemaMutationStmt
8. **DML хелпери** — InsertStmt, UpdateStmt, DeleteStmt, OnConflictDef
9. **TCL хелпери** — TransactionStmt, BeginStmt
10. **SetOpDef, AggregationDef** — рідше використовуються

## Приклад: до і після

### SELECT з WHERE (до)
```rust
let stmt = QueryStmt {
    columns: vec![
        SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        },
        SelectColumn::Field {
            field: FieldRef::new("users", "email"),
            alias: None,
        },
    ],
    from: Some(vec![FromItem {
        source: TableSource::Table(SchemaRef::new("users")),
        only: false,
        sample: None,
        index_hint: None,
    }]),
    where_clause: Some(Conditions {
        children: vec![ConditionNode::Comparison(Comparison {
            left: Expr::Field(FieldRef::new("users", "age")),
            op: CompareOp::Gte,
            right: Expr::Value(Value::Int(18)),
            negate: false,
        })],
        connector: Connector::And,
        negated: false,
    }),
    joins: None,
    group_by: None,
    having: None,
    window: None,
    order_by: None,
    limit: None,
    lock: None,
    distinct: None,
    ctes: None,
};
```

### SELECT з WHERE (після)
```rust
let stmt = QueryStmt {
    columns: vec![
        SelectColumn::field("users", "name"),
        SelectColumn::field("users", "email"),
    ],
    from: Some(vec![FromItem::table("users")]),
    where_clause: Some(
        Conditions::gte(FieldRef::new("users", "age"), Value::Int(18))
    ),
    ..Default::default()
};
```

### INSERT (до)
```rust
let stmt = MutationStmt::Insert(InsertStmt {
    table: SchemaRef::new("users"),
    columns: Some(vec!["name".into(), "email".into()]),
    source: InsertSource::Values(vec![
        vec![Expr::Value(Value::Str("John".into())), Expr::Value(Value::Str("john@example.com".into()))],
    ]),
    on_conflict: None,
    returning: None,
    ctes: None,
    overriding: None,
    conflict_resolution: None,
    partition: None,
    ignore: false,
});
```

### INSERT (після)
```rust
let stmt = MutationStmt::Insert(
    InsertStmt::values("users", vec!["name", "email"], vec![
        vec![Expr::value("John"), Expr::value("john@example.com")],
    ])
);
```
