use super::common::FieldRef;
use super::custom::{CustomCompareOp, CustomCondition};
use super::expr::Expr;
use super::query::QueryStmt;
use super::value::Value;

/// A tree of conditions connected by AND/OR.
#[derive(Debug, Clone)]
pub struct Conditions {
    pub children: Vec<ConditionNode>,
    pub connector: Connector,
    pub negated: bool,
}

impl Conditions {
    pub fn and(children: Vec<ConditionNode>) -> Self {
        Self {
            children,
            connector: Connector::And,
            negated: false,
        }
    }

    pub fn or(children: Vec<ConditionNode>) -> Self {
        Self {
            children,
            connector: Connector::Or,
            negated: false,
        }
    }

    pub fn negated(mut self) -> Self {
        self.negated = !self.negated;
        self
    }

    /// `field = expr`
    pub fn eq(field: FieldRef, val: impl Into<Expr>) -> Self {
        Self::comparison(field, CompareOp::Eq, val.into())
    }

    /// `field != expr`
    pub fn neq(field: FieldRef, val: impl Into<Expr>) -> Self {
        Self::comparison(field, CompareOp::Neq, val.into())
    }

    /// `field > expr`
    pub fn gt(field: FieldRef, val: impl Into<Expr>) -> Self {
        Self::comparison(field, CompareOp::Gt, val.into())
    }

    /// `field >= expr`
    pub fn gte(field: FieldRef, val: impl Into<Expr>) -> Self {
        Self::comparison(field, CompareOp::Gte, val.into())
    }

    /// `field < expr`
    pub fn lt(field: FieldRef, val: impl Into<Expr>) -> Self {
        Self::comparison(field, CompareOp::Lt, val.into())
    }

    /// `field <= expr`
    pub fn lte(field: FieldRef, val: impl Into<Expr>) -> Self {
        Self::comparison(field, CompareOp::Lte, val.into())
    }

    /// `field IS NULL`
    pub fn is_null(field: FieldRef) -> Self {
        Self::comparison(field, CompareOp::IsNull, Expr::Value(Value::Bool(true)))
    }

    /// `field IS NOT NULL`
    pub fn is_not_null(field: FieldRef) -> Self {
        Self::comparison(field, CompareOp::IsNull, Expr::Value(Value::Bool(false)))
    }

    /// `field LIKE pattern` (raw — caller provides the full pattern with wildcards)
    pub fn like(field: FieldRef, pattern: &str) -> Self {
        Self::comparison(
            field,
            CompareOp::Like,
            Expr::Value(Value::Str(pattern.to_string())),
        )
    }

    /// `field LIKE '%value%'` — renderer escapes special chars and wraps with `%`.
    pub fn contains(field: FieldRef, val: &str) -> Self {
        Self::comparison(
            field,
            CompareOp::Contains,
            Expr::Value(Value::Str(val.to_string())),
        )
    }

    /// `field LIKE 'value%'` — renderer escapes special chars and appends `%`.
    pub fn starts_with(field: FieldRef, val: &str) -> Self {
        Self::comparison(
            field,
            CompareOp::StartsWith,
            Expr::Value(Value::Str(val.to_string())),
        )
    }

    /// `field LIKE '%value'` — renderer escapes special chars and prepends `%`.
    pub fn ends_with(field: FieldRef, val: &str) -> Self {
        Self::comparison(
            field,
            CompareOp::EndsWith,
            Expr::Value(Value::Str(val.to_string())),
        )
    }

    /// Case-insensitive `field ILIKE '%value%'` (PG) / `LOWER(field) LIKE LOWER('%value%')` (SQLite).
    pub fn icontains(field: FieldRef, val: &str) -> Self {
        Self::comparison(
            field,
            CompareOp::IContains,
            Expr::Value(Value::Str(val.to_string())),
        )
    }

    /// Case-insensitive `field ILIKE 'value%'` (PG) / `LOWER(field) LIKE LOWER('value%')` (SQLite).
    pub fn istarts_with(field: FieldRef, val: &str) -> Self {
        Self::comparison(
            field,
            CompareOp::IStartsWith,
            Expr::Value(Value::Str(val.to_string())),
        )
    }

    /// Case-insensitive `field ILIKE '%value'` (PG) / `LOWER(field) LIKE LOWER('%value')` (SQLite).
    pub fn iends_with(field: FieldRef, val: &str) -> Self {
        Self::comparison(
            field,
            CompareOp::IEndsWith,
            Expr::Value(Value::Str(val.to_string())),
        )
    }

