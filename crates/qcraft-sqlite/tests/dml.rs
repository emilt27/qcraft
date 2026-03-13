use qcraft_core::ast::common::{FieldRef, SchemaRef};
use qcraft_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use qcraft_core::ast::dml::*;
use qcraft_core::ast::expr::Expr;
use qcraft_core::ast::query::SelectColumn;
use qcraft_core::ast::value::Value;
use qcraft_sqlite::SqliteRenderer;

fn render(stmt: &MutationStmt) -> String {
    let renderer = SqliteRenderer::new();
    let (sql, _) = renderer.render_mutation_stmt(stmt).unwrap();
    sql
}

fn render_with_params(stmt: &MutationStmt) -> (String, Vec<Value>) {
    let renderer = SqliteRenderer::new();
    renderer.render_mutation_stmt(stmt).unwrap()
}

fn render_err(stmt: &MutationStmt) -> String {
    let renderer = SqliteRenderer::new();
    renderer.render_mutation_stmt(stmt).unwrap_err().to_string()
}

// ==========================================================================
// INSERT — basic
// ==========================================================================

#[test]
fn insert_single_row() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice".into())),
            Expr::Value(Value::Str("alice@example.com".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("name", "email") VALUES (?, ?)"#
    );
    assert_eq!(
        params,
        vec![
            Value::Str("Alice".into()),
            Value::Str("alice@example.com".into())
        ]
    );
}

#[test]
fn insert_multi_row() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into()]),
        source: InsertSource::Values(vec![
            vec![Expr::Value(Value::Str("Alice".into()))],
            vec![Expr::Value(Value::Str("Bob".into()))],
        ]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"INSERT INTO "users" ("name") VALUES (?), (?)"#);
    assert_eq!(
        params,
        vec![Value::Str("Alice".into()), Value::Str("Bob".into())]
    );
}

#[test]
fn insert_default_values() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("counters"),
        columns: None,
        source: InsertSource::DefaultValues,
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    assert_eq!(render(&stmt), r#"INSERT INTO "counters" DEFAULT VALUES"#);
}

#[test]
fn insert_with_namespace() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users").with_namespace("main"),
        columns: Some(vec!["id".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Int(1))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"INSERT INTO "main"."users" ("id") VALUES (?)"#);
    assert_eq!(params, vec![Value::Int(1)]);
}

// ==========================================================================
// INSERT — conflict resolution (OR REPLACE, OR IGNORE, etc.)
// ==========================================================================

#[test]
fn insert_or_replace() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["id".into(), "name".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Int(1)),
            Expr::Value(Value::Str("Alice".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: Some(ConflictResolution::Replace),
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT OR REPLACE INTO "users" ("id", "name") VALUES (?, ?)"#
    );
    assert_eq!(params, vec![Value::Int(1), Value::Str("Alice".into())]);
}

#[test]
fn insert_or_ignore() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["id".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Int(1))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: Some(ConflictResolution::Ignore),
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"INSERT OR IGNORE INTO "users" ("id") VALUES (?)"#);
    assert_eq!(params, vec![Value::Int(1)]);
}

#[test]
fn insert_or_abort() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("t"),
        columns: None,
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Int(1))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: Some(ConflictResolution::Abort),
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"INSERT OR ABORT INTO "t" VALUES (?)"#);
    assert_eq!(params, vec![Value::Int(1)]);
}

// ==========================================================================
// INSERT — RETURNING
// ==========================================================================

#[test]
fn insert_returning_star() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("Alice".into()))]]),
        on_conflict: None,
        returning: Some(vec![SelectColumn::Star(None)]),
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("name") VALUES (?) RETURNING *"#
    );
    assert_eq!(params, vec![Value::Str("Alice".into())]);
}

#[test]
fn insert_returning_columns() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("Alice".into()))]]),
        on_conflict: None,
        returning: Some(vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: Some("user_name".into()),
            },
        ]),
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("name") VALUES (?) RETURNING "users"."id", "users"."name" AS "user_name""#
    );
    assert_eq!(params, vec![Value::Str("Alice".into())]);
}

// ==========================================================================
// INSERT — ON CONFLICT
// ==========================================================================

#[test]
fn insert_on_conflict_do_nothing() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["email".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("a@b.com".into()))]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Columns {
                columns: vec!["email".into()],
                where_clause: None,
            }),
            action: ConflictAction::DoNothing,
        }]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("email") VALUES (?) ON CONFLICT ("email") DO NOTHING"#
    );
    assert_eq!(params, vec![Value::Str("a@b.com".into())]);
}

