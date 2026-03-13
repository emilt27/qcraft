//! Integration tests for SQLite DML (INSERT / UPDATE / DELETE) statements
//! that run against a real in-memory SQLite database.

use qcraft_core::ast::common::{FieldRef, SchemaRef};
use qcraft_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions, Connector};
use qcraft_core::ast::dml::*;
use qcraft_core::ast::expr::Expr;
use qcraft_core::ast::query::SelectColumn;
use qcraft_core::ast::value::Value;
use qcraft_sqlite::SqliteRenderer;
use rusqlite::Connection;
mod common;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn conn() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn render(stmt: &MutationStmt) -> (String, Vec<Value>) {
    let renderer = SqliteRenderer::new();
    renderer.render_mutation_stmt(stmt).unwrap()
}

fn setup_users(conn: &Connection) {
    conn.execute(
        r#"CREATE TABLE "users" ("id" INTEGER PRIMARY KEY, "name" TEXT NOT NULL, "email" TEXT UNIQUE)"#,
        [],
    )
    .unwrap();
}

fn count_rows(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM \"{table}\""), [], |row| {
        row.get(0)
    })
    .unwrap()
}

// ==========================================================================
// INSERT — basic
// ==========================================================================

#[test]
fn insert_single_row() {
    let c = conn();
    setup_users(&c);

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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let (name, email): (String, String) = c
        .query_row(
            r#"SELECT "name", "email" FROM "users" WHERE rowid = 1"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "Alice");
    assert_eq!(email, "alice@example.com");
}

#[test]
fn insert_multi_row() {
    let c = conn();
    setup_users(&c);

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![
            vec![
                Expr::Value(Value::Str("Alice".into())),
                Expr::Value(Value::Str("alice@example.com".into())),
            ],
            vec![
                Expr::Value(Value::Str("Bob".into())),
                Expr::Value(Value::Str("bob@example.com".into())),
            ],
        ]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 2);
}

#[test]
fn insert_default_values() {
    let c = conn();
    c.execute(
        r#"CREATE TABLE "counters" ("id" INTEGER PRIMARY KEY, "value" INTEGER DEFAULT 0)"#,
        [],
    )
    .unwrap();

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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let val: i64 = c
        .query_row(r#"SELECT "value" FROM "counters""#, [], |row| row.get(0))
        .unwrap();
    assert_eq!(val, 0);
}

#[test]
fn insert_returning_star() {
    let c = conn();
    setup_users(&c);

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice".into())),
            Expr::Value(Value::Str("alice@example.com".into())),
        ]]),
        on_conflict: None,
        returning: Some(vec![SelectColumn::Star(None)]),
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    let (id, name, email): (i64, String, String) = c
        .query_row(&sql, params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(id, 1);
    assert_eq!(name, "Alice");
    assert_eq!(email, "alice@example.com");
}

#[test]
fn insert_returning_columns() {
    let c = conn();
    setup_users(&c);

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice".into())),
            Expr::Value(Value::Str("alice@example.com".into())),
        ]]),
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    let (id, name): (i64, String) = c
        .query_row(&sql, params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(id, 1);
    assert_eq!(name, "Alice");
}

