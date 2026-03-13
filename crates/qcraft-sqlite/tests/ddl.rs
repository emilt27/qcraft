use qcraft_core::ast::common::{FieldRef, SchemaRef};
use qcraft_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use qcraft_core::ast::ddl::*;
use qcraft_core::ast::expr::Expr;
use qcraft_core::ast::value::Value;
use qcraft_sqlite::SqliteRenderer;

fn render(stmt: &SchemaMutationStmt) -> String {
    let renderer = SqliteRenderer::new();
    let stmts = renderer.render_schema_stmt(stmt).unwrap();
    stmts[0].0.clone()
}

fn render_err(stmt: &SchemaMutationStmt) -> String {
    let renderer = SqliteRenderer::new();
    renderer.render_schema_stmt(stmt).unwrap_err().to_string()
}

// ==========================================================================
// CREATE TABLE
// ==========================================================================

#[test]
fn create_table_simple() {
    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
    ];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "users" ("id" INTEGER NOT NULL, "name" TEXT)"#
    );
}

#[test]
fn create_table_if_not_exists() {
    let mut schema = SchemaDef::new("users");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: true,
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE IF NOT EXISTS "users" ("id" INTEGER)"#
    );
}

#[test]
fn create_table_temporary() {
    let mut schema = SchemaDef::new("temp_data");
    schema.columns = vec![ColumnDef::new("val", FieldType::scalar("TEXT"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: true,
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TEMP TABLE "temp_data" ("val" TEXT)"#
    );
}

#[test]
fn create_table_unlogged_ignored() {
    // UNLOGGED is silently ignored for SQLite
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: true,
        tablespace: Some("fast_disk".into()),
        partition_by: None,
        inherits: None,
        using_method: None,
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: false,
        strict: false,
    };
    assert_eq!(render(&stmt), r#"CREATE TABLE "t" ("id" INTEGER)"#);
}

#[test]
fn create_table_with_namespace() {
    let mut schema = SchemaDef::new("users");
    schema.namespace = Some("main".into());
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "main"."users" ("id" INTEGER)"#
    );
}

#[test]
fn create_table_with_default() {
    let mut schema = SchemaDef::new("config");
    schema.columns = vec![
        ColumnDef::new("key", FieldType::scalar("TEXT")).not_null(),
        ColumnDef::new("value", FieldType::scalar("TEXT"))
            .default(Expr::Value(Value::Str("default".into()))),
    ];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "config" ("key" TEXT NOT NULL, "value" TEXT DEFAULT 'default')"#,
    );
}

#[test]
fn create_table_with_generated_column() {
    let mut schema = SchemaDef::new("products");
    schema.columns = vec![
        ColumnDef::new("price", FieldType::scalar("REAL")),
        ColumnDef::new("qty", FieldType::scalar("INTEGER")),
        ColumnDef {
            name: "total".into(),
            field_type: FieldType::scalar("REAL"),
            not_null: false,
            default: None,
            generated: Some(GeneratedColumn {
                expr: Expr::Raw {
                    sql: "price * qty".into(),
                    params: vec![],
                },
                stored: false,
            }),
            identity: None,
            collation: None,
            comment: None,
            storage: None,
            compression: None,
        },
    ];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "products" ("price" REAL, "qty" INTEGER, "total" REAL GENERATED ALWAYS AS (price * qty) VIRTUAL)"#,
    );
}

#[test]
fn create_table_generated_stored() {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef {
        name: "full_name".into(),
        field_type: FieldType::scalar("TEXT"),
        not_null: false,
        default: None,
        generated: Some(GeneratedColumn {
            expr: Expr::Raw {
                sql: "first_name || ' ' || last_name".into(),
                params: vec![],
            },
            stored: true,
        }),
        identity: None,
        collation: None,
        comment: None,
        storage: None,
        compression: None,
    }];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "t" ("full_name" TEXT GENERATED ALWAYS AS (first_name || ' ' || last_name) STORED)"#,
    );
}

#[test]
fn generated_column_strips_table_qualifier() {
    let mut schema = SchemaDef::new("products");
    schema.columns = vec![
        ColumnDef::new("price", FieldType::scalar("NUMERIC")),
        ColumnDef {
            name: "total".into(),
            field_type: FieldType::scalar("NUMERIC"),
            not_null: false,
            default: None,
            generated: Some(GeneratedColumn {
                expr: Expr::Field(FieldRef::new("products", "price")),
                stored: false,
            }),
            identity: None,
            collation: None,
            comment: None,
            storage: None,
            compression: None,
        },
    ];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    // SQLite generated columns must use unqualified column names
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "products" ("price" NUMERIC, "total" NUMERIC GENERATED ALWAYS AS ("price") VIRTUAL)"#,
    );
}

