use rquery_core::ast::common::{OrderDir, SchemaRef};
use rquery_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use rquery_core::ast::ddl::*;
use rquery_core::ast::expr::Expr;
use rquery_core::ast::value::Value;
use rquery_postgres::PostgresRenderer;

fn render(stmt: &SchemaMutationStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _params) = renderer.render_schema_stmt(stmt).unwrap();
    sql
}

fn render_with_params(stmt: &SchemaMutationStmt) -> (String, Vec<Value>) {
    let renderer = PostgresRenderer::new();
    renderer.render_schema_stmt(stmt).unwrap()
}

// ==========================================================================
// CREATE TABLE
// ==========================================================================

#[test]
fn create_table_simple() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "users".into(),
            namespace: None,
            columns: vec![
                ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
                ColumnDef::new("name", FieldType::scalar("TEXT")),
            ],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "users"("id" BIGINT NOT NULL, "name" TEXT)"#
    );
}

#[test]
fn create_table_if_not_exists() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "users".into(),
            namespace: None,
            columns: vec![ColumnDef::new("id", FieldType::scalar("INTEGER"))],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE IF NOT EXISTS "users"("id" INTEGER)"#
    );
}

#[test]
fn create_table_temporary_unlogged() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "tmp".into(),
            namespace: None,
            columns: vec![ColumnDef::new("x", FieldType::scalar("INT"))],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
        if_not_exists: false,
        temporary: true,
        unlogged: true,
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
        r#"CREATE TEMPORARY UNLOGGED TABLE "tmp"("x" INT)"#
    );
}

#[test]
fn create_table_with_namespace() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "users".into(),
            namespace: Some("public".into()),
            columns: vec![ColumnDef::new("id", FieldType::scalar("INT"))],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "public"."users"("id" INT)"#
    );
}

#[test]
fn create_table_with_default() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "posts".into(),
            namespace: None,
            columns: vec![
                ColumnDef::new("id", FieldType::scalar("SERIAL")),
                ColumnDef {
                    name: "status".into(),
                    field_type: FieldType::scalar("TEXT"),
                    not_null: true,
                    default: Some(Expr::Value(Value::Str("draft".into()))),
                    generated: None,
                    identity: None,
                    collation: None,
                    comment: None,
                    storage: None,
                    compression: None,
                },
            ],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "posts"("id" SERIAL, "status" TEXT NOT NULL DEFAULT 'draft')"#
    );
}

#[test]
fn create_table_with_identity() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "items".into(),
            namespace: None,
            columns: vec![ColumnDef {
                name: "id".into(),
                field_type: FieldType::scalar("BIGINT"),
                not_null: true,
                default: None,
                generated: None,
                identity: Some(IdentityColumn {
                    always: true,
                    start: Some(1),
                    increment: Some(1),
                    ..Default::default()
                }),
                collation: None,
                comment: None,
                storage: None,
                compression: None,
            }],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "items"("id" BIGINT NOT NULL GENERATED ALWAYS AS IDENTITY(START WITH 1 INCREMENT BY 1))"#
    );
}

#[test]
fn create_table_with_generated_column() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "products".into(),
            namespace: None,
            columns: vec![
                ColumnDef::new("price", FieldType::scalar("NUMERIC")),
                ColumnDef::new("qty", FieldType::scalar("INTEGER")),
                ColumnDef {
                    name: "total".into(),
                    field_type: FieldType::scalar("NUMERIC"),
                    not_null: false,
                    default: None,
                    generated: Some(GeneratedColumn {
                        expr: Expr::Raw {
                            sql: "price * qty".into(),
                            params: vec![],
                        },
                        stored: true,
                    }),
                    identity: None,
                    collation: None,
                    comment: None,
                    storage: None,
                    compression: None,
                },
            ],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "products"("price" NUMERIC, "qty" INTEGER, "total" NUMERIC GENERATED ALWAYS AS (price * qty) STORED)"#
    );
}

