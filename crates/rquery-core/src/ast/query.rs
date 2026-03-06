use super::common::{FieldRef, OrderByDef, SchemaRef};
use super::conditions::Conditions;
use super::custom::CustomTableSource;
use super::expr::Expr;

// ---------------------------------------------------------------------------
// SELECT statement
// ---------------------------------------------------------------------------

/// A SELECT query statement.
#[derive(Debug, Clone, Default)]
pub struct QueryStmt {
    pub ctes: Option<Vec<CteDef>>,
    pub columns: Vec<SelectColumn>,
    pub distinct: Option<DistinctDef>,
    /// FROM items. None for `SELECT 1` (no FROM clause).
    pub from: Option<Vec<FromItem>>,
    pub joins: Option<Vec<JoinDef>>,
    pub where_clause: Option<Conditions>,
    pub group_by: Option<Vec<GroupByItem>>,
    pub having: Option<Conditions>,
    pub window: Option<Vec<WindowNameDef>>,
    pub order_by: Option<Vec<OrderByDef>>,
    pub limit: Option<LimitDef>,
    /// Multiple lock clauses: PG supports `FOR UPDATE OF t1 FOR SHARE OF t2`.
    pub lock: Option<Vec<SelectLockDef>>,
}

// ---------------------------------------------------------------------------
// SELECT columns
// ---------------------------------------------------------------------------

/// A column in SELECT clause.
#[derive(Debug, Clone)]
pub enum SelectColumn {
    /// All columns: `*` or `table.*`.
    Star(Option<String>),

    /// An expression, optionally aliased.
    Expr { expr: Expr, alias: Option<String> },

    /// A field reference, optionally aliased.
    Field {
        field: FieldRef,
        alias: Option<String>,
    },
}

impl SelectColumn {
    /// `*`
    pub fn all() -> Self {
        SelectColumn::Star(None)
    }

    /// `table.*`
    pub fn all_from(table: impl Into<String>) -> Self {
        SelectColumn::Star(Some(table.into()))
    }

    /// `table.field`
    pub fn field(table: &str, name: &str) -> Self {
        SelectColumn::Field {
            field: FieldRef::new(table, name),
            alias: None,
        }
    }

    /// Expression without alias.
    pub fn expr(expr: Expr) -> Self {
        SelectColumn::Expr { expr, alias: None }
    }

    /// Expression with alias: `expr AS alias`.
    pub fn aliased(expr: Expr, alias: impl Into<String>) -> Self {
        SelectColumn::Expr {
            expr,
            alias: Some(alias.into()),
        }
    }

