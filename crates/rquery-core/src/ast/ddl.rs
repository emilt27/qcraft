use super::common::{OrderDir, SchemaRef};
use super::conditions::Conditions;
use super::custom::{CustomConstraint, CustomFieldType, CustomSchemaMutation};
use super::expr::Expr;

// ---------------------------------------------------------------------------
// Schema mutation statements (DDL)
// ---------------------------------------------------------------------------

/// All DDL operations.
#[derive(Debug, Clone)]
pub enum SchemaMutationStmt {
    // ── Table operations ──
    CreateTable {
        schema: SchemaDef,
        if_not_exists: bool,
        temporary: bool,
        unlogged: bool,
        tablespace: Option<String>,
    },
    DropTable {
        schema_ref: SchemaRef,
        if_exists: bool,
        cascade: bool,
    },
    RenameTable {
        schema_ref: SchemaRef,
        new_name: String,
    },

    // ── Column operations ──
    AddColumn {
        schema_ref: SchemaRef,
        column: ColumnDef,
        if_not_exists: bool,
        position: Option<ColumnPosition>,
    },
    DropColumn {
        schema_ref: SchemaRef,
        name: String,
        if_exists: bool,
        cascade: bool,
    },
    RenameColumn {
        schema_ref: SchemaRef,
        old_name: String,
        new_name: String,
    },
    AlterColumnType {
        schema_ref: SchemaRef,
        column_name: String,
        new_type: FieldType,
        using_expr: Option<Expr>,
    },
    AlterColumnDefault {
        schema_ref: SchemaRef,
        column_name: String,
        default: Option<Expr>,
    },
    AlterColumnNullability {
        schema_ref: SchemaRef,
        column_name: String,
        not_null: bool,
    },

    // ── Constraint operations ──
    AddConstraint {
        schema_ref: SchemaRef,
        constraint: ConstraintDef,
        not_valid: bool,
    },
    DropConstraint {
        schema_ref: SchemaRef,
        constraint_name: String,
        if_exists: bool,
        cascade: bool,
    },
    RenameConstraint {
        schema_ref: SchemaRef,
        old_name: String,
        new_name: String,
    },
    ValidateConstraint {
        schema_ref: SchemaRef,
        constraint_name: String,
    },

    // ── Index operations ──
    CreateIndex {
        schema_ref: SchemaRef,
        index: IndexDef,
        if_not_exists: bool,
        concurrently: bool,
    },
    DropIndex {
        schema_ref: SchemaRef,
        index_name: String,
        if_exists: bool,
        concurrently: bool,
        cascade: bool,
    },

    // ── Extension operations (PostgreSQL) ──
    CreateExtension {
        name: String,
        if_not_exists: bool,
        schema: Option<String>,
        version: Option<String>,
        cascade: bool,
    },
    DropExtension {
        name: String,
        if_exists: bool,
        cascade: bool,
    },

    /// User-defined DDL operation (extension point).
    Custom(Box<dyn CustomSchemaMutation>),
}

// ---------------------------------------------------------------------------
// Table / Schema definition
// ---------------------------------------------------------------------------

/// Complete table definition (for CREATE TABLE).
#[derive(Debug, Clone)]
pub struct SchemaDef {
    pub name: String,
    pub namespace: Option<String>,
    pub columns: Vec<ColumnDef>,
    pub constraints: Option<Vec<ConstraintDef>>,
    pub indexes: Option<Vec<IndexDef>>,
}

impl SchemaDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            columns: Vec::new(),
            constraints: None,
            indexes: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Column definition
// ---------------------------------------------------------------------------

/// A column in a table.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub field_type: FieldType,
    pub not_null: bool,
    pub default: Option<Expr>,
    pub generated: Option<GeneratedColumn>,
    pub identity: Option<IdentityColumn>,
    pub collation: Option<String>,
    pub comment: Option<String>,
}

impl ColumnDef {
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            not_null: false,
            default: None,
            generated: None,
            identity: None,
            collation: None,
            comment: None,
        }
    }

    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    pub fn default(mut self, expr: Expr) -> Self {
        self.default = Some(expr);
        self
    }
}

/// Column type.
#[derive(Debug, Clone)]
pub enum FieldType {
    /// Well-known scalar type: text, integer, bigint, boolean, float, double,
    /// serial, bigserial, json, jsonb, uuid, timestamp, timestamptz,
    /// bytea, numeric, date, time, interval, etc.
    Scalar(String),

    /// Custom type with optional parameters: `VARCHAR(255)`, `NUMERIC(10,2)`.
    Parameterized {
        name: String,
        params: Vec<String>,
    },