#[test]
fn create_table_parameterized_types() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "t".into(),
            namespace: None,
            columns: vec![
                ColumnDef::new("name", FieldType::parameterized("VARCHAR", vec!["255"])),
                ColumnDef::new("amount", FieldType::parameterized("NUMERIC", vec!["10", "2"])),
                ColumnDef::new("tags", FieldType::Array(Box::new(FieldType::scalar("TEXT")))),
                ColumnDef::new("embedding", FieldType::Vector(1536)),
            ],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "t"("name" VARCHAR(255), "amount" NUMERIC(10, 2), "tags" TEXT[], "embedding" VECTOR(1536))"#
    );
}

// ==========================================================================
// CREATE TABLE with constraints
// ==========================================================================

#[test]
fn create_table_primary_key() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "users".into(),
            namespace: None,
            columns: vec![
                ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
                ColumnDef::new("name", FieldType::scalar("TEXT")),
            ],
            constraints: Some(vec![ConstraintDef::PrimaryKey {
                name: Some("pk_users".into()),
                columns: vec!["id".into()],
                include: None,
                autoincrement: false,
            }]),
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "users"("id" BIGINT NOT NULL, "name" TEXT, CONSTRAINT "pk_users" PRIMARY KEY("id"))"#
    );
}

#[test]
fn create_table_foreign_key() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "posts".into(),
            namespace: None,
            columns: vec![
                ColumnDef::new("id", FieldType::scalar("BIGINT")),
                ColumnDef::new("user_id", FieldType::scalar("BIGINT")),
            ],
            constraints: Some(vec![ConstraintDef::ForeignKey {
                name: Some("fk_posts_user".into()),
                columns: vec!["user_id".into()],
                ref_table: SchemaRef::new("users"),
                ref_columns: vec!["id".into()],
                on_delete: Some(ReferentialAction::Cascade),
                on_update: Some(ReferentialAction::NoAction),
                deferrable: None,
                match_type: None,
            }]),
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "posts"("id" BIGINT, "user_id" BIGINT, CONSTRAINT "fk_posts_user" FOREIGN KEY("user_id") REFERENCES "users"("id") ON DELETE CASCADE ON UPDATE NO ACTION)"#
    );
}

#[test]
fn create_table_unique_check() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "accounts".into(),
            namespace: None,
            columns: vec![
                ColumnDef::new("id", FieldType::scalar("INT")),
                ColumnDef::new("email", FieldType::scalar("TEXT")),
                ColumnDef::new("age", FieldType::scalar("INT")),
            ],
            constraints: Some(vec![
                ConstraintDef::Unique {
                    name: Some("uq_email".into()),
                    columns: vec!["email".into()],
                    include: None,
                    nulls_distinct: None,
                    condition: None,
                },
                ConstraintDef::Check {
                    name: Some("ck_age".into()),
                    condition: Conditions {
                        children: vec![ConditionNode::Comparison(Comparison {
                            left: Expr::Raw { sql: "\"age\"".into(), params: vec![] },
                            op: CompareOp::Gt,
                            right: Expr::Value(Value::Int(0)),
                            negate: false,
                        })],
                        connector: Connector::And,
                        negated: false,
                    },
                    no_inherit: false,
                    enforced: None,
                },
            ]),
            indexes: None,
            like_tables: None,
        },
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
        r#"CREATE TABLE "accounts"("id" INT, "email" TEXT, "age" INT, CONSTRAINT "uq_email" UNIQUE("email"), CONSTRAINT "ck_age" CHECK("age" > 0))"#
    );
}

