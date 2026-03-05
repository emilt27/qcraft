# DML AST Redesign Notes

Issues with the current `crates/rquery-core/src/ast/dml.rs` identified during research.

## InsertStmt

- `DataRow.data: Vec<(String, Value)>` → should be `Vec<(String, Expr)>` — values can be expressions (DEFAULT, functions, subqueries), not just literals.
- `InsertFromSelectStmt` should be merged into `InsertStmt` — query as alternative to VALUES.
- `columns: Option<Vec<FieldRef>>` in InsertFromSelect → should be `Option<Vec<String>>` (just column names).
- Missing: CTE (`WITH` clause) — needed for all DML.
- Missing: `OVERRIDING { SYSTEM | USER } VALUE` (PG identity columns).

## OnConflictDef

- `fields: Vec<FieldRef>` → conflict target should be:
  - `Vec<String>` (column names, not FieldRef)
  - OR `ON CONSTRAINT constraint_name`
  - With optional `WHERE` for partial index matching
- `update_fields: Option<Vec<FieldRef>>` → should be `Vec<(String, Expr)>` (assignments, not just field list).
- Missing: `WHERE` clause on `DO UPDATE SET` (conditional update).

## UpdateStmt

- Missing: CTE (`WITH` clause).
- Missing: table alias.
- `from_tables: Option<Vec<TableSource>>` — good but may need rethinking for MySQL multi-table syntax.

## DeleteStmt

- Missing: `USING` / `FROM` for JOIN (PG uses USING, SQL Server uses FROM).
- Missing: CTE (`WITH` clause).
- Missing: table alias.

## RETURNING

- `Option<Vec<FieldRef>>` → should be `Option<Vec<Expr>>` (can be expressions, `*`, aliases).
- PG/SQLite return result sets; Oracle returns INTO variables; SQL Server uses OUTPUT DELETED/INSERTED.

## MutationStmt

- `Truncate(SchemaRef)` is a duplicate — already moved to `SchemaMutationStmt::TruncateTable` in DDL.
- Remove from DML enum.

## Proposed New Structure

```rust
pub struct InsertStmt {
    pub table: SchemaRef,
    pub alias: Option<String>,
    pub columns: Option<Vec<String>>,
    pub source: InsertSource,  // Values | Select | DefaultValues
    pub on_conflict: Option<OnConflictDef>,
    pub returning: Option<Vec<SelectColumn>>,  // reuse from query
    pub ctes: Option<Vec<CteDef>>,
    pub overriding: Option<OverridingKind>,  // PG identity
}

pub enum InsertSource {
    Values(Vec<Vec<Expr>>),  // rows of expressions
    Select(Box<QueryStmt>),
    DefaultValues,
}

pub struct OnConflictDef {
    pub target: Option<ConflictTarget>,  // None = catch-all (SQLite)
    pub action: ConflictAction,
}

pub enum ConflictTarget {
    Columns {
        columns: Vec<String>,
        where_clause: Option<Conditions>,  // partial index
    },
    Constraint(String),  // ON CONSTRAINT name
}

pub enum ConflictAction {
    DoNothing,
    DoUpdate {
        assignments: Vec<(String, Expr)>,
        where_clause: Option<Conditions>,
    },
}

pub struct UpdateStmt {
    pub table: SchemaRef,
    pub alias: Option<String>,
    pub assignments: Vec<(String, Expr)>,
    pub from: Option<Vec<TableSource>>,  // PG/SQLite/Oracle/MSSQL
    pub where_clause: Option<Conditions>,
    pub returning: Option<Vec<SelectColumn>>,
    pub ctes: Option<Vec<CteDef>>,
}

pub struct DeleteStmt {
    pub table: SchemaRef,
    pub alias: Option<String>,
    pub using: Option<Vec<TableSource>>,  // PG USING / MSSQL FROM
    pub where_clause: Option<Conditions>,
    pub returning: Option<Vec<SelectColumn>>,
    pub ctes: Option<Vec<CteDef>>,
}
```
