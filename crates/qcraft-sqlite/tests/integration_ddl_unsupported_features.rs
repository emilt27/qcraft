//! Tests that verify SQLite renderer correctly rejects unsupported DDL features
//! with meaningful errors (not silent failures).

use qcraft_core::ast::common::SchemaRef;
use qcraft_core::ast::conditions::{Conditions, Connector};
use qcraft_core::ast::ddl::*;
use qcraft_core::ast::expr::Expr;
use qcraft_core::ast::value::Value;
use qcraft_sqlite::SqliteRenderer;

fn render_err(stmt: &SchemaMutationStmt) -> String {
    let renderer = SqliteRenderer::new();
    renderer.render_schema_stmt(stmt).unwrap_err().to_string()
}

fn create_table(schema: SchemaDef) -> SchemaMutationStmt {
    SchemaMutationStmt::CreateTable {
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

// ==========================================================================
// Column types — unsupported
// ==========================================================================

#[test]
fn array_type_errors() {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new(
        "tags",
        FieldType::Array(Box::new(FieldType::scalar("TEXT"))),
    )];
    let err = render_err(&create_table(schema));
    assert!(
        err.to_lowercase().contains("array"),
        "expected array error, got: {err}"
    );
}

#[test]
fn identity_column_errors() {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef {
        name: "id".into(),
        field_type: FieldType::scalar("INTEGER"),
        not_null: true,
        default: None,
        generated: None,
        identity: Some(IdentityColumn::default()),
        collation: None,
        comment: None,
        storage: None,
        compression: None,
    }];
    let err = render_err(&create_table(schema));
    assert!(
        err.contains("identity") || err.contains("Identity"),
        "expected identity error, got: {err}"
    );
}

// ==========================================================================
// Constraints — unsupported
// ==========================================================================

#[test]
fn exclusion_constraint_errors() {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    schema.constraints = Some(vec![ConstraintDef::Exclusion {
        name: None,
        elements: vec![],
        index_method: "gist".into(),
        condition: None,
    }]);
    let err = render_err(&create_table(schema));
    assert!(
        err.contains("EXCLUDE"),
        "expected EXCLUDE error, got: {err}"
    );
}

// ==========================================================================
// ALTER TABLE — unsupported operations
// ==========================================================================

#[test]
fn alter_column_type_errors() {
    let stmt = SchemaMutationStmt::AlterColumnType {
        schema_ref: SchemaRef::new("t"),
        column_name: "x".into(),
        new_type: FieldType::scalar("TEXT"),
        using_expr: None,
    };
    let err = render_err(&stmt);
    assert!(err.contains("ALTER COLUMN TYPE"), "got: {err}");
}

#[test]
fn alter_column_set_default_errors() {
    let stmt = SchemaMutationStmt::AlterColumnDefault {
        schema_ref: SchemaRef::new("t"),
        column_name: "x".into(),
        default: Some(Expr::Value(Value::Int(0))),
    };
    let err = render_err(&stmt);
    assert!(err.contains("ALTER COLUMN DEFAULT"), "got: {err}");
}

#[test]
fn alter_column_drop_default_errors() {
    let stmt = SchemaMutationStmt::AlterColumnDefault {
        schema_ref: SchemaRef::new("t"),
        column_name: "x".into(),
        default: None,
    };
    let err = render_err(&stmt);
    assert!(err.contains("ALTER COLUMN DEFAULT"), "got: {err}");
}

#[test]
fn alter_column_set_not_null_errors() {
    let stmt = SchemaMutationStmt::AlterColumnNullability {
        schema_ref: SchemaRef::new("t"),
        column_name: "x".into(),
        not_null: true,
    };
    let err = render_err(&stmt);
    assert!(err.contains("ALTER COLUMN NOT NULL"), "got: {err}");
}

#[test]
fn alter_column_drop_not_null_errors() {
    let stmt = SchemaMutationStmt::AlterColumnNullability {
        schema_ref: SchemaRef::new("t"),
        column_name: "x".into(),
        not_null: false,
    };
    let err = render_err(&stmt);
    assert!(err.contains("ALTER COLUMN NOT NULL"), "got: {err}");
}

#[test]
fn add_constraint_errors() {
    let stmt = SchemaMutationStmt::AddConstraint {
        schema_ref: SchemaRef::new("t"),
        constraint: ConstraintDef::Check {
            name: None,
            condition: Conditions {
                children: vec![],
                connector: Connector::And,
                negated: false,
            },
            no_inherit: false,
            enforced: None,
        },
        not_valid: false,
    };
    let err = render_err(&stmt);
    assert!(err.contains("ADD CONSTRAINT"), "got: {err}");
}

#[test]
fn drop_constraint_errors() {
    let stmt = SchemaMutationStmt::DropConstraint {
        schema_ref: SchemaRef::new("t"),
        constraint_name: "ck_1".into(),
        if_exists: false,
        cascade: false,
    };
    let err = render_err(&stmt);
    assert!(err.contains("DROP CONSTRAINT"), "got: {err}");
}

#[test]
fn validate_constraint_errors() {
    let stmt = SchemaMutationStmt::ValidateConstraint {
        schema_ref: SchemaRef::new("t"),
        constraint_name: "ck_1".into(),
    };
    let err = render_err(&stmt);
    assert!(err.contains("VALIDATE CONSTRAINT"), "got: {err}");
}

#[test]
fn rename_constraint_errors() {
    let stmt = SchemaMutationStmt::RenameConstraint {
        schema_ref: SchemaRef::new("t"),
        old_name: "old".into(),
        new_name: "new".into(),
    };
    let err = render_err(&stmt);
    // Should error — SQLite doesn't support RENAME CONSTRAINT
    assert!(
        !err.is_empty(),
        "expected error for RENAME CONSTRAINT, got empty"
    );
}

// ==========================================================================
// Extensions — unsupported
// ==========================================================================

#[test]
fn create_extension_errors() {
    let stmt = SchemaMutationStmt::CreateExtension {
        name: "uuid-ossp".into(),
        if_not_exists: true,
        schema: None,
        version: None,
        cascade: false,
    };
    let err = render_err(&stmt);
    assert!(err.to_lowercase().contains("extension"), "got: {err}");
}

#[test]
fn drop_extension_errors() {
    let stmt = SchemaMutationStmt::DropExtension {
        name: "pg_trgm".into(),
        if_exists: true,
        cascade: false,
    };
    let err = render_err(&stmt);
    assert!(err.to_lowercase().contains("extension"), "got: {err}");
}

// ==========================================================================
// TRUNCATE — options ignored (not errors, but verify it still works)
// ==========================================================================

#[test]
fn truncate_restart_identity_ignored() {
    // restart_identity is PG-specific, SQLite renders as DELETE FROM
    let renderer = SqliteRenderer::new();
    let stmt = SchemaMutationStmt::TruncateTable {
        schema_ref: SchemaRef::new("t"),
        restart_identity: true,
        cascade: true,
    };
    let (sql, _) = renderer.render_schema_stmt(&stmt).unwrap();
    assert_eq!(sql, r#"DELETE FROM "t""#);
}