#[test]
fn create_table_with_tablespace() {
    let stmt = SchemaMutationStmt::CreateTable {
        schema: SchemaDef {
            name: "big".into(),
            namespace: None,
            columns: vec![ColumnDef::new("id", FieldType::scalar("INT"))],
            constraints: None,
            indexes: None,
            like_tables: None,
        },
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: Some("fast_storage".into()),
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
        r#"CREATE TABLE "big"("id" INT) TABLESPACE "fast_storage""#
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
fn drop_table_if_exists_cascade() {
    let stmt = SchemaMutationStmt::DropTable {
        schema_ref: SchemaRef::new("users"),
        if_exists: true,
        cascade: true,
    };
    assert_eq!(render(&stmt), r#"DROP TABLE IF EXISTS "users" CASCADE"#);
}

// ==========================================================================
// ALTER TABLE
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
fn alter_table_add_column() {
    let stmt = SchemaMutationStmt::AddColumn {
        schema_ref: SchemaRef::new("users"),
        column: ColumnDef::new("email", FieldType::scalar("TEXT")).not_null(),
        if_not_exists: true,
        position: None,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "email" TEXT NOT NULL"#
    );
}

#[test]
fn alter_table_drop_column() {
    let stmt = SchemaMutationStmt::DropColumn {
        schema_ref: SchemaRef::new("users"),
        name: "old_col".into(),
        if_exists: true,
        cascade: true,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" DROP COLUMN IF EXISTS "old_col" CASCADE"#
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
        r#"ALTER TABLE "users" RENAME COLUMN "name" TO "full_name""#
    );
}

#[test]
fn alter_column_type() {
    let stmt = SchemaMutationStmt::AlterColumnType {
        schema_ref: SchemaRef::new("users"),
        column_name: "age".into(),
        new_type: FieldType::scalar("BIGINT"),
        using_expr: None,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" ALTER COLUMN "age" SET DATA TYPE BIGINT"#
    );
}

#[test]
fn alter_column_type_with_using() {
    let stmt = SchemaMutationStmt::AlterColumnType {
        schema_ref: SchemaRef::new("t"),
        column_name: "x".into(),
        new_type: FieldType::scalar("INTEGER"),
        using_expr: Some(Expr::Raw {
            sql: "x::INTEGER".into(),
            params: vec![],
        }),
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "t" ALTER COLUMN "x" SET DATA TYPE INTEGER USING x::INTEGER"#
    );
}

#[test]
fn alter_column_set_default() {
    let stmt = SchemaMutationStmt::AlterColumnDefault {
        schema_ref: SchemaRef::new("users"),
        column_name: "status".into(),
        default: Some(Expr::Value(Value::Str("active".into()))),
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" ALTER COLUMN "status" SET DEFAULT 'active'"#
    );
}

#[test]
fn alter_column_drop_default() {
    let stmt = SchemaMutationStmt::AlterColumnDefault {
        schema_ref: SchemaRef::new("users"),
        column_name: "status".into(),
        default: None,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" ALTER COLUMN "status" DROP DEFAULT"#
    );
}

#[test]
fn alter_column_set_not_null() {
    let stmt = SchemaMutationStmt::AlterColumnNullability {
        schema_ref: SchemaRef::new("users"),
        column_name: "email".into(),
        not_null: true,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" ALTER COLUMN "email" SET NOT NULL"#
    );
}

#[test]
fn alter_column_drop_not_null() {
    let stmt = SchemaMutationStmt::AlterColumnNullability {
        schema_ref: SchemaRef::new("users"),
        column_name: "email".into(),
        not_null: false,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" ALTER COLUMN "email" DROP NOT NULL"#
    );
}

#[test]
fn alter_table_add_constraint() {
    let stmt = SchemaMutationStmt::AddConstraint {
        schema_ref: SchemaRef::new("users"),
        constraint: ConstraintDef::Unique {
            name: Some("uq_email".into()),
            columns: vec!["email".into()],
            include: None,
            nulls_distinct: None,
            condition: None,
        },
        not_valid: false,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" ADD CONSTRAINT "uq_email" UNIQUE("email")"#
    );
}

#[test]
fn alter_table_add_constraint_not_valid() {
    let stmt = SchemaMutationStmt::AddConstraint {
        schema_ref: SchemaRef::new("orders"),
        constraint: ConstraintDef::ForeignKey {
            name: Some("fk_user".into()),
            columns: vec!["user_id".into()],
            ref_table: SchemaRef::new("users"),
            ref_columns: vec!["id".into()],
            on_delete: None,
            on_update: None,
            deferrable: None,
            match_type: None,
        },
        not_valid: true,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "orders" ADD CONSTRAINT "fk_user" FOREIGN KEY("user_id") REFERENCES "users"("id") NOT VALID"#
    );
}

#[test]
fn alter_table_drop_constraint() {
    let stmt = SchemaMutationStmt::DropConstraint {
        schema_ref: SchemaRef::new("users"),
        constraint_name: "uq_email".into(),
        if_exists: true,
        cascade: false,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" DROP CONSTRAINT IF EXISTS "uq_email""#
    );
}

#[test]
fn alter_table_validate_constraint() {
    let stmt = SchemaMutationStmt::ValidateConstraint {
        schema_ref: SchemaRef::new("orders"),
        constraint_name: "fk_user".into(),
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "orders" VALIDATE CONSTRAINT "fk_user""#
    );
}

#[test]
fn alter_table_rename_constraint() {
    let stmt = SchemaMutationStmt::RenameConstraint {
        schema_ref: SchemaRef::new("users"),
        old_name: "old_pk".into(),
        new_name: "new_pk".into(),
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "users" RENAME CONSTRAINT "old_pk" TO "new_pk""#
    );
}

// ==========================================================================
// CREATE INDEX
// ==========================================================================

#[test]
fn create_index_simple() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new("idx_users_email", vec![
            IndexColumnDef {
                expr: IndexExpr::Column("email".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            },
        ]),
        if_not_exists: false,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE INDEX "idx_users_email" ON "users"("email")"#
    );
}

#[test]
fn create_unique_index_concurrently() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new("idx_users_email", vec![
            IndexColumnDef {
                expr: IndexExpr::Column("email".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            },
        ]).unique(),
        if_not_exists: true,
        concurrently: true,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS "idx_users_email" ON "users"("email")"#
    );
}

#[test]
fn create_index_with_type_and_options() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("docs"),
        index: IndexDef {
            name: "idx_docs_content".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("content".into()),
                direction: None,
                nulls: None,
                opclass: Some("gin_trgm_ops".into()),
                collation: None,
            }],
            unique: false,
            index_type: Some("GIN".into()),
            include: None,
            condition: None,
            parameters: None,
            tablespace: None,
            nulls_distinct: None,
        },
        if_not_exists: false,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE INDEX "idx_docs_content" ON "docs" USING GIN("content" gin_trgm_ops)"#
    );
}

