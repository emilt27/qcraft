//! Integration tests for PostgreSQL DML (INSERT / UPDATE / DELETE) rendering
//! executed against a real PostgreSQL instance via testcontainers.

use postgres::{Client, NoTls};
use testcontainers::runners::SyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;

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

fn connect() -> (impl std::any::Any, Client) {
    let node = Postgres::default().with_tag("16-alpine").start().unwrap();
    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        node.get_host().unwrap(),
        node.get_host_port_ipv4(5432).unwrap(),
    );
    let client = Client::connect(&conn_str, NoTls).unwrap();
    (node, client)
}

// ==========================================================================
// INSERT — basic
// ==========================================================================

#[test]
fn insert_single_row() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT)",
            &[],
        )
        .unwrap();

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
    client.execute(&render(&stmt), &[]).unwrap();

    let rows = client.query("SELECT name, email FROM users", &[]).unwrap();
    assert_eq!(rows.len(), 1);
    let name: &str = rows[0].get(0);
    let email: &str = rows[0].get(1);
    assert_eq!(name, "Alice");
    assert_eq!(email, "alice@example.com");
}

#[test]
fn insert_multi_row() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)", &[])
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into()]),
        source: InsertSource::Values(vec![
            vec![Expr::Value(Value::Str("Alice".into()))],
            vec![Expr::Value(Value::Str("Bob".into()))],
            vec![Expr::Value(Value::Str("Charlie".into()))],
        ]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 3);
}

#[test]
fn insert_default_values() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE counters (id SERIAL PRIMARY KEY, value INTEGER DEFAULT 0)",
            &[],
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
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one("SELECT id, value FROM counters", &[])
        .unwrap();
    let id: i32 = row.get(0);
    let value: i32 = row.get(1);
    assert_eq!(id, 1);
    assert_eq!(value, 0);
}

#[test]
fn insert_no_columns() {
    let (_node, mut client) = connect();
    client
        .execute("CREATE TABLE t (a INTEGER, b TEXT)", &[])
        .unwrap();

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
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client.query_one("SELECT a, b FROM t", &[]).unwrap();
    let a: i32 = row.get(0);
    let b: &str = row.get(1);
    assert_eq!(a, 1);
    assert_eq!(b, "x");
}

// ==========================================================================
// INSERT — RETURNING
// ==========================================================================

#[test]
fn insert_returning_star() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();

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
    let rows = client.query(&render(&stmt), &[]).unwrap();
    assert_eq!(rows.len(), 1);
    let id: i32 = rows[0].get(0);
    let name: &str = rows[0].get(1);
    assert_eq!(id, 1);
    assert_eq!(name, "Alice");
}

#[test]
fn insert_returning_columns() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();

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
    let rows = client.query(&render(&stmt), &[]).unwrap();
    assert_eq!(rows.len(), 1);
    let id: i32 = rows[0].get(0);
    let user_name: &str = rows[0].get(1);
    assert_eq!(id, 1);
    assert_eq!(user_name, "Alice");
}

// ==========================================================================
// INSERT — ON CONFLICT
// ==========================================================================

#[test]
fn insert_on_conflict_do_nothing() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, email TEXT UNIQUE NOT NULL, name TEXT)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO users (email, name) VALUES ('a@b.com', 'Alice')",
            &[],
        )
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["email".into(), "name".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("a@b.com".into())),
            Expr::Value(Value::Str("Bob".into())),
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
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    let name: String = client
        .query_one("SELECT name FROM users WHERE email = 'a@b.com'", &[])
        .unwrap()
        .get(0);
    assert_eq!(name, "Alice");
}