#[test]
fn create_table_identity_error() {
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
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("identity") || err.contains("Identity"),
        "expected identity error, got: {err}"
    );
}

#[test]
fn create_table_parameterized_type() {
    let mut schema = SchemaDef::new("data");
    schema.columns = vec![
        ColumnDef::new(
            "amount",
            FieldType::parameterized("DECIMAL", vec!["10", "2"]),
        ),
        ColumnDef::new("code", FieldType::parameterized("VARCHAR", vec!["50"])),
    ];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "data" ("amount" DECIMAL(10, 2), "code" VARCHAR(50))"#,
    );
}

#[test]
fn create_table_primary_key() {
    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
    ];
    schema.constraints = Some(vec![ConstraintDef::PrimaryKey {
        name: None,
        columns: vec!["id".into()],
        include: None,
    }]);
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "users" ("id" INTEGER NOT NULL, "name" TEXT, PRIMARY KEY ("id"))"#,
    );
}

#[test]
fn create_table_foreign_key() {
    let mut schema = SchemaDef::new("posts");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("user_id", FieldType::scalar("INTEGER")),
    ];
    schema.constraints = Some(vec![ConstraintDef::ForeignKey {
        name: Some("fk_user".into()),
        columns: vec!["user_id".into()],
        ref_table: SchemaRef::new("users"),
        ref_columns: vec!["id".into()],
        on_delete: Some(ReferentialAction::Cascade),
        on_update: Some(ReferentialAction::NoAction),
        deferrable: None,
        match_type: None,
    }]);
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "posts" ("id" INTEGER NOT NULL, "user_id" INTEGER, CONSTRAINT "fk_user" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE ON UPDATE NO ACTION)"#,
    );
}

#[test]
fn create_table_unique_check() {
    let mut schema = SchemaDef::new("users");
    schema.columns = vec![
        ColumnDef::new("email", FieldType::scalar("TEXT")).not_null(),
        ColumnDef::new("age", FieldType::scalar("INTEGER")),
    ];
    schema.constraints = Some(vec![
        ConstraintDef::Unique {
            name: None,
            columns: vec!["email".into()],
            include: None,
            nulls_distinct: None,
            condition: None,
        },
        ConstraintDef::Check {
            name: Some("age_positive".into()),
            condition: Conditions {
                children: vec![ConditionNode::Comparison(Box::new(Comparison {
                    left: Expr::Raw {
                        sql: "\"age\"".into(),
                        params: vec![],
                    },
                    op: CompareOp::Gt,
                    right: Expr::Value(Value::Int(0)),
                    negate: false,
                }))],
                connector: Connector::And,
                negated: false,
            },
            no_inherit: false,
            enforced: None,
        },
    ]);
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "users" ("email" TEXT NOT NULL, "age" INTEGER, UNIQUE ("email"), CONSTRAINT "age_positive" CHECK ("age" > 0))"#,
    );
}

#[test]
fn create_table_deferrable_fk() {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("ref_id", FieldType::scalar("INTEGER"))];
    schema.constraints = Some(vec![ConstraintDef::ForeignKey {
        name: None,
        columns: vec!["ref_id".into()],
        ref_table: SchemaRef::new("other"),
        ref_columns: vec!["id".into()],
        on_delete: None,
        on_update: None,
        deferrable: Some(DeferrableConstraint {
            deferrable: true,
            initially_deferred: true,
        }),
        match_type: None,
    }]);
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "t" ("ref_id" INTEGER, FOREIGN KEY ("ref_id") REFERENCES "other" ("id") DEFERRABLE INITIALLY DEFERRED)"#,
    );
}

#[test]
fn create_table_array_type_error() {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new(
        "tags",
        FieldType::Array(Box::new(FieldType::scalar("TEXT"))),
    )];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    let err = render_err(&stmt);
    assert!(err.contains("array"), "expected array error, got: {err}");
}