#[test]
fn create_index_multi_column_with_direction() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("events"),
        index: IndexDef::new("idx_events_composite", vec![
            IndexColumnDef {
                expr: IndexExpr::Column("created_at".into()),
                direction: Some(OrderDir::Desc),
                nulls: Some(NullsOrder::Last),
                opclass: None,
                collation: None,
            },
            IndexColumnDef {
                expr: IndexExpr::Column("priority".into()),
                direction: Some(OrderDir::Asc),
                nulls: None,
                opclass: None,
                collation: None,
            },
        ]),
        if_not_exists: false,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE INDEX "idx_events_composite" ON "events"("created_at" DESC NULLS LAST, "priority" ASC)"#
    );
}

#[test]
fn create_index_with_include_and_where() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef {
            name: "idx_active_email".into(),
            columns: vec![IndexColumnDef {
                expr: IndexExpr::Column("email".into()),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            }],
            unique: true,
            index_type: None,
            include: Some(vec!["name".into()]),
            condition: Some(Conditions {
                children: vec![ConditionNode::Comparison(Comparison {
                    left: Expr::Raw { sql: "\"active\"".into(), params: vec![] },
                    op: CompareOp::Eq,
                    right: Expr::Value(Value::Bool(true)),
                    negate: false,
                })],
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
        r#"CREATE UNIQUE INDEX "idx_active_email" ON "users"("email") INCLUDE("name") WHERE "active" = TRUE"#
    );
}

#[test]
fn create_index_expression() {
    let stmt = SchemaMutationStmt::CreateIndex {
        schema_ref: SchemaRef::new("users"),
        index: IndexDef::new("idx_lower_email", vec![
            IndexColumnDef {
                expr: IndexExpr::Expression(Expr::Func {
                    name: "lower".into(),
                    args: vec![Expr::Raw { sql: "\"email\"".into(), params: vec![] }],
                }),
                direction: None,
                nulls: None,
                opclass: None,
                collation: None,
            },
        ]),
        if_not_exists: false,
        concurrently: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE INDEX "idx_lower_email" ON "users"((lower("email")))"#
    );
}

// ==========================================================================
// DROP INDEX
// ==========================================================================

#[test]
fn drop_index_simple() {
    let stmt = SchemaMutationStmt::DropIndex {
        schema_ref: SchemaRef::new("users"),
        index_name: "idx_email".into(),
        if_exists: false,
        concurrently: false,
        cascade: false,
    };
    assert_eq!(render(&stmt), r#"DROP INDEX "idx_email""#);
}

#[test]
fn drop_index_concurrently_if_exists() {
    let stmt = SchemaMutationStmt::DropIndex {
        schema_ref: SchemaRef::new("users"),
        index_name: "idx_email".into(),
        if_exists: true,
        concurrently: true,
        cascade: true,
    };
    assert_eq!(
        render(&stmt),
        r#"DROP INDEX CONCURRENTLY IF EXISTS "idx_email" CASCADE"#
    );
}

// ==========================================================================
// Extensions
// ==========================================================================

#[test]
fn create_extension() {
    let stmt = SchemaMutationStmt::CreateExtension {
        name: "pg_trgm".into(),
        if_not_exists: true,
        schema: Some("public".into()),
        version: None,
        cascade: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE EXTENSION IF NOT EXISTS "pg_trgm" SCHEMA "public""#
    );
}

#[test]
fn drop_extension() {
    let stmt = SchemaMutationStmt::DropExtension {
        name: "pg_trgm".into(),
        if_exists: true,
        cascade: true,
    };
    assert_eq!(
        render(&stmt),
        r#"DROP EXTENSION IF EXISTS "pg_trgm" CASCADE"#
    );
}

// ==========================================================================
// Deferrable FK
// ==========================================================================

#[test]
fn foreign_key_deferrable() {
    let stmt = SchemaMutationStmt::AddConstraint {
        schema_ref: SchemaRef::new("orders"),
        constraint: ConstraintDef::ForeignKey {
            name: Some("fk_user".into()),
            columns: vec!["user_id".into()],
            ref_table: SchemaRef::new("users"),
            ref_columns: vec!["id".into()],
            on_delete: Some(ReferentialAction::SetNull(None)),
            on_update: None,
            deferrable: Some(DeferrableConstraint {
                deferrable: true,
                initially_deferred: true,
            }),
            match_type: Some(MatchType::Full),
        },
        not_valid: false,
    };
    assert_eq!(
        render(&stmt),
        r#"ALTER TABLE "orders" ADD CONSTRAINT "fk_user" FOREIGN KEY("user_id") REFERENCES "users"("id") MATCH FULL ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED"#
    );
}

// ==========================================================================
// TRUNCATE TABLE
// ==========================================================================

#[test]
fn truncate_table_simple() {
    let stmt = SchemaMutationStmt::TruncateTable {
        schema_ref: SchemaRef::new("users"),
        restart_identity: false,
        cascade: false,
    };
    assert_eq!(render(&stmt), r#"TRUNCATE TABLE "users""#);
}

#[test]
fn truncate_table_restart_identity_cascade() {
    let stmt = SchemaMutationStmt::TruncateTable {
        schema_ref: SchemaRef::new("orders"),
        restart_identity: true,
        cascade: true,
    };
    assert_eq!(render(&stmt), r#"TRUNCATE TABLE "orders" RESTART IDENTITY CASCADE"#);
}

// ==========================================================================
// Extended CREATE TABLE features
// ==========================================================================

#[test]
fn create_table_partition_by_range() {
    use rquery_core::ast::ddl::*;
    let mut schema = SchemaDef::new("logs");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("BIGINT")).not_null(),
        ColumnDef::new("created_at", FieldType::scalar("TIMESTAMPTZ")).not_null(),
        ColumnDef::new("message", FieldType::scalar("TEXT")),
    ];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: Some(PartitionByDef {
            strategy: PartitionStrategy::Range,
            columns: vec![PartitionColumnDef {
                expr: IndexExpr::Column("created_at".into()),
                collation: None,
                opclass: None,
            }],
        }),
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
        r#"CREATE TABLE "logs"("id" BIGINT NOT NULL, "created_at" TIMESTAMPTZ NOT NULL, "message" TEXT) PARTITION BY RANGE("created_at")"#,
    );
}

#[test]
fn create_table_partition_by_list_with_expression() {
    use rquery_core::ast::ddl::*;
    let mut schema = SchemaDef::new("events");
    schema.columns = vec![
        ColumnDef::new("id", FieldType::scalar("INT")),
        ColumnDef::new("region", FieldType::scalar("TEXT")),
    ];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: Some(PartitionByDef {
            strategy: PartitionStrategy::List,
            columns: vec![PartitionColumnDef {
                expr: IndexExpr::Expression(Expr::Func {
                    name: "lower".into(),
                    args: vec![Expr::Raw { sql: "region".into(), params: vec![] }],
                }),
                collation: None,
                opclass: None,
            }],
        }),
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
        r#"CREATE TABLE "events"("id" INT, "region" TEXT) PARTITION BY LIST((lower(region)))"#,
    );
}

