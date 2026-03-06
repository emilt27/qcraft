/// Reference to a table (or schema object).
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaRef {
    pub name: String,
    pub alias: Option<String>,
    pub namespace: Option<String>,
}

impl SchemaRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            alias: None,
            namespace: None,
        }
    }

    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }
}

/// A field name, possibly with nested access (e.g. JSON path).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub child: Option<Box<FieldDef>>,
}

impl FieldDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            child: None,
        }
    }
}

/// A field reference: field + table context.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldRef {
    pub field: FieldDef,
    pub table_name: String,
    pub namespace: Option<String>,
}

impl FieldRef {
    pub fn new(table: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            field: FieldDef::new(field),
            table_name: table.into(),
            namespace: None,
        }
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDir {
    Asc,
    Desc,
}

/// NULLS placement in ORDER BY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullsOrder {
    First,
    Last,
}

/// ORDER BY element.
#[derive(Debug, Clone)]
pub struct OrderByDef {
    pub expr: super::expr::Expr,
    pub direction: OrderDir,
    pub nulls: Option<NullsOrder>,
}

impl OrderByDef {
    pub fn asc(expr: super::expr::Expr) -> Self {
        Self {
            expr,
            direction: OrderDir::Asc,
            nulls: None,
        }
    }

    pub fn desc(expr: super::expr::Expr) -> Self {
        Self {
            expr,
            direction: OrderDir::Desc,
            nulls: None,
        }
    }

    pub fn nulls_first(mut self) -> Self {
        self.nulls = Some(NullsOrder::First);
        self
    }

    pub fn nulls_last(mut self) -> Self {
        self.nulls = Some(NullsOrder::Last);
        self
    }
}