#[test]
fn insert_on_conflict_do_update() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, email TEXT UNIQUE NOT NULL, name TEXT)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO users (email, name) VALUES ('a@b.com', 'Alice')",
            &[],
        )
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["email".into(), "name".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("a@b.com".into())),
            Expr::Value(Value::Str("Bob".into())),
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
                        sql: "EXCLUDED.\"name\"".into(),
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
    client.execute(&render(&stmt), &[]).unwrap();

    let name: String = client
        .query_one("SELECT name FROM users WHERE email = 'a@b.com'", &[])
        .unwrap()
        .get(0);
    assert_eq!(name, "Bob");
}

#[test]
fn insert_on_conflict_on_constraint() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, email TEXT NOT NULL, CONSTRAINT uq_email UNIQUE (email))",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (email) VALUES ('a@b.com')", &[])
        .unwrap();

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
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}

#[test]
fn insert_on_conflict_do_update_with_where() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE counters (key TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO counters (key, value) VALUES ('hits', 999)",
            &[],
        )
        .unwrap();

    // value < 1000, so update should apply: 999 + 1 = 1000
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
                assignments: vec![(
                    "value".into(),
                    Expr::Raw {
                        sql: "\"counters\".\"value\" + EXCLUDED.\"value\"".into(),
                        params: vec![],
                    },
                )],
                where_clause: Some(Conditions {
                    children: vec![ConditionNode::Comparison(Comparison {
                        left: Expr::Raw {
                            sql: "\"counters\".\"value\"".into(),
                            params: vec![],
                        },
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
    client.execute(&render(&stmt), &[]).unwrap();

    let value: i32 = client
        .query_one("SELECT value FROM counters WHERE key = 'hits'", &[])
        .unwrap()
        .get(0);
    assert_eq!(value, 1000);

    // Now value = 1000, so WHERE counters.value < 1000 is false — no update
    client.execute(&render(&stmt), &[]).unwrap();
    let value: i32 = client
        .query_one("SELECT value FROM counters WHERE key = 'hits'", &[])
        .unwrap()
        .get(0);
    assert_eq!(value, 1000);
}

#[test]
fn insert_on_conflict_partial_index() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, email TEXT NOT NULL, active BOOLEAN NOT NULL DEFAULT TRUE)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "CREATE UNIQUE INDEX idx_email_active ON users (email) WHERE active = TRUE",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO users (email, active) VALUES ('a@b.com', TRUE)",
            &[],
        )
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["email".into(), "active".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("a@b.com".into())),
            Expr::Value(Value::Bool(true)),
        ]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Columns {
                columns: vec!["email".into()],
                where_clause: Some(Conditions {
                    children: vec![ConditionNode::Comparison(Comparison {
                        left: Expr::Raw {
                            sql: "\"active\"".into(),
                            params: vec![],
                        },
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
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}

#[test]
fn insert_overriding_system_value() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();

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
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one("SELECT id, name FROM users", &[])
        .unwrap();
    let id: i32 = row.get(0);
    let name: &str = row.get(1);
    assert_eq!(id, 100);
    assert_eq!(name, "Alice");
}

#[test]
fn insert_with_namespace() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
            &[],
        )
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users").with_namespace("public"),
        columns: Some(vec!["name".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Str("Alice".into()))]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let name: String = client
        .query_one("SELECT name FROM public.users", &[])
        .unwrap()
        .get(0);
    assert_eq!(name, "Alice");
}

#[test]
fn insert_with_expression() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE events (id SERIAL PRIMARY KEY, name TEXT NOT NULL, created_at TIMESTAMPTZ)",
            &[],
        )
        .unwrap();

    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("events"),
        columns: Some(vec!["name".into(), "created_at".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("login".into())),
            Expr::Func {
                name: "now".into(),
                args: vec![],
            },
        ]]),
        on_conflict: None,
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one("SELECT name, created_at IS NOT NULL AS has_ts FROM events", &[])
        .unwrap();
    let name: &str = row.get(0);
    let has_ts: bool = row.get(1);
    assert_eq!(name, "login");
    assert!(has_ts);
}

// ==========================================================================
// UPDATE — basic
// ==========================================================================