#[test]
fn create_table_inherits() {
    let mut schema = SchemaDef::new("child_table");
    schema.columns = vec![ColumnDef::new("extra", FieldType::scalar("TEXT"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: None,
        inherits: Some(vec![SchemaRef::new("parent_table")]),
        using_method: None,
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: false,
        strict: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "child_table"("extra" TEXT) INHERITS("parent_table")"#,
    );
}

#[test]
fn create_table_with_options() {
    let mut schema = SchemaDef::new("hot_data");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INT"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: None,
        inherits: None,
        using_method: None,
        with_options: Some(vec![
            ("fillfactor".into(), "70".into()),
            ("autovacuum_enabled".into(), "true".into()),
        ]),
        on_commit: None,
        table_options: None,
        without_rowid: false,
        strict: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "hot_data"("id" INT) WITH(fillfactor = 70, autovacuum_enabled = true)"#,
    );
}

#[test]
fn create_table_on_commit() {
    use rquery_core::ast::ddl::OnCommitAction;
    let mut schema = SchemaDef::new("temp_work");
    schema.columns = vec![ColumnDef::new("data", FieldType::scalar("TEXT"))];
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
        on_commit: Some(OnCommitAction::DeleteRows),
        table_options: None,
        without_rowid: false,
        strict: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TEMPORARY TABLE "temp_work"("data" TEXT) ON COMMIT DELETE ROWS"#,
    );
}