    /// `table.field AS alias`
    pub fn field_aliased(table: &str, name: &str, alias: impl Into<String>) -> Self {
        SelectColumn::Field {
            field: FieldRef::new(table, name),
            alias: Some(alias.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// DISTINCT
// ---------------------------------------------------------------------------

/// DISTINCT clause.
#[derive(Debug, Clone)]
pub enum DistinctDef {
    /// Plain DISTINCT (all databases).
    Distinct,
    /// DISTINCT ON (expr, ...) — PostgreSQL only.
    DistinctOn(Vec<Expr>),
}

// ---------------------------------------------------------------------------
// FROM item
// ---------------------------------------------------------------------------

/// A single item in the FROM clause, wrapping a table source with decorations.
#[derive(Debug, Clone)]
pub struct FromItem {
    pub source: TableSource,
    /// PG: ONLY (exclude inherited/child tables).
    pub only: bool,
    /// TABLESAMPLE / SAMPLE clause.
    pub sample: Option<TableSampleDef>,
    /// SQLite: INDEXED BY / NOT INDEXED.
    pub index_hint: Option<SqliteIndexHint>,
}

impl FromItem {
    pub fn table(schema_ref: SchemaRef) -> Self {
        Self {
            source: TableSource::Table(schema_ref),
            only: false,
            sample: None,
            index_hint: None,
        }
    }

    pub fn lateral(inner: FromItem) -> Self {
        Self {
            source: TableSource::Lateral(Box::new(inner)),
            only: false,
            sample: None,
            index_hint: None,
        }
    }

    pub fn function(name: impl Into<String>, args: Vec<Expr>, alias: impl Into<String>) -> Self {
        Self {
            source: TableSource::Function {
                name: name.into(),
                args,
                alias: Some(alias.into()),
            },
            only: false,
            sample: None,
            index_hint: None,
        }
    }

    pub fn values(rows: Vec<Vec<Expr>>, alias: impl Into<String>) -> Self {
        Self {
            source: TableSource::Values {
                rows,
                alias: alias.into(),
                column_aliases: None,
            },
            only: false,
            sample: None,
            index_hint: None,
        }
    }

    pub fn subquery(query: QueryStmt, alias: String) -> Self {
        Self {
            source: TableSource::SubQuery(SubQueryDef {
                query: Box::new(query),
                alias,
            }),
            only: false,
            sample: None,
            index_hint: None,
        }
    }
}

/// Source of data in FROM clause.
#[derive(Debug, Clone)]
pub enum TableSource {
    /// A table or view.
    Table(SchemaRef),
    /// A subquery with alias.
    SubQuery(SubQueryDef),
    /// Set operation (UNION/INTERSECT/EXCEPT).
    SetOp(Box<SetOpDef>),
    /// LATERAL (subquery).
    Lateral(Box<FromItem>),
    /// Table-valued function: `generate_series(1, 10)`, `json_each(col)`.
    Function {
        name: String,
        args: Vec<Expr>,
        alias: Option<String>,
    },
    /// VALUES as a table source: `(VALUES (1,'a'), (2,'b')) AS t(id, name)`.
    Values {
        rows: Vec<Vec<Expr>>,
        alias: String,
        column_aliases: Option<Vec<String>>,
    },
    /// User-defined table source (extension point).
    Custom(Box<dyn CustomTableSource>),
}

// ---------------------------------------------------------------------------
// TABLESAMPLE
// ---------------------------------------------------------------------------

/// TABLESAMPLE / SAMPLE clause.
#[derive(Debug, Clone)]
pub struct TableSampleDef {
    /// Sampling method: BERNOULLI, SYSTEM, BLOCK (Oracle).
    pub method: SampleMethod,
    /// Sample percentage (0.0 - 100.0).
    pub percentage: f64,
    /// REPEATABLE / SEED value for reproducible sampling.
    pub seed: Option<i64>,
}

/// Sampling method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleMethod {
    /// Row-level random sampling (PG, SQL Server).
    Bernoulli,
    /// Page/block-level random sampling (PG, SQL Server).
    System,
    /// Block-level sampling (Oracle: SAMPLE BLOCK).
    Block,
}

// ---------------------------------------------------------------------------
// SQLite index hints
// ---------------------------------------------------------------------------

/// SQLite-specific index hints in FROM.
#[derive(Debug, Clone)]
pub enum SqliteIndexHint {
    /// INDEXED BY index_name.
    IndexedBy(String),
    /// NOT INDEXED.
    NotIndexed,
}

// ---------------------------------------------------------------------------
// JOIN
// ---------------------------------------------------------------------------

/// JOIN definition.
#[derive(Debug, Clone)]
pub struct JoinDef {
    pub source: FromItem,
    pub condition: Option<JoinCondition>,
    pub join_type: JoinType,
    pub natural: bool,
}

impl JoinDef {
    pub fn inner(source: FromItem, on: Conditions) -> Self {
        Self {
            source,
            condition: Some(JoinCondition::On(on)),
            join_type: JoinType::Inner,
            natural: false,
        }
    }

    pub fn left(source: FromItem, on: Conditions) -> Self {
        Self {
            source,
            condition: Some(JoinCondition::On(on)),
            join_type: JoinType::Left,
            natural: false,
        }
    }

    pub fn right(source: FromItem, on: Conditions) -> Self {
        Self {
            source,
            condition: Some(JoinCondition::On(on)),
            join_type: JoinType::Right,
            natural: false,
        }
    }

    pub fn full(source: FromItem, on: Conditions) -> Self {
        Self {
            source,
            condition: Some(JoinCondition::On(on)),
            join_type: JoinType::Full,
            natural: false,
        }
    }

    pub fn cross(source: FromItem) -> Self {
        Self {
            source,
            condition: None,
            join_type: JoinType::Cross,
            natural: false,
        }
    }

    pub fn using(join_type: JoinType, source: FromItem, columns: Vec<String>) -> Self {
        Self {
            source,
            condition: Some(JoinCondition::Using(columns)),
            join_type,
            natural: false,
        }
    }

    pub fn natural(mut self) -> Self {
        self.natural = true;
        self
    }
}

/// JOIN condition.
#[derive(Debug, Clone)]
pub enum JoinCondition {
    /// ON condition.
    On(Conditions),
    /// USING (col1, col2, ...).
    Using(Vec<String>),
}

/// Types of JOIN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
    /// SQL Server / Oracle: CROSS APPLY (equivalent to INNER LATERAL JOIN).
    CrossApply,
    /// SQL Server / Oracle: OUTER APPLY (equivalent to LEFT LATERAL JOIN).
    OuterApply,
}

// ---------------------------------------------------------------------------
// Subquery
// ---------------------------------------------------------------------------

/// Subquery with alias.
#[derive(Debug, Clone)]
pub struct SubQueryDef {
    pub query: Box<QueryStmt>,
    pub alias: String,
}

// ---------------------------------------------------------------------------
// Set operations (UNION / INTERSECT / EXCEPT)
// ---------------------------------------------------------------------------

/// Set operation (UNION, INTERSECT, EXCEPT).
#[derive(Debug, Clone)]
pub struct SetOpDef {
    pub left: Box<QueryStmt>,
    pub right: Box<QueryStmt>,
    pub operation: SetOperationType,
}

impl SetOpDef {
    pub fn union(left: QueryStmt, right: QueryStmt) -> Self {
        Self {
            left: Box::new(left),
            right: Box::new(right),
            operation: SetOperationType::Union,
        }
    }

    pub fn union_all(left: QueryStmt, right: QueryStmt) -> Self {
        Self {
            left: Box::new(left),
            right: Box::new(right),
            operation: SetOperationType::UnionAll,
        }
    }

    pub fn intersect(left: QueryStmt, right: QueryStmt) -> Self {
        Self {
            left: Box::new(left),
            right: Box::new(right),
            operation: SetOperationType::Intersect,
        }
    }

    pub fn except(left: QueryStmt, right: QueryStmt) -> Self {
        Self {
            left: Box::new(left),
            right: Box::new(right),
            operation: SetOperationType::Except,
        }
    }
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

// ---------------------------------------------------------------------------
// GROUP BY
// ---------------------------------------------------------------------------

/// GROUP BY element.
#[derive(Debug, Clone)]
pub enum GroupByItem {
    /// Simple expression: `GROUP BY col1, col2`.
    Expr(Expr),
    /// ROLLUP(a, b) or MySQL `WITH ROLLUP`.
    Rollup(Vec<Expr>),
    /// CUBE(a, b).
    Cube(Vec<Expr>),
    /// GROUPING SETS ((a, b), (a), ()).
    GroupingSets(Vec<Vec<Expr>>),
}

// ---------------------------------------------------------------------------
// WINDOW clause (named windows)
// ---------------------------------------------------------------------------

/// Named window definition in the WINDOW clause.
#[derive(Debug, Clone)]
pub struct WindowNameDef {
    pub name: String,
    /// Optional base window name for inheritance: `WINDOW w2 AS (w1 ORDER BY y)`.
    pub base_window: Option<String>,
    pub partition_by: Option<Vec<Expr>>,
    pub order_by: Option<Vec<OrderByDef>>,
    pub frame: Option<super::expr::WindowFrameDef>,
}

// ---------------------------------------------------------------------------
// LIMIT / OFFSET / FETCH / TOP
// ---------------------------------------------------------------------------

/// Pagination definition.
#[derive(Debug, Clone)]
pub struct LimitDef {
    pub kind: LimitKind,
    pub offset: Option<u64>,
}

impl LimitDef {
    pub fn limit(count: u64) -> Self {
        Self {
            kind: LimitKind::Limit(count),
            offset: None,
        }
    }

    pub fn limit_offset(count: u64, offset: u64) -> Self {
        Self {
            kind: LimitKind::Limit(count),
            offset: Some(offset),
        }
    }

    pub fn fetch_first(count: u64) -> Self {
        Self {
            kind: LimitKind::FetchFirst {
                count,
                with_ties: false,
                percent: false,
            },
            offset: None,
        }
    }

    pub fn fetch_first_with_ties(count: u64) -> Self {
        Self {
            kind: LimitKind::FetchFirst {
                count,
                with_ties: true,
                percent: false,
            },
            offset: None,
        }
    }

    pub fn top(count: u64) -> Self {
        Self {
            kind: LimitKind::Top {
                count,
                with_ties: false,
                percent: false,
            },
            offset: None,
        }
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// The type of row limiting.
#[derive(Debug, Clone)]
pub enum LimitKind {
    /// LIMIT n (PG, SQLite, MySQL).
    Limit(u64),
    /// FETCH FIRST n ROWS { ONLY | WITH TIES } (PG, Oracle, SQL Server).
    FetchFirst {
        count: u64,
        with_ties: bool,
        /// Oracle: FETCH FIRST n PERCENT ROWS.
        percent: bool,
    },
    /// SQL Server: TOP(n) [PERCENT] [WITH TIES].
    Top {
        count: u64,
        with_ties: bool,
        percent: bool,
    },
}

// ---------------------------------------------------------------------------
// Common Table Expressions (WITH clause)
// ---------------------------------------------------------------------------

/// Common Table Expression (WITH clause).
#[derive(Debug, Clone)]
pub struct CteDef {
    pub name: String,
    pub query: Box<QueryStmt>,
    pub recursive: bool,
    /// Explicit column names: `WITH cte(a, b) AS (...)`.
    pub column_names: Option<Vec<String>>,
    /// PG: MATERIALIZED / NOT MATERIALIZED hint.
    pub materialized: Option<CteMaterialized>,
}

impl CteDef {
    pub fn new(name: impl Into<String>, query: QueryStmt) -> Self {
        Self {
            name: name.into(),
            query: Box::new(query),
            recursive: false,
            column_names: None,
            materialized: None,
        }
    }

    pub fn recursive(name: impl Into<String>, query: QueryStmt) -> Self {
        Self {
            name: name.into(),
            query: Box::new(query),
            recursive: true,
            column_names: None,
            materialized: None,
        }
    }

    pub fn columns(mut self, cols: Vec<&str>) -> Self {
        self.column_names = Some(cols.into_iter().map(String::from).collect());
        self
    }

    pub fn materialized(mut self) -> Self {
        self.materialized = Some(CteMaterialized::Materialized);
        self
    }

    pub fn not_materialized(mut self) -> Self {
        self.materialized = Some(CteMaterialized::NotMaterialized);
        self
    }
}

/// CTE materialization hint (PostgreSQL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CteMaterialized {
    Materialized,
    NotMaterialized,
}

// ---------------------------------------------------------------------------
// SELECT ... FOR UPDATE / SHARE (row locking)
// ---------------------------------------------------------------------------

/// SELECT ... FOR UPDATE / SHARE.
#[derive(Debug, Clone)]
pub struct SelectLockDef {
    pub strength: LockStrength,
    pub of: Option<Vec<SchemaRef>>,
    pub nowait: bool,
    pub skip_locked: bool,
    /// Oracle: FOR UPDATE WAIT N seconds.
    pub wait: Option<u64>,
}

/// Lock strength.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStrength {
    Update,
    /// PG: FOR NO KEY UPDATE.
    NoKeyUpdate,
    Share,
    /// PG: FOR KEY SHARE.
    KeyShare,
}