#[test]
fn insert_on_conflict_do_update() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["email".into(), "name".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("a@b.com".into())),
            Expr::Value(Value::Str("Alice".into())),
        ]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Columns {
                columns: vec!["email".into()],
                where_clause: None,
            }),
            action: ConflictAction::DoUpdate {
                assignments: vec![(
                    "name".into(),
                    Expr::Raw {
                        sql: "excluded.\"name\"".into(),
                        params: vec![],
                    },
                )],
                where_clause: None,
            },
        }]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("email", "name") VALUES (?, ?) ON CONFLICT ("email") DO UPDATE SET "name" = excluded."name""#
    );
    assert_eq!(
        params,
        vec![Value::Str("a@b.com".into()), Value::Str("Alice".into())]
    );
}

#[test]
fn insert_on_conflict_catch_all() {
    // SQLite allows last ON CONFLICT clause without target (catch-all)
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("t"),
        columns: Some(vec!["id".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Int(1))]]),
        on_conflict: Some(vec![OnConflictDef {
            target: None,
            action: ConflictAction::DoNothing,
        }]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "t" ("id") VALUES (?) ON CONFLICT DO NOTHING"#
    );
    assert_eq!(params, vec![Value::Int(1)]);
}

#[test]
fn insert_on_conflict_on_constraint_error() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("t"),
        columns: Some(vec!["id".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Int(1))]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Constraint("pk_t".into())),
            action: ConflictAction::DoNothing,
        }]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let err = render_err(&stmt);
    assert!(err.contains("ON CONSTRAINT"), "got: {err}");
}

#[test]
fn insert_multiple_on_conflict() {
    // SQLite supports multiple ON CONFLICT clauses processed in order
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["id".into(), "email".into(), "name".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Int(1)),
            Expr::Value(Value::Str("a@b.com".into())),
            Expr::Value(Value::Str("Alice".into())),
        ]]),
        on_conflict: Some(vec![
            OnConflictDef {
                target: Some(ConflictTarget::Columns {
                    columns: vec!["id".into()],
                    where_clause: None,
                }),
                action: ConflictAction::DoNothing,
            },
            OnConflictDef {
                target: Some(ConflictTarget::Columns {
                    columns: vec!["email".into()],
                    where_clause: None,
                }),
                action: ConflictAction::DoUpdate {
                    assignments: vec![(
                        "name".into(),
                        Expr::Raw {
                            sql: "excluded.\"name\"".into(),
                            params: vec![],
                        },
                    )],
                    where_clause: None,
                },
            },
        ]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("id", "email", "name") VALUES (?, ?, ?) ON CONFLICT ("id") DO NOTHING ON CONFLICT ("email") DO UPDATE SET "name" = excluded."name""#
    );
    assert_eq!(
        params,
        vec![
            Value::Int(1),
            Value::Str("a@b.com".into()),
            Value::Str("Alice".into())
        ]
    );
}

// ==========================================================================
// INSERT — bool values (1/0)
// ==========================================================================

#[test]
fn insert_bool_as_integer() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("flags"),
        columns: Some(vec!["active".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Bool(true))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"INSERT INTO "flags" ("active") VALUES (?)"#);
    assert_eq!(params, vec![Value::Bool(true)]);
}

// ==========================================================================
// UPDATE — basic
// ==========================================================================

#[test]
fn update_simple() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            }))],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"UPDATE "users" SET "name" = ? WHERE "id" = ?"#);
    assert_eq!(params, vec![Value::Str("Bob".into()), Value::Int(1)]);
}

#[test]
fn update_or_replace() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: None,
        returning: None,
        ctes: None,
        conflict_resolution: Some(ConflictResolution::Replace),
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"UPDATE OR REPLACE "users" SET "name" = ?"#);
    assert_eq!(params, vec![Value::Str("Bob".into())]);
}

#[test]
fn update_with_returning() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            }))],
            connector: Connector::And,
            negated: false,
        }),
        returning: Some(vec![SelectColumn::Star(None)]),
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"UPDATE "users" SET "name" = ? WHERE "id" = ? RETURNING *"#
    );
    assert_eq!(params, vec![Value::Str("Bob".into()), Value::Int(1)]);
}

#[test]
fn update_with_alias() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users").with_alias("u"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: None,
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"UPDATE "users" AS "u" SET "name" = ?"#);
    assert_eq!(params, vec![Value::Str("Bob".into())]);
}

