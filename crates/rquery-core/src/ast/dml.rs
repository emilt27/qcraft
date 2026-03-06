use super::common::SchemaRef;
use super::conditions::Conditions;
use super::custom::CustomMutation;
use super::expr::Expr;
use super::query::{CteDef, SelectColumn, TableSource};

/// Data manipulation statements.
#[derive(Debug, Clone)]
pub enum MutationStmt {
    Insert(InsertStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    Custom(Box<dyn CustomMutation>),
}

// ---------------------------------------------------------------------------
// INSERT
// ---------------------------------------------------------------------------

/// INSERT INTO ... VALUES / SELECT / DEFAULT VALUES.
#[derive(Debug, Clone)]
pub struct InsertStmt {
    pub table: SchemaRef,
    pub columns: Option<Vec<String>>,
    pub source: InsertSource,
    /// Multiple ON CONFLICT clauses (SQLite processes in order; last may omit target).
    pub on_conflict: Option<Vec<OnConflictDef>>,
    pub returning: Option<Vec<SelectColumn>>,
    pub ctes: Option<Vec<CteDef>>,
    /// PG: OVERRIDING { SYSTEM | USER } VALUE (for identity columns).
    pub overriding: Option<OverridingKind>,
    /// SQLite: INSERT OR REPLACE / OR IGNORE / OR ABORT / etc.
    pub conflict_resolution: Option<ConflictResolution>,
    /// MySQL/Oracle: PARTITION targeting.
    pub partition: Option<Vec<String>>,
    /// MySQL: IGNORE modifier (downgrades errors to warnings).
    pub ignore: bool,
}

impl Default for InsertStmt {
    fn default() -> Self {
        Self {
            table: SchemaRef::new(""),
            columns: None,
            source: InsertSource::DefaultValues,
            on_conflict: None,
            returning: None,
            ctes: None,
            overriding: None,
            conflict_resolution: None,
            partition: None,
            ignore: false,
        }
    }
}

impl InsertStmt {
    pub fn values(table: &str, columns: Vec<&str>, rows: Vec<Vec<Expr>>) -> Self {
        Self {
            table: SchemaRef::new(table),
            columns: Some(columns.into_iter().map(String::from).collect()),
            source: InsertSource::Values(rows),
            ..Default::default()
        }
    }

    pub fn from_select(table: &str, columns: Vec<&str>, query: super::query::QueryStmt) -> Self {
        Self {
            table: SchemaRef::new(table),
            columns: Some(columns.into_iter().map(String::from).collect()),
            source: InsertSource::Select(Box::new(query)),
            ..Default::default()
        }
    }

    pub fn default_values(table: &str) -> Self {
        Self {
            table: SchemaRef::new(table),
            ..Default::default()
        }
    }

    pub fn returning(mut self, cols: Vec<SelectColumn>) -> Self {
        self.returning = Some(cols);
        self
    }

    pub fn on_conflict(mut self, def: OnConflictDef) -> Self {
        self.on_conflict = Some(vec![def]);
        self
    }
}

/// Source of data for INSERT.
#[derive(Debug, Clone)]
pub enum InsertSource {
    /// VALUES (expr, ...), (expr, ...), ...
    Values(Vec<Vec<Expr>>),
    /// INSERT INTO ... SELECT ...
    Select(Box<super::query::QueryStmt>),
    /// DEFAULT VALUES (PG, SQLite, SQL Server).
    DefaultValues,
}

/// PG identity column override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverridingKind {
    System,
    User,
}

/// SQLite conflict resolution for INSERT/UPDATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Rollback,
    Abort,
    Fail,
    Ignore,
    Replace,
}

// ---------------------------------------------------------------------------
// ON CONFLICT (upsert)
// ---------------------------------------------------------------------------

/// ON CONFLICT clause (PG / SQLite upsert).
#[derive(Debug, Clone)]
pub struct OnConflictDef {
    /// Conflict target. None = catch-all (SQLite last clause).
    pub target: Option<ConflictTarget>,
    pub action: ConflictAction,
}

/// What triggers the conflict.
#[derive(Debug, Clone)]
pub enum ConflictTarget {
    /// ON CONFLICT (col1, col2, ...) [WHERE ...]
    Columns {
        columns: Vec<String>,
        where_clause: Option<Conditions>,
    },
    /// ON CONSTRAINT constraint_name (PG only).
    Constraint(String),
}

/// What to do on conflict.
#[derive(Debug, Clone)]
pub enum ConflictAction {
    DoNothing,
    DoUpdate {
        assignments: Vec<(String, Expr)>,
        where_clause: Option<Conditions>,
    },
}

