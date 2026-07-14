use super::common::{FieldRef, OrderByDef};
use super::conditions::Conditions;
use super::custom::{CustomBinaryOp, CustomExpr};
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

    /// Explicit grouping: `(expr)`.
    ///
    /// Operator operands are bracketed automatically (see [`Expr::needs_operand_parens`]),
    /// so this is only needed to group an opaque `Raw`/`Custom` expression or to force
    /// brackets for readability.
    Paren(Box<Expr>),

    /// Build a JSON array: PG `jsonb_build_array(...)`, SQLite `json_array(...)`.
    JsonArray(Vec<Expr>),

    /// Build a JSON object: PG `jsonb_build_object(k, v, ...)`, SQLite `json_object(k, v, ...)`.
    JsonObject(Vec<(String, Expr)>),

    /// Aggregate into JSON array: PG `jsonb_agg(...)`, SQLite `json_group_array(...)`.
    JsonAgg {
        expr: Box<Expr>,
        distinct: bool,
        filter: Option<Conditions>,
        order_by: Option<Vec<OrderByDef>>,
    },

    /// Concatenate strings: PG `string_agg(expr, delim)`, SQLite `group_concat(expr, delim)`.
    StringAgg {
        expr: Box<Expr>,
        delimiter: String,
        distinct: bool,
        filter: Option<Conditions>,
        order_by: Option<Vec<OrderByDef>>,
    },

    /// JSON text extraction: `expr->>'path'` on both PG and SQLite.
    /// Unlike `->` (which returns JSON), `->>` returns the value as text.
    JsonPathText { expr: Box<Expr>, path: String },

    /// Current timestamp: PG `now()`, SQLite `datetime('now')`.
    Now,

    /// SQL CURRENT_TIMESTAMP keyword (rendered without parentheses).
    CurrentTimestamp,

    /// SQL CURRENT_DATE keyword (rendered without parentheses).
    CurrentDate,

    /// SQL CURRENT_TIME keyword (rendered without parentheses).
    CurrentTime,

    /// Row/tuple constructor: `(expr1, expr2, ...)`.
    Tuple(Vec<Expr>),

    /// Unbound parameter placeholder for executemany/batch operations.
    /// Renders as `$N` (PG) or `?` (SQLite) without a concrete value.
    /// Optional `type_hint` renders as `$N::type` in PG.
    Param { type_hint: Option<String> },

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

    /// Build a JSON array from expressions.
    pub fn json_array(items: Vec<Expr>) -> Self {
        Expr::JsonArray(items)
    }

    /// Build a JSON object from key-value pairs.
    pub fn json_object(pairs: Vec<(impl Into<String>, Expr)>) -> Self {
        Expr::JsonObject(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// Aggregate values into a JSON array.
    pub fn json_agg(expr: Expr) -> Self {
        Expr::JsonAgg {
            expr: Box::new(expr),
            distinct: false,
            filter: None,
            order_by: None,
        }
    }

    /// Concatenate strings with a delimiter.
    pub fn string_agg(expr: Expr, delimiter: impl Into<String>) -> Self {
        Expr::StringAgg {
            expr: Box::new(expr),
            delimiter: delimiter.into(),
            distinct: false,
            filter: None,
            order_by: None,
        }
    }

    /// JSON text extraction: `expr->>'path'`.
    pub fn json_path_text(expr: Expr, path: impl Into<String>) -> Self {
        Expr::JsonPathText {
            expr: Box::new(expr),
            path: path.into(),
        }
    }

    /// Current timestamp.
    pub fn now() -> Self {
        Expr::Now
    }

    /// Explicit grouping: `(expr)`.
    pub fn paren(expr: Expr) -> Self {
        Expr::Paren(Box::new(expr))
    }

    /// True if this expression must be parenthesized when it appears as the operand
    /// of an operator (`+`, `::`, `COLLATE`, `->>`, a comparison, …).
    ///
    /// An operator expression carries its grouping in the tree, but SQL text carries
    /// it in brackets: printed flat, `Binary(Binary(1, +, 2), *, 3)` becomes
    /// `1 + 2 * 3`, which every engine reads back as `Binary(1, +, Binary(2, *, 3))`
    /// — 7 instead of 9. Bracketing is structural rather than driven by a precedence
    /// table because precedence is dialect-specific: SQLite binds `||` tighter than
    /// `*`, PostgreSQL binds it looser than `+`.
    ///
    /// A [`Expr::Field`] whose [`FieldDef`](crate::ast::common::FieldDef) carries a
    /// `child` renders as a JSON path chain (`"data"->'age'`) — an operator expression
    /// like [`Expr::JsonPathText`], and bracketed for the same reason. A plain field
    /// is a bare identifier and is not.
    ///
    /// Self-delimiting forms (literals, identifiers, function calls, `CAST(…)`,
    /// `CASE … END`, subqueries, tuples, [`Expr::Paren`]) carry their own boundaries
    /// and render bare. `Raw` and `Custom` are opaque escape hatches whose contents
    /// need not be an expression at all, so they are never bracketed automatically —
    /// wrap them in [`Expr::Paren`] when they need grouping.
    pub fn needs_operand_parens(&self) -> bool {
        match self {
            Expr::Binary { .. }
            | Expr::Unary { .. }
            | Expr::Collate { .. }
            | Expr::JsonPathText { .. }
            | Expr::Window(_) => true,
            Expr::Field(field_ref) => field_ref.field.child.is_some(),
            _ => false,
        }
    }

    /// True if this expression tree contains an unbound `Expr::Param` placeholder.
    /// Used to reject double-render forms that would corrupt positional binding.
    /// Does not descend into subquery `QueryStmt`s (those are rejected separately).
    /// Note: `Raw` and `Custom` are opaque escape hatches — a placeholder hidden inside
    /// `Expr::Raw` SQL or a `CustomExpr`/`CustomCondition` cannot be detected here, so
    /// callers using those in a SQLite XOR operand are responsible for double-render safety.
    pub fn contains_unbound_param(&self) -> bool {
        match self {
            Expr::Param { .. } => true,
            Expr::Binary { left, right, .. } => {
                left.contains_unbound_param() || right.contains_unbound_param()
            }
            Expr::Unary { expr, .. }
            | Expr::Cast { expr, .. }
            | Expr::Collate { expr, .. }
            | Expr::JsonPathText { expr, .. } => expr.contains_unbound_param(),
            Expr::Paren(expr) => expr.contains_unbound_param(),
            Expr::Func { args, .. } | Expr::Tuple(args) | Expr::JsonArray(args) => {
                args.iter().any(|a| a.contains_unbound_param())
            }
            Expr::JsonObject(pairs) => pairs.iter().any(|(_, v)| v.contains_unbound_param()),
            Expr::Aggregate(agg) => {
                agg.expression
                    .as_ref()
                    .is_some_and(|e| e.contains_unbound_param())
                    || agg
                        .args
                        .as_ref()
                        .is_some_and(|a| a.iter().any(|e| e.contains_unbound_param()))
                    || agg
                        .filter
                        .as_ref()
                        .is_some_and(|f| f.contains_unbound_param())
                    || agg
                        .order_by
                        .as_ref()
                        .is_some_and(|obs| obs.iter().any(|o| o.expr.contains_unbound_param()))
            }
            Expr::Window(w) => {
                w.expression.contains_unbound_param()
                    || w.partition_by
                        .as_ref()
                        .is_some_and(|ps| ps.iter().any(|e| e.contains_unbound_param()))
                    || w.order_by
                        .as_ref()
                        .is_some_and(|obs| obs.iter().any(|o| o.expr.contains_unbound_param()))
            }
            Expr::JsonAgg {
                expr,
                filter,
                order_by,
                ..
            }
            | Expr::StringAgg {
                expr,
                filter,
                order_by,
                ..
            } => {
                expr.contains_unbound_param()
                    || filter.as_ref().is_some_and(|f| f.contains_unbound_param())
                    || order_by
                        .as_ref()
                        .is_some_and(|obs| obs.iter().any(|o| o.expr.contains_unbound_param()))
            }
            Expr::Case(c) => {
                c.cases.iter().any(|w| {
                    w.condition.contains_unbound_param() || w.result.contains_unbound_param()
                }) || c
                    .default
                    .as_ref()
                    .is_some_and(|d| d.contains_unbound_param())
            }
            _ => false,
        }
    }

    /// True if this expression tree contains a subquery
    /// (`Exists`/`SubQuery`/`ArraySubQuery`), including nested inside other exprs.
    /// Used to reject SQLite XOR operands that would be executed twice.
    /// Note: `Raw` and `Custom` are opaque escape hatches — a subquery hidden inside
    /// `Expr::Raw` SQL or a `CustomExpr`/`CustomCondition` cannot be detected here, so
    /// callers using those in a SQLite XOR operand are responsible for double-execution safety.
    pub fn contains_subquery(&self) -> bool {
        match self {
            Expr::Exists(_) | Expr::SubQuery(_) | Expr::ArraySubQuery(_) => true,
            Expr::Binary { left, right, .. } => {
                left.contains_subquery() || right.contains_subquery()
            }
            Expr::Unary { expr, .. }
            | Expr::Cast { expr, .. }
            | Expr::Collate { expr, .. }
            | Expr::JsonPathText { expr, .. } => expr.contains_subquery(),
            Expr::Paren(expr) => expr.contains_subquery(),
            Expr::Func { args, .. } | Expr::Tuple(args) | Expr::JsonArray(args) => {
                args.iter().any(|a| a.contains_subquery())
            }
            Expr::JsonObject(pairs) => pairs.iter().any(|(_, v)| v.contains_subquery()),
            Expr::Aggregate(agg) => {
                agg.expression
                    .as_ref()
                    .is_some_and(|e| e.contains_subquery())
                    || agg
                        .args
                        .as_ref()
                        .is_some_and(|a| a.iter().any(|e| e.contains_subquery()))
                    || agg.filter.as_ref().is_some_and(|f| f.contains_subquery())
                    || agg
                        .order_by
                        .as_ref()
                        .is_some_and(|obs| obs.iter().any(|o| o.expr.contains_subquery()))
            }
            Expr::Window(w) => {
                w.expression.contains_subquery()
                    || w.partition_by
                        .as_ref()
                        .is_some_and(|ps| ps.iter().any(|e| e.contains_subquery()))
                    || w.order_by
                        .as_ref()
                        .is_some_and(|obs| obs.iter().any(|o| o.expr.contains_subquery()))
            }
            Expr::JsonAgg {
                expr,
                filter,
                order_by,
                ..
            }
            | Expr::StringAgg {
                expr,
                filter,
                order_by,
                ..
            } => {
                expr.contains_subquery()
                    || filter.as_ref().is_some_and(|f| f.contains_subquery())
                    || order_by
                        .as_ref()
                        .is_some_and(|obs| obs.iter().any(|o| o.expr.contains_subquery()))
            }
            Expr::Case(c) => {
                c.cases
                    .iter()
                    .any(|w| w.condition.contains_subquery() || w.result.contains_subquery())
                    || c.default.as_ref().is_some_and(|d| d.contains_subquery())
            }
            _ => false,
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
#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Power,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    Concat,

    /// User-defined binary operator (extension point).
    Custom(Box<dyn CustomBinaryOp>),
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

#[cfg(test)]
mod predicate_tests {
    use super::*;
    use crate::ast::conditions::Conditions;
    use crate::ast::query::QueryStmt;

    fn param() -> Expr {
        Expr::Param { type_hint: None }
    }

    #[test]
    fn contains_unbound_param_detects_nested_param() {
        let e = Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(param()),
                op: BinaryOp::Add,
                right: Box::new(Expr::Value(Value::Int(1))),
            }),
            op: BinaryOp::Mul,
            right: Box::new(Expr::Value(Value::Int(2))),
        };
        assert!(e.contains_unbound_param());
    }

    #[test]
    fn contains_unbound_param_false_for_plain_values() {
        let e = Expr::Binary {
            left: Box::new(Expr::field("t", "a")),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Int(1))),
        };
        assert!(!e.contains_unbound_param());
    }

    #[test]
    fn contains_subquery_detects_nested_subquery() {
        let sub = Expr::SubQuery(Box::<QueryStmt>::default());
        let e = Expr::Binary {
            left: Box::new(sub),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Int(1))),
        };
        assert!(e.contains_subquery());
    }

    #[test]
    fn contains_subquery_false_for_plain_expr() {
        let e = Expr::Binary {
            left: Box::new(Expr::field("t", "a")),
            op: BinaryOp::BitwiseXor,
            right: Box::new(Expr::Value(Value::Int(1))),
        };
        assert!(!e.contains_subquery());
    }

    #[test]
    fn contains_unbound_param_detects_param_in_case_condition() {
        let e = Expr::Case(CaseDef {
            cases: vec![WhenClause {
                condition: Conditions::eq(FieldRef::new("t", "a"), param()),
                result: Expr::value(1),
            }],
            default: None,
        });
        assert!(e.contains_unbound_param());
    }

    #[test]
    fn contains_unbound_param_detects_param_in_aggregate_filter() {
        let mut agg = match Expr::sum(Expr::field("t", "a")) {
            Expr::Aggregate(def) => def,
            _ => unreachable!(),
        };
        agg.filter = Some(Conditions::eq(FieldRef::new("t", "a"), param()));
        let e = Expr::Aggregate(agg);
        assert!(e.contains_unbound_param());
    }

    #[test]
    fn contains_subquery_detects_subquery_in_case_condition() {
        let e = Expr::Case(CaseDef {
            cases: vec![WhenClause {
                condition: Conditions::in_subquery(FieldRef::new("t", "a"), QueryStmt::default()),
                result: Expr::value(1),
            }],
            default: None,
        });
        assert!(e.contains_subquery());
    }

    #[test]
    fn predicates_false_for_plain_case() {
        let e = Expr::Case(CaseDef {
            cases: vec![WhenClause {
                condition: Conditions::eq(FieldRef::new("t", "a"), Expr::value(1)),
                result: Expr::value(2),
            }],
            default: Some(Box::new(Expr::value(3))),
        });
        assert!(!e.contains_unbound_param());
        assert!(!e.contains_subquery());
    }
}