#[test]
fn create_table_exclusion_error() {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))];
    schema.constraints = Some(vec![ConstraintDef::Exclusion {
        name: None,
        elements: vec![],
        index_method: "gist".into(),
        condition: None,
    }]);
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("EXCLUDE"),
        "expected exclusion error, got: {err}"
    );
}

// ==========================================================================
// CREATE TABLE — WITHOUT ROWID, STRICT, AUTOINCREMENT
// ==========================================================================

#[test]
fn create_table_without_rowid() {
    let mut schema = SchemaDef::new("kv");
    schema.columns = vec![
        ColumnDef::new("key", FieldType::scalar("TEXT")).not_null(),
        ColumnDef::new("value", FieldType::scalar("BLOB")),
    ];
    schema.constraints = Some(vec![ConstraintDef::PrimaryKey {
        name: None,
        columns: vec!["key".into()],
        include: None,
    }]);
    let stmt = SchemaMutationStmt::CreateTable {
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
        without_rowid: true,
        strict: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "kv" ("key" TEXT NOT NULL, "value" BLOB, PRIMARY KEY ("key")) WITHOUT ROWID"#,
    );
}

#[test]
fn create_table_strict() {
    let mut schema = SchemaDef::new("data");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INTEGER")).not_null(),
        ColumnDef::new("name", FieldType::scalar("TEXT")),
    ];
    let stmt = SchemaMutationStmt::CreateTable {
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
        strict: true,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "data" ("id" INTEGER NOT NULL, "name" TEXT) STRICT"#,
    );
}

#[test]
fn create_table_without_rowid_strict() {
    let mut schema = SchemaDef::new("kv_strict");
    schema.columns = vec![
        ColumnDef::new("key", FieldType::scalar("TEXT")).not_null(),
        ColumnDef::new("val", FieldType::scalar("INTEGER")),
    ];
    schema.constraints = Some(vec![ConstraintDef::PrimaryKey {
        name: None,
        columns: vec!["key".into()],
        include: None,
    }]);
    let stmt = SchemaMutationStmt::CreateTable {
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
        without_rowid: true,
        strict: true,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "kv_strict" ("key" TEXT NOT NULL, "val" INTEGER, PRIMARY KEY ("key")) WITHOUT ROWID, STRICT"#,
    );
}

#[test]
fn create_table_primary_key_autoincrement() {
    let mut schema = SchemaDef::new("events");
    let mut id_col = ColumnDef::new("id", FieldType::scalar("INTEGER"));
    id_col.not_null = true;
    id_col.identity = Some(IdentityColumn {
        always: true,
        ..Default::default()
    });
    schema.columns = vec![id_col, ColumnDef::new("name", FieldType::scalar("TEXT"))];
    schema.constraints = Some(vec![ConstraintDef::primary_key(vec!["id"])]);
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "events" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "name" TEXT)"#,
    );
}

// ==========================================================================
// DROP TABLE
// ==========================================================================

#[test]
fn drop_table_simple() {
    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("users"),
        if_exists: false,
        cascade: false,
    };
    assert_eq!(render(&stmt), r#"DROP TABLE "users""#);
}

#[test]
fn drop_table_if_exists() {
    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("users"),
        if_exists: true,
        cascade: false,
    };
    assert_eq!(render(&stmt), r#"DROP TABLE IF EXISTS "users""#);
}

#[test]
fn drop_table_cascade_ignored() {
    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("users"),
        if_exists: false,
        cascade: true,
    };
    // CASCADE is silently ignored
    assert_eq!(render(&stmt), r#"DROP TABLE "users""#);
}

// ==========================================================================
// ALTER TABLE (supported operations)
// ==========================================================================

#[test]
fn alter_table_rename() {
    let stmt = SchemaMutationStmt::RenameTable {
        schema_ref: SchemaRef::new("old_name"),
        new_name: "new_name".into(),
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "old_name" RENAME TO "new_name""#
    );
}

#[test]
fn alter_table_rename_column() {
    let stmt = SchemaMutationStmt::RenameColumn {
        schema_ref: SchemaRef::new("users"),
        old_name: "name".into(),
        new_name: "full_name".into(),
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" RENAME COLUMN "name" TO "full_name""#,
    );
}

#[test]
fn alter_table_add_column() {
    let stmt = SchemaMutationStmt::AddColumn {
        schema_ref: SchemaRef::new("users"),
        column: Box::new(ColumnDef::new("email", FieldType::scalar("TEXT"))),
        if_not_exists: false,
        position: None,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" ADD COLUMN "email" TEXT"#
    );
}

