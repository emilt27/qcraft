/// What to do when a feature is not supported by the target dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedPolicy {
    /// Silently skip the feature.
    Ignore,
    /// Skip the feature but record a warning.
    Warn,
    /// Return an error and stop rendering.
    Error,
}

/// A warning emitted when a feature is skipped.
#[derive(Debug, Clone)]
pub struct Warning {
    pub feature: &'static str,
    pub message: String,
}

/// Known features that may be unsupported by some dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    // Table-level
    IfNotExists,
    Temporary,
    Unlogged,
    Tablespace,
    Inherits,
    PartitionBy,
    TableComment,

    // Column-level
    ColumnCollation,
    ColumnComment,
    ColumnStorage,
    ColumnCompression,
    GeneratedVirtual,
    GeneratedStored,
    Identity,

    // Constraints
    Deferrable,
    CheckNoInherit,
    CheckEnforced,
    NullsDistinct,
    ExclusionConstraint,
    ForeignKeyOnUpdate,
    ForeignKeyMatchType,
    ConstraintNotValid,
    ValidateConstraint,
    RenameConstraint,

    // Index
    IndexIfNotExists,
    IndexConcurrently,
    IndexOnline,
    IndexType,
    IndexInclude,
    IndexPartialWhere,
    IndexExpressionColumn,
    IndexNullsOrder,
    IndexOperatorClass,
    IndexParameters,
    IndexTablespace,
    IndexInvisible,

    // Alter table
    AlterColumnType,
    AlterColumnDefault,
    AlterColumnNullability,
    AddConstraint,
    DropConstraint,
    ColumnPosition,

    // Drop table
    DropCascade,
    DropRestrict,

    // Drop index
    DropIndexConcurrently,
    DropIndexCascade,

    // Extensions
    CreateExtension,
    DropExtension,
}