    /// `field IN (subquery)`
    pub fn in_subquery(field: FieldRef, query: QueryStmt) -> Self {
        Self::and(vec![ConditionNode::Comparison(Box::new(Comparison {
            left: Expr::Field(field),
            op: CompareOp::In,
            right: Expr::SubQuery(Box::new(query)),
            negate: false,
        }))])
    }

    /// Combine: `self AND other`.
    pub fn and_also(mut self, other: Conditions) -> Self {
        if self.connector == Connector::And && !self.negated {
            self.children.push(ConditionNode::Group(other));
            self
        } else {
            Self::and(vec![
                ConditionNode::Group(self),
                ConditionNode::Group(other),
            ])
        }
    }

    /// Combine: `self OR other`.
    pub fn or_else(mut self, other: Conditions) -> Self {
        if self.connector == Connector::Or && !self.negated {
            self.children.push(ConditionNode::Group(other));
            self
        } else {
            Self::or(vec![
                ConditionNode::Group(self),
                ConditionNode::Group(other),
            ])
        }
    }

    fn comparison(field: FieldRef, op: CompareOp, right: Expr) -> Self {
        Self::and(vec![ConditionNode::Comparison(Box::new(Comparison {
            left: Expr::Field(field),
            op,
            right,
            negate: false,
        }))])
    }

    /// True if any comparison operand in this condition tree contains an unbound
    /// `Expr::Param`. Does not descend into `Exists`/subquery bodies (same
    /// invariant as `Expr::contains_unbound_param`).
    pub fn contains_unbound_param(&self) -> bool {
        self.children.iter().any(|node| match node {
            ConditionNode::Comparison(c) => {
                c.left.contains_unbound_param() || c.right.contains_unbound_param()
            }
            ConditionNode::Group(g) => g.contains_unbound_param(),
            ConditionNode::Exists(_) | ConditionNode::Custom(_) => false,
        })
    }

    /// True if this condition tree contains a subquery: either an `Exists` node
    /// or a comparison operand that itself contains a subquery.
    pub fn contains_subquery(&self) -> bool {
        self.children.iter().any(|node| match node {
            ConditionNode::Comparison(c) => {
                c.left.contains_subquery() || c.right.contains_subquery()
            }
            ConditionNode::Group(g) => g.contains_subquery(),
            ConditionNode::Exists(_) => true,
            ConditionNode::Custom(_) => false,
        })
    }
}

impl Comparison {
    pub fn new(left: Expr, op: CompareOp, right: Expr) -> Self {
        Self {
            left,
            op,
            right,
            negate: false,
        }
    }
}

/// Logical connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connector {
    And,
    Or,
}

/// A node in the condition tree.
#[derive(Debug, Clone)]
pub enum ConditionNode {
    /// Single comparison.
    Comparison(Box<Comparison>),

    /// Nested group of conditions.
    Group(Conditions),

    /// EXISTS (subquery).
    Exists(Box<QueryStmt>),

    /// User-defined condition (extension point).
    Custom(Box<dyn CustomCondition>),
}

/// A single comparison: `left op right`.
#[derive(Debug, Clone)]
pub struct Comparison {
    pub left: Expr,
    pub op: CompareOp,
    pub right: Expr,
    pub negate: bool,
}

/// Comparison operators.
#[derive(Debug, Clone)]
pub enum CompareOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    Like,
    ILike,

    // High-level LIKE operators (renderer handles escaping + wildcard wrapping)
    Contains,
    StartsWith,
    EndsWith,
    IContains,
    IStartsWith,
    IEndsWith,
    Between,
    IsNull,
    Similar,
    Regex,
    IRegex,

    // PostgreSQL JSONB
    JsonbContains,
    JsonbContainedBy,
    JsonbHasKey,
    JsonbHasAnyKey,
    JsonbHasAllKeys,

    // PostgreSQL Full-Text Search
    FtsMatch,

    // PostgreSQL Trigram
    TrigramSimilar,
    TrigramWordSimilar,
    TrigramStrictWordSimilar,

    // PostgreSQL Range
    RangeContains,
    RangeContainedBy,
    RangeOverlap,
    RangeStrictlyLeft,
    RangeStrictlyRight,
    RangeNotLeft,
    RangeNotRight,
    RangeAdjacent,

    /// User-defined operator (extension point).
    Custom(Box<dyn CustomCompareOp>),
}
