use rquery_core::ast::common::{FieldRef, SchemaRef};
use rquery_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use rquery_core::ast::dml::*;
use rquery_core::ast::expr::Expr;
use rquery_core::ast::query::SelectColumn;
use rquery_core::ast::value::Value;
use rquery_postgres::PostgresRenderer;

fn render(stmt: &MutationStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _) = renderer.render_mutation_stmt(stmt).unwrap();
    sql
}

fn render_with_params(stmt: &MutationStmt) -> (String, Vec<Value>) {
    let renderer = PostgresRenderer::new();
    renderer.render_mutation_stmt(stmt).unwrap()
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
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("name", "email") VALUES('Alice', 'alice@example.com')"#,
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
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("name") VALUES('Alice'), ('Bob')"#,
    );
}

#[test]
fn insert_with_params() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "age".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice".into())),
            Expr::Value(Value::Int(30)),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    let (sql, _params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"INSERT INTO "users"("name", "age") VALUES('Alice', 30)"#,
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
fn insert_no_columns() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("t"),
        columns: None,
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Int(1)),
            Expr::Value(Value::Str("x".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    assert_eq!(render(&stmt), r#"INSERT INTO "t" VALUES(1, 'x')"#);
}

#[test]
fn insert_with_namespace() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users").with_namespace("public"),
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
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "public"."users"("id") VALUES(1)"#,
    );
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
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("name") VALUES('Alice') RETURNING *"#,
    );
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
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("name") VALUES('Alice') RETURNING "users"."id", "users"."name" AS "user_name""#,
    );
}

// ==========================================================================
// INSERT — ON CONFLICT
// ==========================================================================

#[test]
fn insert_on_conflict_do_nothing() {
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
            action: ConflictAction::DoNothing,
        }]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("email", "name") VALUES('a@b.com', 'Alice') ON CONFLICT("email") DO NOTHING"#,
    );
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
                assignments: vec![
                    ("name".into(), Expr::Raw { sql: "EXCLUDED.\"name\"".into(), params: vec![] }),
                ],
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
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("email", "name") VALUES('a@b.com', 'Alice') ON CONFLICT("email") DO UPDATE SET "name" = EXCLUDED."name""#,
    );
}

#[test]
fn insert_on_conflict_on_constraint() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["email".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("a@b.com".into()))]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Constraint("uq_email".into())),
            action: ConflictAction::DoNothing,
        }]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("email") VALUES('a@b.com') ON CONFLICT ON CONSTRAINT "uq_email" DO NOTHING"#,
    );
}

#[test]
fn insert_on_conflict_do_update_with_where() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("counters"),
        columns: Some(vec!["key".into(), "value".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("hits".into())),
            Expr::Value(Value::Int(1)),
        ]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Columns {
                columns: vec!["key".into()],
                where_clause: None,
            }),
            action: ConflictAction::DoUpdate {
                assignments: vec![
                    ("value".into(), Expr::Raw {
                        sql: "\"counters\".\"value\" + EXCLUDED.\"value\"".into(),
                        params: vec![],
                    }),
                ],
                where_clause: Some(Conditions {
                    children: vec![ConditionNode::Comparison(Comparison {
                        left: Expr::Raw { sql: "\"counters\".\"value\"".into(), params: vec![] },
                        op: CompareOp::Lt,
                        right: Expr::Value(Value::Int(1000)),
                        negate: false,
                    })],
                    connector: Connector::And,
                    negated: false,
                }),
            },
        }]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "counters"("key", "value") VALUES('hits', 1) ON CONFLICT("key") DO UPDATE SET "value" = "counters"."value" + EXCLUDED."value" WHERE "counters"."value" < 1000"#,
    );
}

#[test]
fn insert_on_conflict_partial_index() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["email".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("a@b.com".into()))]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Columns {
                columns: vec!["email".into()],
                where_clause: Some(Conditions {
                    children: vec![ConditionNode::Comparison(Comparison {
                        left: Expr::Raw { sql: "\"active\"".into(), params: vec![] },
                        op: CompareOp::Eq,
                        right: Expr::Value(Value::Bool(true)),
                        negate: false,
                    })],
                    connector: Connector::And,
                    negated: false,
                }),
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
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("email") VALUES('a@b.com') ON CONFLICT("email") WHERE "active" = TRUE DO NOTHING"#,
    );
}

// ==========================================================================
// INSERT — OVERRIDING
// ==========================================================================

#[test]
fn insert_overriding_system_value() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["id".into(), "name".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Int(100)),
            Expr::Value(Value::Str("Alice".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: Some(OverridingKind::System),
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("id", "name") OVERRIDING SYSTEM VALUE VALUES(100, 'Alice')"#,
    );
}

// ==========================================================================
// UPDATE — basic
// ==========================================================================

#[test]
fn update_simple() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![
            ("name".into(), Expr::Value(Value::Str("Bob".into()))),
        ],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"id\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
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
    assert_eq!(
        render(&stmt),
        r#"UPDATE "users" SET "name" = 'Bob' WHERE "id" = 1"#,
    );
}

