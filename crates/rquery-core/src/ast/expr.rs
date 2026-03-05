use super::common::{FieldRef, OrderByDef};
use super::conditions::Conditions;
use super::custom::CustomExpr;
use super::query::QueryStmt;
use super::value::Value;

/// An expression in a SQL statement.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal value.
    Value(Value),

    /// Column reference.
    Field(FieldRef),

    /// Binary operation: `left op right`.
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    /// Unary operation: `-expr`, `NOT expr`.
    Unary { op: UnaryOp, expr: Box<Expr> },

    /// Function call: `name(args...)`.
    Func { name: String, args: Vec<Expr> },

    /// Aggregate function: `COUNT(expr)`, `SUM(DISTINCT expr) FILTER (WHERE ...)`.
    Aggregate(AggregationDef),

    /// Type cast: `expr::type` (PG) or `CAST(expr AS type)`.
    Cast {
        expr: Box<Expr>,
        to_type: String,
    },

    /// CASE WHEN ... THEN ... ELSE ... END.
    Case(CaseDef),

    /// Window function: `expr OVER (PARTITION BY ... ORDER BY ... frame)`.
    Window(WindowDef),

    /// EXISTS (subquery).
    Exists(Box<QueryStmt>),

    /// Scalar subquery.
    SubQuery(Box<QueryStmt>),

    /// ARRAY(subquery).
    ArraySubQuery(Box<QueryStmt>),

    /// Raw SQL with parameters (escape hatch).
    Raw { sql: String, params: Vec<Value> },

    /// User-defined expression (extension point).
    Custom(Box<dyn CustomExpr>),
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitwiseAnd,
    BitwiseOr,
    ShiftLeft,
    ShiftRight,
    Concat,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitwiseNot,
}

/// Aggregate function definition.
#[derive(Debug, Clone)]
pub struct AggregationDef {
    pub name: String,
    pub expression: Option<Box<Expr>>,
    pub distinct: bool,
    pub filter: Option<Conditions>,
    pub args: Option<Vec<Expr>>,
    pub order_by: Option<Vec<OrderByDef>>,
}

/// CASE expression.
#[derive(Debug, Clone)]
pub struct CaseDef {
    pub cases: Vec<WhenClause>,
    pub default: Option<Box<Expr>>,
}

/// WHEN condition THEN result.
#[derive(Debug, Clone)]
pub struct WhenClause {
    pub condition: Conditions,
    pub result: Expr,
}

/// Window function definition.
#[derive(Debug, Clone)]
pub struct WindowDef {
    pub expression: Box<Expr>,
    pub partition_by: Option<Vec<Expr>>,
    pub order_by: Option<Vec<OrderByDef>>,
    pub frame: Option<WindowFrameDef>,
}

/// Window frame specification.
#[derive(Debug, Clone)]
pub struct WindowFrameDef {
    pub frame_type: WindowFrameType,
    pub start: WindowFrameBound,
    pub end: Option<WindowFrameBound>,
}

/// Window frame type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFrameType {
    Rows,
    Range,
    Groups,
}

/// Window frame bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowFrameBound {
    CurrentRow,
    Preceding(Option<u64>),
    Following(Option<u64>),
}
