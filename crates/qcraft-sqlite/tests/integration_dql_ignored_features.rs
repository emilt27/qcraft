//! Integration tests for DQL features that are silently ignored
//! by the SQLite renderer (PG-specific features that don't cause errors).

use qcraft_core::ast::common::*;
use qcraft_core::ast::conditions::*;
use qcraft_core::ast::expr::*;
use qcraft_core::ast::query::*;
use qcraft_core::ast::value::Value;
use qcraft_sqlite::SqliteRenderer;
use rusqlite::Connection;
use rusqlite::types::ToSql as RusqliteToSql;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn render(stmt: &QueryStmt) -> (String, Vec<Value>) {
    let renderer = SqliteRenderer::new();
    renderer.render_query_stmt(stmt).unwrap()
}

fn to_sqlite_params(values: &[Value]) -> Vec<Box<dyn RusqliteToSql>> {
    values
        .iter()
        .map(|v| -> Box<dyn RusqliteToSql> {
            match v {
                Value::Null => Box::new(rusqlite::types::Null),
                Value::Bool(b) => Box::new(*b),
                Value::Int(n) => Box::new(*n),
                Value::Float(f) => Box::new(*f),
                Value::Str(s) => Box::new(s.clone()),
                _ => Box::new(format!("{:?}", v)),
            }
        })
        .collect()
}

fn as_sqlite_params(boxed: &[Box<dyn RusqliteToSql>]) -> Vec<&dyn RusqliteToSql> {
    boxed.iter().map(|b| b.as_ref()).collect()
}

fn setup_db(conn: &Connection) {
    conn.execute_batch(
        "
        CREATE TABLE \"users\" (
            \"id\" INTEGER PRIMARY KEY,
            \"name\" TEXT NOT NULL,
            \"email\" TEXT UNIQUE,
            \"age\" INTEGER,
            \"active\" INTEGER NOT NULL DEFAULT 1,
            \"department\" TEXT
        );

        INSERT INTO \"users\" VALUES (1, 'Alice', 'alice@example.com', 30, 1, 'engineering');
        INSERT INTO \"users\" VALUES (2, 'Bob', 'bob@example.com', 25, 1, 'engineering');
        INSERT INTO \"users\" VALUES (3, 'Charlie', 'charlie@example.com', 35, 0, 'sales');
    ",
    )
    .unwrap();
}

fn conn() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    setup_db(&db);
    db
}

fn simple_query() -> QueryStmt {
    QueryStmt {
        ctes: None,
        columns: vec![SelectColumn::Star(None)],
        distinct: None,
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        joins: None,
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
    }
}

// ---------------------------------------------------------------------------
// ONLY (PG-specific) — silently ignored
// ---------------------------------------------------------------------------

#[test]
fn from_only_ignored() {
    let db = conn();
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("users")),
            only: true, // PG-specific, should be ignored
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    // ONLY should not appear in the rendered SQL
    assert!(!sql.contains("ONLY"));
    // Should still execute correctly
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 3);
}

// ---------------------------------------------------------------------------
// CTE MATERIALIZED hint — silently ignored
// ---------------------------------------------------------------------------

#[test]
fn cte_materialized_ignored() {
    let db = conn();
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "active_users".into(),
            query: Box::new(QueryStmt {
                where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
                    Comparison {
                        left: Expr::Field(FieldRef::new("users", "active")),
                        op: CompareOp::Eq,
                        right: Expr::Value(Value::Int(1)),
                        negate: false,
                    },
                ))])),
                ..simple_query()
            }),
            recursive: false,
            column_names: None,
            materialized: Some(CteMaterialized::Materialized),
        }]),
        from: Some(vec![FromItem::table(SchemaRef::new("active_users"))]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    // MATERIALIZED hint should not appear
    assert!(!sql.contains("MATERIALIZED"));
    // Should still execute correctly
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2); // Alice and Bob are active
}