#[test]
fn update_multiple_assignments() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![
            ("name".into(), Expr::Value(Value::Str("Bob".into()))),
            ("age".into(), Expr::Value(Value::Int(30))),
        ],
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
    assert_eq!(
        render(&stmt),
        r#"UPDATE "users" SET "name" = 'Bob', "age" = 30"#,
    );
}

#[test]
fn update_with_returning() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![
            ("name".into(), Expr::Value(Value::Str("Bob".into()))),
        ],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"id\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
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
    assert_eq!(
        render(&stmt),
        r#"UPDATE "users" SET "name" = 'Bob' WHERE "id" = 1 RETURNING *"#,
    );
}

#[test]
fn update_only() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("events"),
        assignments: vec![
            ("status".into(), Expr::Value(Value::Str("archived".into()))),
        ],
        from: None,
        where_clause: None,
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: None,
        limit: None,
        offset: None,
        only: true,
        partition: None,
        ignore: false,
    });
    assert_eq!(
        render(&stmt),
        r#"UPDATE ONLY "events" SET "status" = 'archived'"#,
    );
}

#[test]
fn update_with_alias() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users").with_alias("u"),
        assignments: vec![
            ("name".into(), Expr::Value(Value::Str("Bob".into()))),
        ],
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
    assert_eq!(
        render(&stmt),
        r#"UPDATE "users" AS "u" SET "name" = 'Bob'"#,
    );
}

#[test]
fn update_with_from() {
    use rquery_core::ast::query::TableSource;
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("orders").with_alias("o"),
        assignments: vec![
            ("status".into(), Expr::Value(Value::Str("shipped".into()))),
        ],
        from: Some(vec![TableSource::Table(SchemaRef::new("users").with_alias("u"))]),
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"o\".\"user_id\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Raw { sql: "\"u\".\"id\"".into(), params: vec![] },
                negate: false,
            })],
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
    // render_from for TableSource::Table renders the schema ref + alias
    // This test verifies FROM clause is rendered
    let sql = render(&stmt);
    assert!(sql.contains("FROM"), "expected FROM clause, got: {sql}");
    assert!(sql.contains(r#"SET "status" = 'shipped'"#), "expected SET clause, got: {sql}");
    assert!(sql.contains("WHERE"), "expected WHERE clause, got: {sql}");
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
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"id\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
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
    assert_eq!(
        render(&stmt),
        r#"DELETE FROM "users" WHERE "id" = 1"#,
    );
}

#[test]
fn delete_no_where() {
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("temp_data"),
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
    assert_eq!(render(&stmt), r#"DELETE FROM "temp_data""#);
}

#[test]
fn delete_with_returning() {
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"active\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(false)),
                negate: false,
            })],
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
    assert_eq!(
        render(&stmt),
        r#"DELETE FROM "users" WHERE "active" = FALSE RETURNING *"#,
    );
}

#[test]
fn delete_only() {
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("events"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: true,
        partition: None,
        ignore: false,
    });
    assert_eq!(render(&stmt), r#"DELETE FROM ONLY "events""#);
}

#[test]
fn delete_with_alias() {
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users").with_alias("u"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"u\".\"id\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })],
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
    assert_eq!(
        render(&stmt),
        r#"DELETE FROM "users" AS "u" WHERE "u"."id" = 1"#,
    );
}

#[test]
fn delete_with_using() {
    use rquery_core::ast::query::TableSource;
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("orders").with_alias("o"),
        using: Some(vec![TableSource::Table(SchemaRef::new("users").with_alias("u"))]),
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"o\".\"user_id\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Raw { sql: "\"u\".\"id\"".into(), params: vec![] },
                negate: false,
            })],
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
    let sql = render(&stmt);
    assert!(sql.contains("USING"), "expected USING clause, got: {sql}");
    assert!(sql.contains("WHERE"), "expected WHERE clause, got: {sql}");
}

// ==========================================================================
// INSERT — expression values (DEFAULT, functions)
// ==========================================================================

#[test]
fn insert_with_expression_values() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("events"),
        columns: Some(vec!["name".into(), "created_at".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("login".into())),
            Expr::Func { name: "now".into(), args: vec![] },
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "events"("name", "created_at") VALUES('login', now())"#,
    );
}

// ==========================================================================
// UPDATE — expression in SET
// ==========================================================================

#[test]
fn update_with_expression() {
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("counters"),
        assignments: vec![
            ("value".into(), Expr::Raw {
                sql: "\"value\" + 1".into(),
                params: vec![],
            }),
        ],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw { sql: "\"key\"".into(), params: vec![] },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("hits".into())),
                negate: false,
            })],
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
    assert_eq!(
        render(&stmt),
        r#"UPDATE "counters" SET "value" = "value" + 1 WHERE "key" = 'hits'"#,
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
                assignments: vec![
                    ("name".into(), Expr::Raw {
                        sql: "EXCLUDED.\"name\"".into(),
                        params: vec![],
                    }),
                ],
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
    assert_eq!(
        render(&stmt),
        r#"INSERT INTO "users"("email", "name") VALUES('alice@example.com', 'Alice') ON CONFLICT("email") DO UPDATE SET "name" = EXCLUDED."name" RETURNING *"#,
    );
}
