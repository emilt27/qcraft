use super::common::{NullsOrder, OrderDir, SchemaRef};
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
        partition_by: Option<PartitionByDef>,
        inherits: Option<Vec<SchemaRef>>,
        using_method: Option<String>,
        with_options: Option<Vec<(String, String)>>,
        on_commit: Option<OnCommitAction>,
        /// Generic table options (MySQL ENGINE, ROW_FORMAT, etc.).
        table_options: Option<Vec<(String, String)>>,
        /// SQLite WITHOUT ROWID.
        without_rowid: bool,
        /// SQLite STRICT mode.
        strict: bool,
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
    TruncateTable {
        schema_ref: SchemaRef,
        restart_identity: bool,
        cascade: bool,
    },

    // ── Column operations ──
    AddColumn {
        schema_ref: SchemaRef,
        column: Box<ColumnDef>,
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

impl SchemaMutationStmt {
    pub fn create_table(schema: SchemaDef) -> Self {
        Self::CreateTable {
            schema,
            if_not_exists: false,
            temporary: false,
            unlogged: false,
            tablespace: None,
            partition_by: None,
            inherits: None,
            using_method: None,
            with_options: None,
            on_commit: None,
            table_options: None,
            without_rowid: false,
            strict: false,
        }
    }

    pub fn drop_table(name: &str) -> Self {
        Self::DropTable {
            schema_ref: SchemaRef::new(name),
            if_exists: false,
            cascade: false,
        }
    }

    pub fn drop_table_if_exists(name: &str) -> Self {
        Self::DropTable {
            schema_ref: SchemaRef::new(name),
            if_exists: true,
            cascade: false,
        }
    }

    pub fn create_index(table: &str, index: IndexDef) -> Self {
        Self::CreateIndex {
            schema_ref: SchemaRef::new(table),
            index,
            if_not_exists: false,
            concurrently: false,
        }
    }

    pub fn drop_index(table: &str, name: &str) -> Self {
        Self::DropIndex {
            schema_ref: SchemaRef::new(table),
            index_name: name.to_string(),
            if_exists: false,
            concurrently: false,
            cascade: false,
        }
    }

    pub fn add_column(table: &str, column: ColumnDef) -> Self {
        Self::AddColumn {
            schema_ref: SchemaRef::new(table),
            column: Box::new(column),
            if_not_exists: false,
            position: None,
        }
    }

    pub fn drop_column(table: &str, name: &str) -> Self {
        Self::DropColumn {
            schema_ref: SchemaRef::new(table),
            name: name.to_string(),
            if_exists: false,
            cascade: false,
        }
    }

    pub fn rename_table(old: &str, new_name: &str) -> Self {
        Self::RenameTable {
            schema_ref: SchemaRef::new(old),
            new_name: new_name.to_string(),
        }
    }

    pub fn rename_column(table: &str, old: &str, new_name: &str) -> Self {
        Self::RenameColumn {
            schema_ref: SchemaRef::new(table),
            old_name: old.to_string(),
            new_name: new_name.to_string(),
        }
    }

    pub fn truncate(table: &str) -> Self {
        Self::TruncateTable {
            schema_ref: SchemaRef::new(table),
            restart_identity: false,
            cascade: false,
        }
    }
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
    pub like_tables: Option<Vec<LikeTableDef>>,
}

impl SchemaDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            columns: Vec::new(),
            constraints: None,
            indexes: None,
            like_tables: None,
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
    pub storage: Option<String>,
    pub compression: Option<String>,
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
            storage: None,
            compression: None,
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

    pub fn generated(mut self, expr: Expr, stored: bool) -> Self {
        self.generated = Some(GeneratedColumn { expr, stored });
        self
    }

    pub fn collation(mut self, name: impl Into<String>) -> Self {
        self.collation = Some(name.into());
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
    Parameterized { name: String, params: Vec<String> },

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
#[derive(Debug, Clone, Default)]
pub struct IdentityColumn {
    pub always: bool,
    pub start: Option<i64>,
    pub increment: Option<i64>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub cycle: bool,
    pub cache: Option<i64>,
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
        /// SQLite AUTOINCREMENT (only valid with single INTEGER PRIMARY KEY).
        autoincrement: bool,
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

impl ConstraintDef {
    pub fn primary_key(columns: Vec<&str>) -> Self {
        Self::PrimaryKey {
            name: None,
            columns: columns.into_iter().map(String::from).collect(),
            include: None,
            autoincrement: false,
        }
    }

    pub fn foreign_key(columns: Vec<&str>, ref_table: &str, ref_columns: Vec<&str>) -> Self {
        Self::ForeignKey {
            name: None,
            columns: columns.into_iter().map(String::from).collect(),
            ref_table: SchemaRef::new(ref_table),
            ref_columns: ref_columns.into_iter().map(String::from).collect(),
            on_delete: None,
            on_update: None,
            deferrable: None,
            match_type: None,
        }
    }

    pub fn unique(columns: Vec<&str>) -> Self {
        Self::Unique {
            name: None,
            columns: columns.into_iter().map(String::from).collect(),
            include: None,
            nulls_distinct: None,
            condition: None,
        }
    }

    pub fn check(condition: Conditions) -> Self {
        Self::Check {
            name: None,
            condition,
            no_inherit: false,
            enforced: None,
        }
    }
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

impl IndexColumnDef {
    pub fn column(name: impl Into<String>) -> Self {
        Self {
            expr: IndexExpr::Column(name.into()),
            direction: None,
            nulls: None,
            opclass: None,
            collation: None,
        }
    }

    pub fn expression(expr: Expr) -> Self {
        Self {
            expr: IndexExpr::Expression(expr),
            direction: None,
            nulls: None,
            opclass: None,
            collation: None,
        }
    }

    pub fn asc(mut self) -> Self {
        self.direction = Some(OrderDir::Asc);
        self
    }

    pub fn desc(mut self) -> Self {
        self.direction = Some(OrderDir::Desc);
        self
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

/// What's being indexed: a column name or an expression.
#[derive(Debug, Clone)]
pub enum IndexExpr {
    Column(String),
    Expression(Expr),
}

// ---------------------------------------------------------------------------
// Partition definition
// ---------------------------------------------------------------------------

/// PARTITION BY clause for CREATE TABLE.
#[derive(Debug, Clone)]
pub struct PartitionByDef {
    pub strategy: PartitionStrategy,
    pub columns: Vec<PartitionColumnDef>,
}

/// Partition strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionStrategy {
    Range,
    List,
    Hash,
}

/// A column or expression in a PARTITION BY clause.
#[derive(Debug, Clone)]
pub struct PartitionColumnDef {
    pub expr: IndexExpr,
    pub collation: Option<String>,
    pub opclass: Option<String>,
}

// ---------------------------------------------------------------------------
// LIKE table definition
// ---------------------------------------------------------------------------

/// LIKE source_table [ like_option ... ] in CREATE TABLE.
#[derive(Debug, Clone)]
pub struct LikeTableDef {
    pub source_table: SchemaRef,
    pub options: Vec<LikeOption>,
}

/// LIKE options: INCLUDING or EXCLUDING specific properties.
#[derive(Debug, Clone)]
pub struct LikeOption {
    pub kind: LikeOptionKind,
    pub include: bool,
}

/// What to include/exclude from the LIKE source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LikeOptionKind {
    Comments,
    Compression,
    Constraints,
    Defaults,
    Generated,
    Identity,
    Indexes,
    Statistics,
    Storage,
    All,
}

// ---------------------------------------------------------------------------
// ON COMMIT action for temporary tables
// ---------------------------------------------------------------------------

/// ON COMMIT action for temporary tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnCommitAction {
    PreserveRows,
    DeleteRows,
    Drop,
}