#[test]
fn update_simple() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (name) VALUES ('Alice'), ('Bob')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Charlie".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
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
    client.execute(&render(&stmt), &[]).unwrap();

    let name: String = client
        .query_one("SELECT name FROM users WHERE id = 1", &[])
        .unwrap()
        .get(0);
    assert_eq!(name, "Charlie");

    // Ensure the other row is untouched
    let name: String = client
        .query_one("SELECT name FROM users WHERE id = 2", &[])
        .unwrap()
        .get(0);
    assert_eq!(name, "Bob");
}

#[test]
fn update_multiple_assignments() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, age INTEGER NOT NULL DEFAULT 0)",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (name, age) VALUES ('Alice', 25)", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![
            ("name".into(), Expr::Value(Value::Str("Bob".into()))),
            ("age".into(), Expr::Value(Value::Int(30))),
        ],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
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
    client.execute(&render(&stmt), &[]).unwrap();

    let row = client
        .query_one("SELECT name, age FROM users WHERE id = 1", &[])
        .unwrap();
    let name: &str = row.get(0);
    let age: i32 = row.get(1);
    assert_eq!(name, "Bob");
    assert_eq!(age, 30);
}

#[test]
fn update_no_where() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO users (name) VALUES ('Alice'), ('Bob'), ('Charlie')",
            &[],
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
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM users WHERE name = 'Updated'", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 3);
}

#[test]
fn update_with_returning() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (name) VALUES ('Alice')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
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
    let rows = client.query(&render(&stmt), &[]).unwrap();
    assert_eq!(rows.len(), 1);
    let id: i32 = rows[0].get(0);
    let name: &str = rows[0].get(1);
    assert_eq!(id, 1);
    assert_eq!(name, "Bob");
}

#[test]
fn update_only() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE events (id SERIAL PRIMARY KEY, status TEXT NOT NULL DEFAULT 'active')",
            &[],
        )
        .unwrap();
    client
        .execute(
            "CREATE TABLE child_events () INHERITS (events)",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO events (status) VALUES ('active')", &[])
        .unwrap();
    client
        .execute("INSERT INTO child_events (status) VALUES ('active')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("events"),
        assignments: vec![(
            "status".into(),
            Expr::Value(Value::Str("archived".into())),
        )],
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
    client.execute(&render(&stmt), &[]).unwrap();

    // Parent row should be updated
    let status: String = client
        .query_one("SELECT status FROM ONLY events", &[])
        .unwrap()
        .get(0);
    assert_eq!(status, "archived");

    // Child row should be unaffected
    let status: String = client
        .query_one("SELECT status FROM child_events", &[])
        .unwrap()
        .get(0);
    assert_eq!(status, "active");
}

#[test]
fn update_with_from() {
    use rquery_core::ast::query::TableSource;

    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "CREATE TABLE orders (id SERIAL PRIMARY KEY, user_id INTEGER NOT NULL, status TEXT NOT NULL DEFAULT 'pending')",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (name) VALUES ('Alice')", &[])
        .unwrap();
    client
        .execute(
            "INSERT INTO orders (user_id, status) VALUES (1, 'pending')",
            &[],
        )
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("orders").with_alias("o"),
        assignments: vec![(
            "status".into(),
            Expr::Value(Value::Str("shipped".into())),
        )],
        from: Some(vec![TableSource::Table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
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
    client.execute(&render(&stmt), &[]).unwrap();

    let status: String = client
        .query_one("SELECT status FROM orders WHERE id = 1", &[])
        .unwrap()
        .get(0);
    assert_eq!(status, "shipped");
}

#[test]
fn update_with_expression() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE counters (key TEXT PRIMARY KEY, value INTEGER NOT NULL DEFAULT 0)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO counters (key, value) VALUES ('hits', 10)",
            &[],
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
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"key\"".into(),
                    params: vec![],
                },
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
    client.execute(&render(&stmt), &[]).unwrap();

    let value: i32 = client
        .query_one("SELECT value FROM counters WHERE key = 'hits'", &[])
        .unwrap()
        .get(0);
    assert_eq!(value, 11);
}

#[test]
fn update_with_alias() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (name) VALUES ('Alice')", &[])
        .unwrap();

    let stmt = MutationStmt::Update(UpdateStmt {
        table: SchemaRef::new("users").with_alias("u"),
        assignments: vec![("name".into(), Expr::Value(Value::Str("Bob".into())))],
        from: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"u\".\"id\"".into(),
                    params: vec![],
                },
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
    client.execute(&render(&stmt), &[]).unwrap();

    let name: String = client
        .query_one("SELECT name FROM users WHERE id = 1", &[])
        .unwrap()
        .get(0);
    assert_eq!(name, "Bob");
}

// ==========================================================================
// DELETE — basic
// ==========================================================================

#[test]
fn delete_simple() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (name) VALUES ('Alice'), ('Bob')", &[])
        .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"id\"".into(),
                    params: vec![],
                },
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
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    let name: String = client
        .query_one("SELECT name FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(name, "Bob");
}