#[test]
fn insert_or_replace() {
    let c = conn();
    setup_users(&c);

    // Insert original row
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();

    // INSERT OR REPLACE with same PK, different data
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["id".into(), "name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Int(1)),
            Expr::Value(Value::Str("Bob".into())),
            Expr::Value(Value::Str("bob@example.com".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: Some(ConflictResolution::Replace),
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let name: String = c
        .query_row(r#"SELECT "name" FROM "users" WHERE "id" = 1"#, [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(name, "Bob");
    assert_eq!(count_rows(&c, "users"), 1);
}

#[test]
fn insert_or_ignore() {
    let c = conn();
    setup_users(&c);

    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["id".into(), "name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Int(1)),
            Expr::Value(Value::Str("Bob".into())),
            Expr::Value(Value::Str("bob@example.com".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: Some(ConflictResolution::Ignore),
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    // First row should remain unchanged
    let name: String = c
        .query_row(r#"SELECT "name" FROM "users" WHERE "id" = 1"#, [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(name, "Alice");
    assert_eq!(count_rows(&c, "users"), 1);
}

#[test]
fn insert_or_abort() {
    let c = conn();
    setup_users(&c);

    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();

    // Start a transaction
    c.execute("BEGIN", []).unwrap();
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (2, 'Charlie', 'charlie@example.com')"#,
        [],
    )
    .unwrap();

    // INSERT OR ABORT with conflicting PK — should error
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["id".into(), "name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Int(1)),
            Expr::Value(Value::Str("Dup".into())),
            Expr::Value(Value::Str("dup@example.com".into())),
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: Some(ConflictResolution::Abort),
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    let result = c.execute(&sql, params.as_slice());
    assert!(result.is_err(), "expected conflict error");

    // Transaction should still be usable after OR ABORT
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (3, 'Dave', 'dave@example.com')"#,
        [],
    )
    .unwrap();
    c.execute("COMMIT", []).unwrap();

    // Alice (id=1) stays, Charlie (id=2) committed, Dave (id=3) committed
    assert_eq!(count_rows(&c, "users"), 3);
}

#[test]
fn insert_on_conflict_do_nothing() {
    let c = conn();
    setup_users(&c);

    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Bob".into())),
            Expr::Value(Value::Str("alice@example.com".into())),
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    // Original row stays
    let name: String = c
        .query_row(
            r#"SELECT "name" FROM "users" WHERE "email" = 'alice@example.com'"#,
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "Alice");
    assert_eq!(count_rows(&c, "users"), 1);
}

#[test]
fn insert_on_conflict_do_update() {
    let c = conn();
    setup_users(&c);

    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice Updated".into())),
            Expr::Value(Value::Str("alice@example.com".into())),
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let name: String = c
        .query_row(r#"SELECT "name" FROM "users" WHERE "id" = 1"#, [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(name, "Alice Updated");
    assert_eq!(count_rows(&c, "users"), 1);
}

#[test]
fn insert_on_conflict_catch_all() {
    let c = conn();
    c.execute(r#"CREATE TABLE "t" ("id" INTEGER PRIMARY KEY)"#, [])
        .unwrap();
    c.execute(r#"INSERT INTO "t" ("id") VALUES (1)"#, [])
        .unwrap();

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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    // Original row stays, no error
    assert_eq!(count_rows(&c, "t"), 1);
}

#[test]
fn insert_with_namespace() {
    let c = conn();
    setup_users(&c);

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users").with_namespace("main"),
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 1);
}

#[test]
fn insert_bool_as_integer() {
    let c = conn();
    c.execute(
        r#"CREATE TABLE "flags" ("id" INTEGER PRIMARY KEY, "active" INTEGER NOT NULL)"#,
        [],
    )
    .unwrap();

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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let val: i64 = c
        .query_row(r#"SELECT "active" FROM "flags""#, [], |row| row.get(0))
        .unwrap();
    assert_eq!(val, 1);
}

// ==========================================================================
// UPDATE — basic
// ==========================================================================

#[test]
fn update_simple() {
    let c = conn();
    setup_users(&c);
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();

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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let name: String = c
        .query_row(r#"SELECT "name" FROM "users" WHERE "id" = 1"#, [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(name, "Bob");
}

#[test]
fn update_multiple_assignments() {
    let c = conn();
    setup_users(&c);
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![
            ("name".into(), Expr::Value(Value::Str("Bob".into()))),
            (
                "email".into(),
                Expr::Value(Value::Str("bob@example.com".into())),
            ),
        ],
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let (name, email): (String, String) = c
        .query_row(
            r#"SELECT "name", "email" FROM "users" WHERE "id" = 1"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "Bob");
    assert_eq!(email, "bob@example.com");
}

#[test]
fn update_no_where() {
    let c = conn();
    setup_users(&c);
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'a@a.com'), (2, 'Bob', 'b@b.com')"#,
        [],
    )
    .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Updated".into())))],
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let names: Vec<String> = {
        let mut s = c.prepare(r#"SELECT "name" FROM "users""#).unwrap();
        s.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert!(names.iter().all(|n| n == "Updated"));
    assert_eq!(names.len(), 2);
}

#[test]
fn update_with_returning() {
    let c = conn();
    setup_users(&c);
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'alice@example.com')"#,
        [],
    )
    .unwrap();

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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    let (id, name, email): (i64, String, String) = c
        .query_row(&sql, params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(id, 1);
    assert_eq!(name, "Bob");
    assert_eq!(email, "alice@example.com");
}

#[test]
fn update_or_replace() {
    let c = conn();
    c.execute(
        r#"CREATE TABLE "kv" ("key" TEXT PRIMARY KEY, "value" TEXT NOT NULL)"#,
        [],
    )
    .unwrap();
    c.execute(
        r#"INSERT INTO "kv" VALUES ('a', 'alpha'), ('b', 'beta')"#,
        [],
    )
    .unwrap();

    // UPDATE OR REPLACE: update key 'b' to 'a' — replaces existing 'a' row
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("kv"),
        assignments: vec![("key".into(), Expr::Value(Value::Str("a".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Raw {
                    sql: "\"key\"".into(),
                    params: vec![],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("b".into())),
                negate: false,
            }))],
            connector: Connector::And,
            negated: false,
        }),
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    // Only one row should remain with key='a', value='beta'
    assert_eq!(count_rows(&c, "kv"), 1);
    let val: String = c
        .query_row(r#"SELECT "value" FROM "kv" WHERE "key" = 'a'"#, [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(val, "beta");
}

#[test]
fn update_with_limit_offset() {
    use qcraft_core::ast::common::OrderByDef;

    // NOTE: UPDATE ... ORDER BY ... LIMIT requires SQLITE_ENABLE_UPDATE_DELETE_LIMIT
    // which is not enabled in the bundled SQLite. We verify the rendered SQL instead.
    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("logs"),
        assignments: vec![("archived".into(), Expr::Value(Value::Bool(true)))],
        from: None,
        where_clause: None,
        returning: None,
        ctes: None,
        conflict_resolution: None,
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("logs", "id")),
            direction: qcraft_core::ast::common::OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(2),
        offset: Some(1),
        only: false,
        partition: None,
        ignore: false,
    });

    let (sql, params) = render(&stmt);
    assert_eq!(
        sql,
        r#"UPDATE "logs" SET "archived" = ? ORDER BY "logs"."id" ASC LIMIT 2 OFFSET 1"#,
    );
    assert_eq!(params, vec![Value::Bool(true)]);
}

#[test]
fn update_with_expression() {
    let c = conn();
    c.execute(
        r#"CREATE TABLE "counters" ("id" INTEGER PRIMARY KEY, "value" INTEGER NOT NULL)"#,
        [],
    )
    .unwrap();
    c.execute(
        r#"INSERT INTO "counters" ("id", "value") VALUES (1, 10)"#,
        [],
    )
    .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("counters"),
        assignments: vec![(
            "value".into(),
            Expr::Raw {
                sql: "\"value\" + 1".into(),
                params: vec![],
            },
        )],
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();

    let val: i64 = c
        .query_row(
            r#"SELECT "value" FROM "counters" WHERE "id" = 1"#,
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(val, 11);
}

// ==========================================================================
// DELETE — basic
// ==========================================================================

#[test]
fn delete_simple() {
    let c = conn();
    setup_users(&c);
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'a@a.com'), (2, 'Bob', 'b@b.com')"#,
        [],
    )
    .unwrap();

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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 1);

    let name: String = c
        .query_row(r#"SELECT "name" FROM "users""#, [], |row| row.get(0))
        .unwrap();
    assert_eq!(name, "Bob");
}

#[test]
fn delete_no_where() {
    let c = conn();
    setup_users(&c);
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'a@a.com'), (2, 'Bob', 'b@b.com')"#,
        [],
    )
    .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
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

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    c.execute(&sql, params.as_slice()).unwrap();
    assert_eq!(count_rows(&c, "users"), 0);
}

#[test]
fn delete_with_returning() {
    let c = conn();
    setup_users(&c);
    c.execute(
        r#"INSERT INTO "users" ("id", "name", "email") VALUES (1, 'Alice', 'a@a.com'), (2, 'Bob', 'b@b.com')"#,
        [],
    )
    .unwrap();

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
        returning: Some(vec![SelectColumn::Star(None)]),
        ctes: None,
        order_by: None,
        limit: None,
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    let (id, name, email): (i64, String, String) = c
        .query_row(&sql, params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(id, 1);
    assert_eq!(name, "Alice");
    assert_eq!(email, "a@a.com");

    // Verify the row was actually deleted
    assert_eq!(count_rows(&c, "users"), 1);
}

#[test]
fn delete_with_limit() {
    use qcraft_core::ast::common::OrderByDef;

    // NOTE: DELETE ... ORDER BY ... LIMIT requires SQLITE_ENABLE_UPDATE_DELETE_LIMIT
    // which is not enabled in the bundled SQLite. We verify the rendered SQL instead.
    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("logs"),
        using: None,
        where_clause: None,
        returning: None,
        ctes: None,
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("logs", "id")),
            direction: qcraft_core::ast::common::OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(2),
        offset: None,
        only: false,
        partition: None,
        ignore: false,
    });

    assert_eq!(
        render(&stmt).0,
        r#"DELETE FROM "logs" ORDER BY "logs"."id" ASC LIMIT 2"#,
    );
}

#[test]
fn insert_timedelta_into_text_column() {
    let db = conn();
    db.execute_batch("CREATE TABLE events (id INTEGER PRIMARY KEY, duration TEXT)")
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("events"),
        columns: Some(vec!["id".into(), "duration".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Int(1)),
            Expr::Value(Value::TimeDelta {
                years: 0,
                months: 0,
                days: 0,
                seconds: 9015,
                microseconds: 0,
            }),
        ]]),
        returning: None,
        on_conflict: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });

    let (sql, values) = render(&stmt);
    // TimeDelta should be in params, caller converts to string before executing
    let boxed = common::to_sqlite_params(&values);
    let params = common::as_sqlite_params(&boxed);
    db.execute(&sql, params.as_slice()).unwrap();

    let stored: String = db
        .query_row("SELECT duration FROM events WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(!stored.is_empty());
}