#[test]
fn create_table_using_method() {
    let mut schema = SchemaDef::new("columnar_data");
    schema.columns = vec![ColumnDef::new("id", FieldType::scalar("INT"))];
    let stmt = SchemaMutationStmt::CreateTable {
        schema,
        if_not_exists: false,
        temporary: false,
        unlogged: false,
        tablespace: None,
        partition_by: None,
        inherits: None,
        using_method: Some("columnar".into()),
        with_options: None,
        on_commit: None,
        table_options: None,
        without_rowid: false,
        strict: false,
    };
    assert_eq!(
        render(&stmt),
        r#"CREATE TABLE "columnar_data"("id" INT) USING columnar"#,
    );
}

#[test]
fn create_table_like() {
    use rquery_core::ast::ddl::*;
    let mut schema = SchemaDef::new("users_copy");
    schema.like_tables = Some(vec![LikeTableDef {
        source_table: SchemaRef::new("users"),
        options: vec![
            LikeOption { kind: LikeOptionKind::All, include: true },
            LikeOption { kind: LikeOptionKind::Indexes, include: false },
        ],
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
        r#"CREATE TABLE "users_copy"(LIKE "users" INCLUDING ALL EXCLUDING INDEXES)"#,
    );
}

#[test]
fn create_table_column_storage_compression() {
    let mut schema = SchemaDef::new("docs");
    schema.columns = vec![ColumnDef {
        name: "body".into(),
        field_type: FieldType::scalar("TEXT"),
        not_null: false,
        default: None,
        generated: None,
        identity: None,
        collation: None,
        comment: None,
        storage: Some("EXTERNAL".into()),
        compression: Some("lz4".into()),
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
        r#"CREATE TABLE "docs"("body" TEXT STORAGE EXTERNAL COMPRESSION lz4)"#,
    );
}