#[test]
fn update_with_limit_offset() {
    use qcraft_core::ast::common::OrderByDef;
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("logs"),
        assignments: vec![("archived".into(), Expr::Value(Value::Bool(true)))],
        from: None,
        where_clause: None,
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("logs", "created_at")),
            direction: qcraft_core::ast::common::OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(100),
        offset: Some(10),
        only: false,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"UPDATE "logs" SET "archived" = ? ORDER BY "logs"."created_at" ASC LIMIT 100 OFFSET 10"#
    );
    assert_eq!(params, vec![Value::Bool(true)]);
}

#[test]
fn update_with_from() {
    use qcraft_core::ast::query::TableSource;
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("orders").with_alias("o"),
        assignments: vec![("status".into(), Expr::Value(Value::Str("shipped".into())))],
        from: Some(vec![TableSource::Table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Raw {
                    sql: "\"o\".\"user_id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Raw {
                    sql: "\"u\".\"id\"".into(),
                    params: vec![],
                },
                negate: false,
            }))],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    let sql = render(&stmt);
    assert!(sql.contains(r#"UPDATE "orders" AS "o""#), "got: {sql}");
    assert!(sql.contains("FROM"), "expected FROM, got: {sql}");
    assert!(
        sql.contains(r#""users" AS "u""#),
        "expected users alias, got: {sql}"
    );
}

// ==========================================================================
// DELETE — basic
// ==========================================================================

#[test]
fn delete_simple() {
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            }))],
            connector: Connector::And,
            negated: false,
        }),
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"DELETE FROM "users" WHERE "id" = ?"#);
    assert_eq!(params, vec![Value::Int(1)]);
}

#[test]
fn delete_no_where() {
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("temp"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    assert_eq!(render(&stmt), r#"DELETE FROM "temp""#);
}

#[test]
fn delete_with_returning() {
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Raw {
                    sql: "\"active\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(false)),
                negate: false,
            }))],
            connector: Connector::And,
            negated: false,
        }),
        returning: Some(vec![SelectColumn::Star(None)]),
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"DELETE FROM "users" WHERE "active" = ? RETURNING *"#);
    assert_eq!(params, vec![Value::Bool(false)]);
}

#[test]
fn delete_with_alias() {
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users").with_alias("u"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    assert_eq!(render(&stmt), r#"DELETE FROM "users" AS "u""#);
}

#[test]
fn delete_with_limit_offset() {
    use qcraft_core::ast::common::OrderByDef;
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("logs"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("logs", "created_at")),
            direction: qcraft_core::ast::common::OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(50),
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });
    assert_eq!(
        render(&stmt),
        r#"DELETE FROM "logs" ORDER BY "logs"."created_at" ASC LIMIT 50"#,
    );
}

// ==========================================================================
// Full upsert scenario
// ==========================================================================

#[test]
fn insert_upsert_returning() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["email".into(), "name".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("alice@example.com".into())),
            Expr::Value(Value::Str("Alice".into())),
        ]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Columns {
                columns: vec!["email".into()],
                where_clause: None,
            }),
            action: ConflictAction::DoUpdate {
                assignments: vec![(
                    "name".into(),
                    Expr::Raw {
                        sql: "excluded.\"name\"".into(),
                        params: vec![],
                    },
                )],
                where_clause: None,
            },
        }]),
        returning: Some(vec![SelectColumn::Star(None)]),
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("email", "name") VALUES (?, ?) ON CONFLICT ("email") DO UPDATE SET "name" = excluded."name" RETURNING *"#
    );
    assert_eq!(
        params,
        vec![
            Value::Str("alice@example.com".into()),
            Value::Str("Alice".into())
        ]
    );
}

// ---------------------------------------------------------------------------
// Array → JSON string conversion
// ---------------------------------------------------------------------------

#[test]
fn insert_array_value_as_json() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("items"),
        columns: Some(vec!["tags".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Array(vec![
            Value::Str("python".into()),
            Value::Str("rust".into()),
        ]))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"INSERT INTO "items" ("tags") VALUES (?)"#);
    assert_eq!(params, vec![Value::Str(r#"["python", "rust"]"#.into())]);
}

#[test]
fn insert_nested_array_as_json() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("t"),
        columns: Some(vec!["data".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Array(vec![
            Value::Int(1),
            Value::Array(vec![Value::Int(2), Value::Int(3)]),
        ]))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"INSERT INTO "t" ("data") VALUES (?)"#);
    assert_eq!(params, vec![Value::Str("[1, [2, 3]]".into())]);
}
