use super::custom::{CustomCompareOp, CustomCondition};
use super::expr::Expr;
use super::query::QueryStmt;

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
    Comparison(Comparison),

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

    /// User-defined operator (extension point).
    Custom(Box<dyn CustomCompareOp>),
}