    /// Array type: `INTEGER[]`, `TEXT[]`.
    Array(Box<FieldType>),

    /// Vector type (pgvector): `VECTOR(1536)`.
    Vector(i64),

    /// User-defined type (extension point).
    Custom(Box<dyn CustomFieldType>),
}

impl FieldType {
    pub fn scalar(name: impl Into<String>) -> Self {
        Self::Scalar(name.into())
    }

    pub fn parameterized(name: impl Into<String>, params: Vec<impl Into<String>>) -> Self {
        Self::Parameterized {
            name: name.into(),
            params: params.into_iter().map(Into::into).collect(),
        }
    }
}

/// Generated (computed) column.
#[derive(Debug, Clone)]
pub struct GeneratedColumn {
    pub expr: Expr,
    pub stored: bool,
}

/// Identity (auto-increment) column.
#[derive(Debug, Clone)]
pub struct IdentityColumn {
    pub always: bool,
    pub start: Option<i64>,
    pub increment: Option<i64>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub cycle: bool,
    pub cache: Option<i64>,
}

impl Default for IdentityColumn {
    fn default() -> Self {
        Self {
            always: false,
            start: None,
            increment: None,
            min_value: None,
            max_value: None,
            cycle: false,
            cache: None,
        }
    }
}

/// Column position for ADD COLUMN (MySQL-specific: FIRST / AFTER).
#[derive(Debug, Clone)]
pub enum ColumnPosition {
    First,
    After(String),
}

// ---------------------------------------------------------------------------
// Constraints
// ---------------------------------------------------------------------------

/// Table or column constraint.
#[derive(Debug, Clone)]
pub enum ConstraintDef {
    PrimaryKey {
        name: Option<String>,
        columns: Vec<String>,
        include: Option<Vec<String>>,
    },

    ForeignKey {
        name: Option<String>,
        columns: Vec<String>,
        ref_table: SchemaRef,
        ref_columns: Vec<String>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
        deferrable: Option<DeferrableConstraint>,
        match_type: Option<MatchType>,
    },

    Unique {
        name: Option<String>,
        columns: Vec<String>,
        include: Option<Vec<String>>,
        nulls_distinct: Option<bool>,
        condition: Option<Conditions>,
    },

    Check {
        name: Option<String>,
        condition: Conditions,
        no_inherit: bool,
        enforced: Option<bool>,
    },

    Exclusion {
        name: Option<String>,
        elements: Vec<ExclusionElement>,
        index_method: String,
        condition: Option<Conditions>,
    },

    /// User-defined constraint (extension point).
    Custom(Box<dyn CustomConstraint>),
}

/// Referential action for ON DELETE / ON UPDATE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferentialAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull(Option<Vec<String>>),
    SetDefault(Option<Vec<String>>),
}

/// Deferrable constraint options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeferrableConstraint {
    pub deferrable: bool,
    pub initially_deferred: bool,
}

/// MATCH type for foreign keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Full,
    Partial,
    Simple,
}

/// Element in an EXCLUSION constraint.
#[derive(Debug, Clone)]
pub struct ExclusionElement {
    pub column: String,
    pub operator: String,
    pub opclass: Option<String>,
}

// ---------------------------------------------------------------------------
// Index definition
// ---------------------------------------------------------------------------

/// An index on a table.
#[derive(Debug, Clone)]
pub struct IndexDef {
    pub name: String,
    pub columns: Vec<IndexColumnDef>,
    pub unique: bool,
    pub index_type: Option<String>,
    pub include: Option<Vec<String>>,
    pub condition: Option<Conditions>,
    pub parameters: Option<Vec<(String, String)>>,
    pub tablespace: Option<String>,
    pub nulls_distinct: Option<bool>,
}

impl IndexDef {
    pub fn new(name: impl Into<String>, columns: Vec<IndexColumnDef>) -> Self {
        Self {
            name: name.into(),
            columns,
            unique: false,
            index_type: None,
            include: None,
            condition: None,
            parameters: None,
            tablespace: None,
            nulls_distinct: None,
        }
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
}

/// A column or expression in an index.
#[derive(Debug, Clone)]
pub struct IndexColumnDef {
    pub expr: IndexExpr,
    pub direction: Option<OrderDir>,
    pub nulls: Option<NullsOrder>,
    pub opclass: Option<String>,
    pub collation: Option<String>,
}

/// What's being indexed: a column name or an expression.
#[derive(Debug, Clone)]
pub enum IndexExpr {
    Column(String),
    Expression(Expr),
}

/// NULLS FIRST / NULLS LAST for indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullsOrder {
    First,
    Last,
}