#[test]
fn delete_no_where() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO users (name) VALUES ('Alice'), ('Bob'), ('Charlie')",
            &[],
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
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn delete_with_returning() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, active BOOLEAN NOT NULL DEFAULT TRUE)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "INSERT INTO users (name, active) VALUES ('Alice', TRUE), ('Bob', FALSE), ('Charlie', FALSE)",
            &[],
        )
        .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"active\"".into(),
                    params: vec![],
                },
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
    let rows = client.query(&render(&stmt), &[]).unwrap();
    assert_eq!(rows.len(), 2);

    let remaining: i64 = client
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(remaining, 1);
}

#[test]
fn delete_only() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE events (id SERIAL PRIMARY KEY, status TEXT NOT NULL DEFAULT 'active')",
            &[],
        )
        .unwrap();
    client
        .execute("CREATE TABLE child_events () INHERITS (events)", &[])
        .unwrap();
    client
        .execute("INSERT INTO events (status) VALUES ('active')", &[])
        .unwrap();
    client
        .execute("INSERT INTO child_events (status) VALUES ('active')", &[])
        .unwrap();

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
    client.execute(&render(&stmt), &[]).unwrap();

    // Parent rows deleted
    let parent_count: i64 = client
        .query_one("SELECT COUNT(*) FROM ONLY events", &[])
        .unwrap()
        .get(0);
    assert_eq!(parent_count, 0);

    // Child rows unaffected
    let child_count: i64 = client
        .query_one("SELECT COUNT(*) FROM child_events", &[])
        .unwrap()
        .get(0);
    assert_eq!(child_count, 1);
}

#[test]
fn delete_with_using() {
    use rquery_core::ast::query::TableSource;

    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute(
            "CREATE TABLE orders (id SERIAL PRIMARY KEY, user_id INTEGER NOT NULL, status TEXT NOT NULL DEFAULT 'pending')",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (name) VALUES ('Alice')", &[])
        .unwrap();
    client
        .execute(
            "INSERT INTO orders (user_id, status) VALUES (1, 'pending'), (1, 'shipped')",
            &[],
        )
        .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("orders").with_alias("o"),
        using: Some(vec![TableSource::Table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
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
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM orders", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn delete_with_alias() {
    let (_node, mut client) = connect();
    client
        .execute(
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    client
        .execute("INSERT INTO users (name) VALUES ('Alice'), ('Bob')", &[])
        .unwrap();

    let stmt = MutationStmt::Delete(DeleteStmt {
        table: SchemaRef::new("users").with_alias("u"),
        using: None,
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Comparison(Comparison {
                left: Expr::Raw {
                    sql: "\"u\".\"id\"".into(),
                    params: vec![],
                },
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
    client.execute(&render(&stmt), &[]).unwrap();

    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    let name: String = client
        .query_one("SELECT name FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(name, "Bob");
}