#[test]
fn alter_table_drop_column() {
    let stmt = SchemaMutationStmt::DropColumn {
        schema_ref: SchemaRef::new("users"),
        name: "old_field".into(),
        if_exists: false,
        cascade: false,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" DROP COLUMN "old_field""#
    );
}

// ==========================================================================
// ALTER TABLE (unsupported operations — errors)
// ==========================================================================

#[test]
fn alter_column_type_error() {
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
fn alter_column_default_error() {
    let stmt = SchemaMutationStmt::AlterColumnDefault {
        schema_ref: SchemaRef::new("t"),
        column_name: "x".into(),
        default: Some(Expr::Value(Value::Int(0))),
    };
    let err = render_err(&stmt);
    assert!(err.contains("ALTER COLUMN DEFAULT"), "got: {err}");
}

#[test]
fn alter_column_nullability_error() {
    let stmt = SchemaMutationStmt::AlterColumnNullability {
        schema_ref: SchemaRef::new("t"),
        column_name: "x".into(),
        not_null: true,
    };
    let err = render_err(&stmt);
    assert!(err.contains("ALTER COLUMN NOT NULL"), "got: {err}");
}

#[test]
fn add_constraint_error() {
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
fn drop_constraint_error() {
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
fn validate_constraint_error() {
    let stmt = SchemaMutationStmt::ValidateConstraint {
        schema_ref: SchemaRef::new("t"),
        constraint_name: "ck_1".into(),
    };
    let err = render_err(&stmt);
    assert!(err.contains("VALIDATE CONSTRAINT"), "got: {err}");
}

// ==========================================================================
// CREATE INDEX
// ==========================================================================

#[test]
fn create_index_simple() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new(
            "idx_name",
            vec![IndexColumnDef {
                expr: IndexExpr::Column("name".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
        ),
        if_not_exists: false,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE INDEX "idx_name" ON "users" ("name")"#
    );
}

#[test]
fn create_unique_index_if_not_exists() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new(
            "idx_email",
            vec![IndexColumnDef {
                expr: IndexExpr::Column("email".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
        )
        .unique(),
        if_not_exists: true,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE UNIQUE INDEX IF NOT EXISTS "idx_email" ON "users" ("email")"#,
    );
}

#[test]
fn create_index_concurrently_ignored() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("t"),
        index: IndexDef::new(
            "idx_a",
            vec![IndexColumnDef {
                expr: IndexExpr::Column("a".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
        ),
        if_not_exists: false,
        concurrently: true,
    };
    // CONCURRENTLY silently ignored
    assert_eq!(render(&stmt), r#"CREATE INDEX "idx_a" ON "t" ("a")"#);
}

#[test]
fn create_index_with_direction() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("events"),
        index: IndexDef::new(
            "idx_events",
            vec![
                IndexColumnDef {
                    expr: IndexExpr::Column("created_at".into()),
                    direction: Some(qcraft_core::ast::common::OrderDir::Desc),
                    nulls: None,
                    opclass: None,
                    collation: None,
                },
                IndexColumnDef {
                    expr: IndexExpr::Column("priority".into()),
                    direction: Some(qcraft_core::ast::common::OrderDir::Asc),
                    nulls: None,
                    opclass: None,
                    collation: None,
                },
            ],
        ),
        if_not_exists: false,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE INDEX "idx_events" ON "events" ("created_at" DESC, "priority" ASC)"#,
    );
}

#[test]
fn create_index_with_where() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef {
            name: "idx_active".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("email".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
            unique: false,
            index_type: None,
            include: None,
            condition: Some(Conditions {
                children: vec![ConditionNode::Comparison(Box::new(Comparison {
                    left: Expr::Raw {
                        sql: "\"active\"".into(),
                        params: vec![],
                    },
                    op: CompareOp::Eq,
                    right: Expr::Value(Value::Bool(true)),
                    negate: false,
                }))],
                connector: Connector::And,
                negated: false,
            }),
            parameters: None,
            tablespace: None,
            nulls_distinct: None,
        },
        if_not_exists: false,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE INDEX "idx_active" ON "users" ("email") WHERE "active" = 1"#,
    );
}

#[test]
fn create_index_expression() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new(
            "idx_lower_email",
            vec![IndexColumnDef {
                expr: IndexExpr::Expression(Expr::Func {
                    name: "lower".into(),
                    args: vec![Expr::Raw {
                        sql: "\"email\"".into(),
                        params: vec![],
                    }],
                }),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
        ),
        if_not_exists: false,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE INDEX "idx_lower_email" ON "users" ((lower("email")))"#,
    );
}

#[test]
fn create_index_include_ignored() {
    // INCLUDE is silently ignored for SQLite
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("t"),
        index: IndexDef {
            name: "idx_a".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("a".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
            unique: false,
            index_type: Some("btree".into()),
            include: Some(vec!["b".into()]),
            condition: None,
            parameters: Some(vec![("fillfactor".into(), "70".into())]),
            tablespace: Some("fast".into()),
            nulls_distinct: Some(false),
        },
        if_not_exists: false,
        concurrently: false,
    };
    // All PG-specific options silently ignored
    assert_eq!(render(&stmt), r#"CREATE INDEX "idx_a" ON "t" ("a")"#);
}

// ==========================================================================
// DROP INDEX
// ==========================================================================

#[test]
fn drop_index_simple() {
    let stmt = SchemaMutationStmt::DropIndex {
        schema_ref: SchemaRef::new("t"),
        index_name: "idx_name".into(),
        if_exists: false,
        concurrently: false,
        cascade: false,
    };
    assert_eq!(render(&stmt), r#"DROP INDEX "idx_name""#);
}

#[test]
fn drop_index_if_exists() {
    let stmt = SchemaMutationStmt::DropIndex {
        schema_ref: SchemaRef::new("t"),
        index_name: "idx_name".into(),
        if_exists: true,
        concurrently: true,
        cascade: true,
    };
    // CONCURRENTLY and CASCADE silently ignored
    assert_eq!(render(&stmt), r#"DROP INDEX IF EXISTS "idx_name""#);
}

// ==========================================================================
// Extension operations — errors
// ==========================================================================

#[test]
fn create_extension_error() {
    let stmt = SchemaMutationStmt::CreateExtension {
        name: "uuid-ossp".into(),
        if_not_exists: true,
        schema: None,
        version: None,
        cascade: false,
    };
    let err = render_err(&stmt);
    assert!(err.contains("extension"), "got: {err}");
}

#[test]
fn drop_extension_error() {
    let stmt = SchemaMutationStmt::DropExtension {
        name: "uuid-ossp".into(),
        if_exists: true,
        cascade: false,
    };
    let err = render_err(&stmt);
    assert!(err.contains("extension"), "got: {err}");
}

// ==========================================================================
// Bool rendering (SQLite uses 1/0 instead of TRUE/FALSE)
// ==========================================================================

#[test]
fn bool_rendered_as_integer() {
    let mut schema = SchemaDef::new("t");
    schema.columns = vec![
        ColumnDef::new("flag", FieldType::scalar("INTEGER"))
            .default(Expr::Value(Value::Bool(true))),
    ];
    let stmt = SchemaMutationStmt::CreateTable {
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
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "t" ("flag" INTEGER DEFAULT 1)"#
    );
}

// ==========================================================================
// TRUNCATE TABLE (rendered as DELETE FROM in SQLite)
// ==========================================================================

#[test]
fn truncate_table_renders_as_delete() {
    let stmt = SchemaMutationStmt::TruncateTable {
        schema_ref: SchemaRef::new("users"),
        restart_identity: false,
        cascade: false,
    };
    assert_eq!(render(&stmt), r#"DELETE FROM "users""#);
}

#[test]
fn truncate_table_options_ignored() {
    // restart_identity and cascade are silently ignored for SQLite
    let stmt = SchemaMutationStmt::TruncateTable {
        schema_ref: SchemaRef::new("orders"),
        restart_identity: true,
        cascade: true,
    };
    assert_eq!(render(&stmt), r#"DELETE FROM "orders""#);
}

// ==========================================================================
// Collation DDL unsupported
// ==========================================================================

#[test]
fn create_collation_unsupported() {
    let stmt = SchemaMutationStmt::create_collation("my_coll");
    let err = render_err(&stmt);
    assert!(err.contains("CreateCollation"));
}

#[test]
fn drop_collation_unsupported() {
    let stmt = SchemaMutationStmt::drop_collation("my_coll");
    let err = render_err(&stmt);
    assert!(err.contains("DropCollation"));
}