impl OnConflictDef {
    pub fn do_nothing() -> Self {
        Self {
            target: None,
            action: ConflictAction::DoNothing,
        }
    }

    pub fn do_update(columns: Vec<&str>, assignments: Vec<(&str, Expr)>) -> Self {
        Self {
            target: Some(ConflictTarget::Columns {
                columns: columns.into_iter().map(String::from).collect(),
                where_clause: None,
            }),
            action: ConflictAction::DoUpdate {
                assignments: assignments
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                where_clause: None,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// UPDATE
// ---------------------------------------------------------------------------

/// UPDATE ... SET ... WHERE ...
#[derive(Debug, Clone)]
pub struct UpdateStmt {
    pub table: SchemaRef,
    pub assignments: Vec<(String, Expr)>,
    pub from: Option<Vec<TableSource>>,
    pub where_clause: Option<Conditions>,
    pub returning: Option<Vec<SelectColumn>>,
    pub ctes: Option<Vec<CteDef>>,
    /// SQLite: UPDATE OR REPLACE / OR IGNORE / etc.
    pub conflict_resolution: Option<ConflictResolution>,
    /// SQLite/MySQL: ORDER BY for UPDATE.
    pub order_by: Option<Vec<super::common::OrderByDef>>,
    /// SQLite/MySQL: LIMIT for UPDATE.
    pub limit: Option<u64>,
    /// SQLite: LIMIT ... OFFSET ...
    pub offset: Option<u64>,
    /// PG: UPDATE ONLY table (exclude inherited/child tables).
    pub only: bool,
    /// MySQL/Oracle: PARTITION targeting.
    pub partition: Option<Vec<String>>,
    /// MySQL: IGNORE modifier (UpdateStmt).
    pub ignore: bool,
}

impl Default for UpdateStmt {
    fn default() -> Self {
        Self {
            table: SchemaRef::new(""),
            assignments: vec![],
            from: None,
            where_clause: None,
            returning: None,
            ctes: None,
            conflict_resolution: None,
            order_by: None,
            limit: None,
            offset: None,
            only: false,
            partition: None,
            ignore: false,
        }
    }
}

impl UpdateStmt {
    pub fn new(table: &str, assignments: Vec<(&str, Expr)>) -> Self {
        Self {
            table: SchemaRef::new(table),
            assignments: assignments
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            ..Default::default()
        }
    }

    pub fn where_clause(mut self, cond: Conditions) -> Self {
        self.where_clause = Some(cond);
        self
    }

    pub fn returning(mut self, cols: Vec<SelectColumn>) -> Self {
        self.returning = Some(cols);
        self
    }
}

// ---------------------------------------------------------------------------
// DELETE
// ---------------------------------------------------------------------------

/// DELETE FROM ... WHERE ...
#[derive(Debug, Clone)]
pub struct DeleteStmt {
    pub table: SchemaRef,
    /// PG: USING from_item [, ...]; SQL Server: FROM table_source [, ...]
    pub using: Option<Vec<TableSource>>,
    pub where_clause: Option<Conditions>,
    pub returning: Option<Vec<SelectColumn>>,
    pub ctes: Option<Vec<CteDef>>,
    /// SQLite/MySQL: ORDER BY for DELETE.
    pub order_by: Option<Vec<super::common::OrderByDef>>,
    /// SQLite/MySQL: LIMIT for DELETE.
    pub limit: Option<u64>,
    /// SQLite: LIMIT ... OFFSET ...
    pub offset: Option<u64>,
    /// PG: DELETE FROM ONLY table (exclude inherited/child tables).
    pub only: bool,
    /// MySQL/Oracle: PARTITION targeting.
    pub partition: Option<Vec<String>>,
    /// MySQL: IGNORE modifier.
    pub ignore: bool,
}

impl Default for DeleteStmt {
    fn default() -> Self {
        Self {
            table: SchemaRef::new(""),
            using: None,
            where_clause: None,
            returning: None,
            ctes: None,
            order_by: None,
            limit: None,
            offset: None,
            only: false,
            partition: None,
            ignore: false,
        }
    }
}

impl DeleteStmt {
    pub fn new(table: &str) -> Self {
        Self {
            table: SchemaRef::new(table),
            ..Default::default()
        }
    }

    pub fn where_clause(mut self, cond: Conditions) -> Self {
        self.where_clause = Some(cond);
        self
    }

    pub fn returning(mut self, cols: Vec<SelectColumn>) -> Self {
        self.returning = Some(cols);
        self
    }
}
