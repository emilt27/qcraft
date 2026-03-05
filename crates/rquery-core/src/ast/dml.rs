use super::common::{FieldRef, SchemaRef};
use super::conditions::Conditions;
use super::custom::CustomMutation;
use super::expr::Expr;
use super::query::{QueryStmt, TableSource};
use super::value::Value;

/// Data manipulation statements.
#[derive(Debug, Clone)]
pub enum MutationStmt {
    Insert(InsertStmt),
    InsertFromSelect(InsertFromSelectStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    Truncate(SchemaRef),
    Custom(Box<dyn CustomMutation>),
}

/// INSERT INTO ... VALUES ...
#[derive(Debug, Clone)]
pub struct InsertStmt {
    pub schema: SchemaRef,
    pub rows: Vec<DataRow>,
    pub on_conflict: Option<OnConflictDef>,
    pub returning: Option<Vec<FieldRef>>,
}

/// A row of data: column name → value pairs.
#[derive(Debug, Clone)]
pub struct DataRow {
    pub data: Vec<(String, Value)>,
}

/// ON CONFLICT / ON DUPLICATE KEY handling.
#[derive(Debug, Clone)]
pub struct OnConflictDef {
    pub fields: Vec<FieldRef>,
    pub action: ConflictAction,
    pub update_fields: Option<Vec<FieldRef>>,
    pub where_clause: Option<Conditions>,
}

/// Conflict resolution action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictAction {
    Nothing,
    Update,
}

/// INSERT INTO ... SELECT ...
#[derive(Debug, Clone)]
pub struct InsertFromSelectStmt {
    pub schema: SchemaRef,
    pub query: QueryStmt,
    pub columns: Option<Vec<FieldRef>>,
    pub returning: Option<Vec<FieldRef>>,
}

/// UPDATE ... SET ... WHERE ...
#[derive(Debug, Clone)]
pub struct UpdateStmt {
    pub schema: SchemaRef,
    pub assignments: Vec<(String, Expr)>,
    pub where_clause: Option<Conditions>,
    pub from_tables: Option<Vec<TableSource>>,
    pub returning: Option<Vec<FieldRef>>,
}

/// DELETE FROM ... WHERE ...
#[derive(Debug, Clone)]
pub struct DeleteStmt {
    pub schema: SchemaRef,
    pub where_clause: Option<Conditions>,
    pub returning: Option<Vec<FieldRef>>,
}
