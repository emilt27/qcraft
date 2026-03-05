use super::common::{FieldRef, OrderByDef, SchemaRef};
use super::conditions::Conditions;
use super::custom::CustomTableSource;
use super::expr::Expr;

/// A SELECT query statement.
#[derive(Debug, Clone)]
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

/// Source of data in FROM clause.
#[derive(Debug, Clone)]
pub enum TableSource {
    Table(SchemaRef),
    SubQuery(SubQueryDef),
    SetOp(Box<SetOpDef>),
    Custom(Box<dyn CustomTableSource>),
}

/// A column in SELECT clause.
#[derive(Debug, Clone)]
pub enum SelectColumn {
    /// All columns: `*` or `table.*`.
    Star(Option<String>),

    /// An expression, optionally aliased.
    Expr {
        expr: Expr,
        alias: Option<String>,
    },

    /// A field reference, optionally aliased.
    Field {
        field: FieldRef,
        alias: Option<String>,
    },
}

/// DISTINCT clause.
#[derive(Debug, Clone)]
pub struct DistinctClause {
    /// DISTINCT ON (fields) — PostgreSQL only.
    pub on_fields: Option<Vec<FieldRef>>,
}

/// JOIN definition.
#[derive(Debug, Clone)]
pub struct JoinDef {
    pub table: TableSource,
    pub on: Option<Conditions>,
    pub join_type: JoinType,
}

/// Types of JOIN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
    InnerLateral,
    LeftLateral,
}

/// Subquery with alias.
#[derive(Debug, Clone)]
pub struct SubQueryDef {
    pub query: Box<QueryStmt>,
    pub alias: String,
}

/// Set operation (UNION, INTERSECT, EXCEPT).
#[derive(Debug, Clone)]
pub struct SetOpDef {
    pub left: Box<QueryStmt>,
    pub right: Box<QueryStmt>,
    pub operation: SetOperationType,
}

/// Set operation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOperationType {
    Union,
    UnionAll,
    Intersect,
    IntersectAll,
    Except,
    ExceptAll,
}

/// LIMIT / OFFSET / FETCH.
#[derive(Debug, Clone)]
pub struct LimitDef {
    pub limit: u64,
    pub offset: u64,
    pub with_ties: bool,
}

/// Common Table Expression (WITH clause).
#[derive(Debug, Clone)]
pub struct CteDef {
    pub name: String,
    pub query: Box<QueryStmt>,
    pub recursive: bool,
}

/// SELECT ... FOR UPDATE / SHARE.
#[derive(Debug, Clone)]
pub struct SelectLockDef {
    pub strength: LockStrength,
    pub of: Option<Vec<SchemaRef>>,
    pub nowait: bool,
    pub skip_locked: bool,
}

/// Lock strength.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStrength {
    Update,
    NoKeyUpdate,
    Share,
    KeyShare,
}
