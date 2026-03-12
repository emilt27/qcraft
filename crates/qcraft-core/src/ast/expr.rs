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
    Cast { expr: Box<Expr>, to_type: String },

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

    /// Collation override: `expr COLLATE "name"`.
    Collate { expr: Box<Expr>, collation: String },

    /// Raw SQL with parameters (escape hatch).
    Raw { sql: String, params: Vec<Value> },

    /// User-defined expression (extension point).
    Custom(Box<dyn CustomExpr>),
}

impl Expr {
    /// Column reference: `table.field`.
    pub fn field(table: &str, name: &str) -> Self {
        Expr::Field(FieldRef::new(table, name))
    }

    /// Literal value.
    pub fn value(val: impl Into<Value>) -> Self {
        Expr::Value(val.into())
    }

    /// Raw SQL expression (no parameters).
    pub fn raw(sql: impl Into<String>) -> Self {
        Expr::Raw {
            sql: sql.into(),
            params: vec![],
        }
    }

    /// Function call: `name(args...)`.
    pub fn func(name: impl Into<String>, args: Vec<Expr>) -> Self {
        Expr::Func {
            name: name.into(),
            args,
        }
    }

    /// Type cast: `CAST(expr AS to_type)`.
    pub fn cast(expr: Expr, to_type: impl Into<String>) -> Self {
        Expr::Cast {
            expr: Box::new(expr),
            to_type: to_type.into(),
        }
    }

    /// COUNT(expr).
    pub fn count(expr: Expr) -> Self {
        Expr::Aggregate(AggregationDef {
            name: "COUNT".into(),
            expression: Some(Box::new(expr)),
            distinct: false,
            filter: None,
            args: None,
            order_by: None,
        })
    }

    /// COUNT(*).
    pub fn count_all() -> Self {
        Expr::Aggregate(AggregationDef {
            name: "COUNT".into(),
            expression: None,
            distinct: false,
            filter: None,
            args: None,
            order_by: None,
        })
    }

    /// SUM(expr).
    pub fn sum(expr: Expr) -> Self {
        Expr::Aggregate(AggregationDef {
            name: "SUM".into(),
            expression: Some(Box::new(expr)),
            distinct: false,
            filter: None,
            args: None,
            order_by: None,
        })
    }

    /// AVG(expr).
    pub fn avg(expr: Expr) -> Self {
        Expr::Aggregate(AggregationDef {
            name: "AVG".into(),
            expression: Some(Box::new(expr)),
            distinct: false,
            filter: None,
            args: None,
            order_by: None,
        })
    }

    /// MIN(expr).
    pub fn min(expr: Expr) -> Self {
        Expr::Aggregate(AggregationDef {
            name: "MIN".into(),
            expression: Some(Box::new(expr)),
            distinct: false,
            filter: None,
            args: None,
            order_by: None,
        })
    }

    /// MAX(expr).
    pub fn max(expr: Expr) -> Self {
        Expr::Aggregate(AggregationDef {
            name: "MAX".into(),
            expression: Some(Box::new(expr)),
            distinct: false,
            filter: None,
            args: None,
            order_by: None,
        })
    }

    /// EXISTS (subquery).
    pub fn exists(query: QueryStmt) -> Self {
        Expr::Exists(Box::new(query))
    }

    /// Scalar subquery.
    pub fn subquery(query: QueryStmt) -> Self {
        Expr::SubQuery(Box::new(query))
    }

    /// Collation override: `expr COLLATE "name"`.
    pub fn collate(self, collation: impl Into<String>) -> Self {
        Expr::Collate {
            expr: Box::new(self),
            collation: collation.into(),
        }
    }
}

impl From<Value> for Expr {
    fn from(v: Value) -> Self {
        Expr::Value(v)
    }
}

impl From<FieldRef> for Expr {
    fn from(f: FieldRef) -> Self {
        Expr::Field(f)
    }
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

impl AggregationDef {
    pub fn new(name: impl Into<String>, expr: Expr) -> Self {
        Self {
            name: name.into(),
            expression: Some(Box::new(expr)),
            distinct: false,
            filter: None,
            args: None,
            order_by: None,
        }
    }

    pub fn count_all() -> Self {
        Self {
            name: "COUNT".into(),
            expression: None,
            distinct: false,
            filter: None,
            args: None,
            order_by: None,
        }
    }

    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    pub fn filter(mut self, cond: Conditions) -> Self {
        self.filter = Some(cond);
        self
    }

    pub fn order_by(mut self, order: Vec<OrderByDef>) -> Self {
        self.order_by = Some(order);
        self
    }
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
