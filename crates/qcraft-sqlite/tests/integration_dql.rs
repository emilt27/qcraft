//! Integration tests for SQLite DQL (SELECT) statements
//! that run against a real in-memory SQLite database.

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
                Value::Bytes(b) => Box::new(b.clone()),
                Value::Date(s) | Value::DateTime(s) | Value::Time(s) => Box::new(s.clone()),
                Value::Decimal(s) => Box::new(s.clone()),
                Value::Uuid(s) => Box::new(s.clone()),
                Value::Json(s) | Value::Jsonb(s) => Box::new(s.clone()),
                Value::IpNetwork(s) => Box::new(s.clone()),
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
        CREATE TABLE \"orders\" (
            \"id\" INTEGER PRIMARY KEY,
            \"user_id\" INTEGER NOT NULL REFERENCES \"users\"(\"id\"),
            \"product\" TEXT NOT NULL,
            \"amount\" REAL NOT NULL,
            \"created_at\" TEXT NOT NULL
        );
        CREATE TABLE \"products\" (
            \"id\" INTEGER PRIMARY KEY,
            \"name\" TEXT NOT NULL,
            \"price\" REAL NOT NULL,
            \"category\" TEXT NOT NULL
        );

        INSERT INTO \"users\" VALUES (1, 'Alice', 'alice@example.com', 30, 1, 'engineering');
        INSERT INTO \"users\" VALUES (2, 'Bob', 'bob@example.com', 25, 1, 'engineering');
        INSERT INTO \"users\" VALUES (3, 'Charlie', 'charlie@example.com', 35, 0, 'sales');
        INSERT INTO \"users\" VALUES (4, 'Diana', 'diana@example.com', 28, 1, 'sales');
        INSERT INTO \"users\" VALUES (5, 'Eve', 'eve@example.com', NULL, 1, 'engineering');

        INSERT INTO \"orders\" VALUES (1, 1, 'Widget', 10.50, '2024-01-15');
        INSERT INTO \"orders\" VALUES (2, 1, 'Gadget', 25.00, '2024-01-20');
        INSERT INTO \"orders\" VALUES (3, 2, 'Widget', 10.50, '2024-02-01');
        INSERT INTO \"orders\" VALUES (4, 4, 'Gizmo', 50.00, '2024-02-15');
        INSERT INTO \"orders\" VALUES (5, 4, 'Widget', 10.50, '2024-03-01');

        INSERT INTO \"products\" VALUES (1, 'Widget', 10.50, 'hardware');
        INSERT INTO \"products\" VALUES (2, 'Gadget', 25.00, 'electronics');
        INSERT INTO \"products\" VALUES (3, 'Gizmo', 50.00, 'electronics');
        INSERT INTO \"products\" VALUES (4, 'Doohickey', 5.00, 'hardware');
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
// SELECT basics
// ---------------------------------------------------------------------------

#[test]
fn select_star() {
    let db = conn();
    let (sql, values) = render(&simple_query());
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut stmt = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = stmt
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn select_columns() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
        ],
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(i64, String)> = st
        .query_map(params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
    assert!(rows.iter().any(|(id, name)| *id == 1 && name == "Alice"));
    assert!(rows.iter().any(|(id, name)| *id == 2 && name == "Bob"));
}

#[test]
fn select_with_alias() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: Some("user_name".into()),
        }],
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let col_name = st.column_name(0).unwrap().to_string();
    assert_eq!(col_name, "user_name");
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn select_expr() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Func {
                name: "COUNT".into(),
                args: vec![Expr::Field(FieldRef::new("users", "id"))],
            },
            alias: Some("cnt".into()),
        }],
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let cnt: i64 = db
        .query_row(&sql, params.as_slice(), |row| row.get(0))
        .unwrap();
    assert_eq!(cnt, 5);
}

#[test]
fn select_table_star() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(Some("u".into()))],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn select_no_from() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: None,
        }],
        from: None,
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let val: i64 = db
        .query_row(&sql, params.as_slice(), |row| row.get(0))
        .unwrap();
    assert_eq!(val, 1);
}

#[test]
fn select_distinct() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "department"),
            alias: None,
        }],
        distinct: Some(DistinctDef::Distinct),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.contains(&"engineering".to_string()));
    assert!(rows.contains(&"sales".to_string()));
}

// ---------------------------------------------------------------------------
// FROM
// ---------------------------------------------------------------------------

#[test]
fn from_with_namespace() {
    let db = conn();
    let stmt = QueryStmt {
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_namespace("main"),
        )]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn from_multiple_tables() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![
            FromItem::table(SchemaRef::new("users")),
            FromItem::table(SchemaRef::new("orders")),
        ]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "id")),
                op: CompareOp::Eq,
                right: Expr::Field(FieldRef::new("orders", "user_id")),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn from_subquery() {
    let db = conn();
    let inner = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let stmt = QueryStmt {
        from: Some(vec![FromItem::subquery(inner, "sub".into())]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn from_table_function() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Function {
                name: "json_each".into(),
                args: vec![Expr::Value(Value::Str("[1,2,3]".into()))],
                alias: Some("j".into()),
            },
            only: false,
            sample: None,
            index_hint: None,
        }]),
        joins: None,
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
        ctes: None,
        distinct: None,
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get::<_, i64>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn from_values() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Values {
                rows: vec![
                    vec![
                        Expr::Value(Value::Int(1)),
                        Expr::Value(Value::Str("a".into())),
                    ],
                    vec![
                        Expr::Value(Value::Int(2)),
                        Expr::Value(Value::Str("b".into())),
                    ],
                ],
                alias: "t".into(),
                column_aliases: None,
            },
            only: false,
            sample: None,
            index_hint: None,
        }]),
        joins: None,
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
        ctes: None,
        distinct: None,
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(i64, String)> = st
        .query_map(params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], (1, "a".to_string()));
    assert_eq!(rows[1], (2, "b".to_string()));
}

// ---------------------------------------------------------------------------
// WHERE
// ---------------------------------------------------------------------------

#[test]
fn where_simple() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn where_and() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![
            ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            })),
            ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("engineering".into())),
                negate: false,
            })),
        ])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows.contains(&"Alice".to_string()));
    assert!(rows.contains(&"Bob".to_string()));
    assert!(rows.contains(&"Eve".to_string()));
}

#[test]
fn where_or() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::or(vec![
            ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("engineering".into())),
                negate: false,
            })),
            ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("sales".into())),
                negate: false,
            })),
        ])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn where_comparison_operators() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(28)),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, i64)> = st
        .query_map(params.as_slice(), |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(3)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(name, age)| name == "Alice" && *age == 30));
    assert!(
        rows.iter()
            .any(|(name, age)| name == "Charlie" && *age == 35)
    );
}

#[test]
fn where_is_null() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::IsNull,
                right: Expr::Value(Value::Null),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], "Eve");
}

#[test]
fn where_is_not_null() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::IsNull,
                right: Expr::Value(Value::Null),
                negate: true,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn where_like() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "name")),
                op: CompareOp::Like,
                right: Expr::Value(Value::Str("A%".into())),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], "Alice");
}

// ---------------------------------------------------------------------------
// Contains / StartsWith / EndsWith (integration)
// ---------------------------------------------------------------------------

#[test]
fn where_contains_filters_correctly() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        where_clause: Some(Conditions::contains(FieldRef::new("users", "name"), "li")),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["Alice", "Charlie"]);
}

#[test]
fn where_starts_with_filters_correctly() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        where_clause: Some(Conditions::starts_with(
            FieldRef::new("users", "name"),
            "Al",
        )),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["Alice"]);
}

#[test]
fn where_ends_with_filters_correctly() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        where_clause: Some(Conditions::ends_with(
            FieldRef::new("users", "name"),
            "ob",
        )),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["Bob"]);
}

#[test]
fn where_icontains_case_insensitive() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        where_clause: Some(Conditions::icontains(
            FieldRef::new("users", "name"),
            "ALICE",
        )),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["Alice"]);
}

#[test]
fn where_istarts_with_case_insensitive() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        where_clause: Some(Conditions::istarts_with(
            FieldRef::new("users", "name"),
            "aLi",
        )),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["Alice"]);
}

#[test]
fn where_iends_with_case_insensitive() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        where_clause: Some(Conditions::iends_with(
            FieldRef::new("users", "name"),
            "BOB",
        )),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["Bob"]);
}

#[test]
fn where_contains_escapes_percent_in_db() {
    let db = conn();
    db.execute(
        "INSERT INTO \"users\" VALUES (10, '50% off', 'promo@test.com', 20, 1, 'sales')",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO \"users\" VALUES (11, '500 items', 'items@test.com', 20, 1, 'sales')",
        [],
    )
    .unwrap();

    // "50%" should match "50% off" but NOT "500 items"
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        where_clause: Some(Conditions::contains(
            FieldRef::new("users", "name"),
            "50%",
        )),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["50% off"]);
}

#[test]
fn where_contains_escapes_underscore_in_db() {
    let db = conn();
    db.execute(
        "INSERT INTO \"users\" VALUES (12, 'user_admin', 'ua@test.com', 20, 1, 'engineering')",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO \"users\" VALUES (13, 'useradmin', 'uadmin@test.com', 20, 1, 'engineering')",
        [],
    )
    .unwrap();

    // "user_" should match "user_admin" but NOT "useradmin"
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        where_clause: Some(Conditions::starts_with(
            FieldRef::new("users", "name"),
            "user_",
        )),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["user_admin"]);
}

#[test]
fn where_contains_no_match() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        where_clause: Some(Conditions::contains(
            FieldRef::new("users", "name"),
            "zzzzz",
        )),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 0);
}

#[test]
fn where_between() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::Between,
                right: Expr::Raw {
                    sql: "25 AND 30".into(),
                    params: vec![],
                },
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn where_in_list() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::In,
                right: Expr::Raw {
                    sql: "('engineering', 'sales')".into(),
                    params: vec![],
                },
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn where_negated() {
    let db = conn();
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(0)),
                negate: true,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 4);
}

// ---------------------------------------------------------------------------
// JOINs
// ---------------------------------------------------------------------------

#[test]
fn inner_join() {
    let db = conn();
    let stmt = QueryStmt {
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders")),
            condition: Some(JoinCondition::On(Conditions::and(vec![
                ConditionNode::Comparison(Box::new(Comparison {
                    left: Expr::Field(FieldRef::new("users", "id")),
                    op: CompareOp::Eq,
                    right: Expr::Field(FieldRef::new("orders", "user_id")),
                    negate: false,
                })),
            ]))),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // Users with orders: Alice(2), Bob(1), Diana(2) = 5 rows
    assert_eq!(rows.len(), 5);
}

#[test]
fn left_join() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("orders", "id"),
                alias: Some("order_id".into()),
            },
        ],
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders")),
            condition: Some(JoinCondition::On(Conditions::and(vec![
                ConditionNode::Comparison(Box::new(Comparison {
                    left: Expr::Field(FieldRef::new("users", "id")),
                    op: CompareOp::Eq,
                    right: Expr::Field(FieldRef::new("orders", "user_id")),
                    negate: false,
                })),
            ]))),
            join_type: JoinType::Left,
            natural: false,
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, Option<i64>)> = st
        .query_map(params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // 5 orders + Charlie(NULL) + Eve(NULL) = 7 rows
    assert_eq!(rows.len(), 7);
    // Charlie and Eve have no orders
    assert!(
        rows.iter()
            .any(|(name, oid)| name == "Charlie" && oid.is_none())
    );
    assert!(
        rows.iter()
            .any(|(name, oid)| name == "Eve" && oid.is_none())
    );
}

#[test]
fn cross_join() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("products"))]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders")),
            condition: None,
            join_type: JoinType::Cross,
            natural: false,
        }]),
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
        ctes: None,
        distinct: None,
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // 4 products * 5 orders = 20
    assert_eq!(rows.len(), 20);
}

#[test]
fn natural_join() {
    // NATURAL JOIN matches on columns with the same name.
    // "users" and "orders" share "id" column so NATURAL JOIN uses that.
    // We test that it doesn't error and produces results.
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders")),
            condition: None,
            join_type: JoinType::Inner,
            natural: true,
        }]),
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
        ctes: None,
        distinct: None,
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // NATURAL JOIN on "id" — matches users.id = orders.id: rows 1-5
    assert_eq!(rows.len(), 5);
}

#[test]
fn join_using() {
    // Create a temp table with user_id to test USING
    let db = conn();
    db.execute_batch(
        "
        CREATE TABLE \"user_prefs\" (
            \"user_id\" INTEGER NOT NULL,
            \"theme\" TEXT NOT NULL
        );
        INSERT INTO \"user_prefs\" VALUES (1, 'dark');
        INSERT INTO \"user_prefs\" VALUES (2, 'light');
    ",
    )
    .unwrap();

    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("orders"))]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("user_prefs")),
            condition: Some(JoinCondition::Using(vec!["user_id".into()])),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
        ctes: None,
        distinct: None,
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // Orders with user_id 1 (2 orders) and user_id 2 (1 order) = 3
    assert_eq!(rows.len(), 3);
}

// ---------------------------------------------------------------------------
// GROUP BY / HAVING
// ---------------------------------------------------------------------------

#[test]
fn group_by_simple() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "department"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Field(FieldRef::new("users", "id"))],
                },
                alias: Some("cnt".into()),
            },
        ],
        group_by: Some(vec![GroupByItem::Expr(Expr::Field(FieldRef::new(
            "users",
            "department",
        )))]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, i64)> = st
        .query_map(params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|(dept, cnt)| dept == "engineering" && *cnt == 3)
    );
    assert!(rows.iter().any(|(dept, cnt)| dept == "sales" && *cnt == 2));
}

#[test]
fn group_by_with_having() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "department"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Field(FieldRef::new("users", "id"))],
                },
                alias: Some("cnt".into()),
            },
        ],
        group_by: Some(vec![GroupByItem::Expr(Expr::Field(FieldRef::new(
            "users",
            "department",
        )))]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Field(FieldRef::new("users", "id"))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(2)),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, i64)> = st
        .query_map(params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // Only engineering has 3 members (> 2)
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "engineering");
    assert_eq!(rows[0].1, 3);
}

#[test]
fn group_by_aggregate_functions() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "department"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Value(Value::Int(1))],
                },
                alias: Some("cnt".into()),
            },
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "AVG".into(),
                    args: vec![Expr::Field(FieldRef::new("users", "age"))],
                },
                alias: Some("avg_age".into()),
            },
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "MAX".into(),
                    args: vec![Expr::Field(FieldRef::new("users", "age"))],
                },
                alias: Some("max_age".into()),
            },
        ],
        group_by: Some(vec![GroupByItem::Expr(Expr::Field(FieldRef::new(
            "users",
            "department",
        )))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "department")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, i64, f64, i64)> = st
        .query_map(params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    // engineering: Alice(30), Bob(25), Eve(NULL) → cnt=3, avg=27.5 (NULL excluded), max=30
    let eng = rows.iter().find(|(d, _, _, _)| d == "engineering").unwrap();
    assert_eq!(eng.1, 3);
    assert!((eng.2 - 27.5).abs() < 0.01);
    assert_eq!(eng.3, 30);
    // sales: Charlie(35), Diana(28) → cnt=2, avg=31.5, max=35
    let sales = rows.iter().find(|(d, _, _, _)| d == "sales").unwrap();
    assert_eq!(sales.1, 2);
    assert!((sales.2 - 31.5).abs() < 0.01);
    assert_eq!(sales.3, 35);
}

// ---------------------------------------------------------------------------
// ORDER BY
// ---------------------------------------------------------------------------

#[test]
fn order_by_asc() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows[0], "Alice");
}

#[test]
fn order_by_desc() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "name")),
            direction: OrderDir::Desc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<String> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows[0], "Eve");
}

#[test]
fn order_by_multiple() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "department"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
        ],
        order_by: Some(vec![
            OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "department")),
                direction: OrderDir::Asc,
                nulls: None,
            },
            OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "name")),
                direction: OrderDir::Desc,
                nulls: None,
            },
        ]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, String)> = st
        .query_map(params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // engineering first (ASC), names DESC within: Eve, Bob, Alice
    assert_eq!(rows[0], ("engineering".to_string(), "Eve".to_string()));
    assert_eq!(rows[1], ("engineering".to_string(), "Bob".to_string()));
    assert_eq!(rows[2], ("engineering".to_string(), "Alice".to_string()));
    // sales next: Diana, Charlie
    assert_eq!(rows[3], ("sales".to_string(), "Diana".to_string()));
    assert_eq!(rows[4], ("sales".to_string(), "Charlie".to_string()));
}

#[test]
fn order_by_nulls_last() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "age"),
                alias: None,
            },
        ],
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "age")),
            direction: OrderDir::Asc,
            nulls: Some(NullsOrder::Last),
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, Option<i64>)> = st
        .query_map(params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // Eve (NULL age) should be last
    assert_eq!(rows.last().unwrap().0, "Eve");
    assert!(rows.last().unwrap().1.is_none());
}

// ---------------------------------------------------------------------------
// LIMIT / OFFSET
// ---------------------------------------------------------------------------

#[test]
fn limit_only() {
    let db = conn();
    let stmt = QueryStmt {
        limit: Some(LimitDef {
            kind: LimitKind::Limit(2),
            offset: None,
        }),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn limit_offset() {
    let db = conn();
    let stmt = QueryStmt {
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(2),
            offset: Some(2),
        }),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    // Ordered by id ASC, offset 2 → ids 3, 4
    assert_eq!(rows[0], 3);
    assert_eq!(rows[1], 4);
}

#[test]
fn fetch_first_converts_to_limit() {
    let db = conn();
    let stmt = QueryStmt {
        limit: Some(LimitDef {
            kind: LimitKind::FetchFirst {
                count: 3,
                with_ties: false,
                percent: false,
            },
            offset: None,
        }),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
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
// CTE
// ---------------------------------------------------------------------------

#[test]
fn cte_simple() {
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
            materialized: None,
        }]),
        from: Some(vec![FromItem::table(SchemaRef::new("active_users"))]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn cte_recursive() {
    let db = conn();
    let base_query = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: Some("n".into()),
        }],
        from: None,
        ..simple_query()
    };
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "nums".into(),
            query: Box::new(base_query),
            recursive: true,
            column_names: Some(vec!["n".into()]),
            materialized: None,
        }]),
        from: Some(vec![FromItem::table(SchemaRef::new("nums"))]),
        ..simple_query()
    };
    // The unit test shows this renders as:
    // WITH RECURSIVE "nums" ("n") AS (SELECT 1 AS "n") SELECT * FROM "nums"
    // But to actually recurse, we need a UNION ALL with the recursive step.
    // The base unit test only tests rendering, not execution.
    // For a proper recursive CTE we use raw SQL to verify the concept works,
    // but the AST-based approach here only defines the base case.
    // Let's verify it at least runs and returns the base case.
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // Only the base case (1 row) since there's no UNION ALL recursive step in the AST
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], 1);
}

// ---------------------------------------------------------------------------
// Set Operations
// ---------------------------------------------------------------------------

#[test]
fn union_all() {
    let db = conn();
    let left = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
        ],
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
        ],
        ..simple_query()
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::SetOp(Box::new(SetOpDef {
                left: Box::new(left),
                right: Box::new(right),
                operation: SetOperationType::UnionAll,
            })),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        joins: None,
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
        ctes: None,
        distinct: None,
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // 5 + 5 = 10
    assert_eq!(rows.len(), 10);
}

#[test]
fn union_distinct() {
    let db = conn();
    let left = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
        ],
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
        ],
        ..simple_query()
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::SetOp(Box::new(SetOpDef {
                left: Box::new(left),
                right: Box::new(right),
                operation: SetOperationType::Union,
            })),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        joins: None,
        where_clause: None,
        group_by: None,
        having: None,
        window: None,
        order_by: None,
        limit: None,
        lock: None,
        ctes: None,
        distinct: None,
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<i64> = st
        .query_map(params.as_slice(), |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // UNION deduplicates: 5 unique users
    assert_eq!(rows.len(), 5);
}

// ---------------------------------------------------------------------------
// WINDOW functions
// ---------------------------------------------------------------------------

#[test]
fn window_row_number() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Window(WindowDef {
                    expression: Box::new(Expr::Func {
                        name: "ROW_NUMBER".into(),
                        args: vec![],
                    }),
                    partition_by: None,
                    order_by: Some(vec![OrderByDef {
                        expr: Expr::Field(FieldRef::new("users", "id")),
                        direction: OrderDir::Asc,
                        nulls: None,
                    }]),
                    frame: None,
                }),
                alias: Some("rn".into()),
            },
        ],
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(i64, String, i64)> = st
        .query_map(params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
    for (i, (_, _, rn)) in rows.iter().enumerate() {
        assert_eq!(*rn, (i + 1) as i64);
    }
}

#[test]
fn window_partition_by() {
    let db = conn();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "department"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Window(WindowDef {
                    expression: Box::new(Expr::Func {
                        name: "ROW_NUMBER".into(),
                        args: vec![],
                    }),
                    partition_by: Some(vec![Expr::Field(FieldRef::new("users", "department"))]),
                    order_by: Some(vec![OrderByDef {
                        expr: Expr::Field(FieldRef::new("users", "name")),
                        direction: OrderDir::Asc,
                        nulls: None,
                    }]),
                    frame: None,
                }),
                alias: Some("rn".into()),
            },
        ],
        order_by: Some(vec![
            OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "department")),
                direction: OrderDir::Asc,
                nulls: None,
            },
            OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "name")),
                direction: OrderDir::Asc,
                nulls: None,
            },
        ]),
        ..simple_query()
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, String, i64)> = st
        .query_map(params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
    // engineering: Alice=1, Bob=2, Eve=3
    let eng: Vec<_> = rows.iter().filter(|(d, _, _)| d == "engineering").collect();
    assert_eq!(eng.len(), 3);
    assert_eq!(eng[0], &("engineering".to_string(), "Alice".to_string(), 1));
    assert_eq!(eng[1], &("engineering".to_string(), "Bob".to_string(), 2));
    assert_eq!(eng[2], &("engineering".to_string(), "Eve".to_string(), 3));
    // sales: Charlie=1, Diana=2
    let sales: Vec<_> = rows.iter().filter(|(d, _, _)| d == "sales").collect();
    assert_eq!(sales.len(), 2);
    assert_eq!(sales[0], &("sales".to_string(), "Charlie".to_string(), 1));
    assert_eq!(sales[1], &("sales".to_string(), "Diana".to_string(), 2));
}

// ---------------------------------------------------------------------------
// Complex queries
// ---------------------------------------------------------------------------

#[test]
fn full_query_with_join_group_having_order_limit() {
    let db = conn();
    let stmt = QueryStmt {
        ctes: None,
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("u", "name"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Field(FieldRef::new("o", "id"))],
                },
                alias: Some("order_count".into()),
            },
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "SUM".into(),
                    args: vec![Expr::Field(FieldRef::new("o", "amount"))],
                },
                alias: Some("total_amount".into()),
            },
        ],
        distinct: None,
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders").with_alias("o")),
            condition: Some(JoinCondition::On(Conditions::and(vec![
                ConditionNode::Comparison(Box::new(Comparison {
                    left: Expr::Field(FieldRef::new("u", "id")),
                    op: CompareOp::Eq,
                    right: Expr::Field(FieldRef::new("o", "user_id")),
                    negate: false,
                })),
            ]))),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("u", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            },
        ))])),
        group_by: Some(vec![GroupByItem::Expr(Expr::Field(FieldRef::new(
            "u", "name",
        )))]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Field(FieldRef::new("o", "id"))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            },
        ))])),
        window: None,
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("u", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(10),
            offset: None,
        }),
        lock: None,
    };
    let (sql, values) = render(&stmt);
    let boxed = to_sqlite_params(&values);
    let params = as_sqlite_params(&boxed);
    let mut st = db.prepare(&sql).unwrap();
    let rows: Vec<(String, i64, f64)> = st
        .query_map(params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // Active users with > 1 order: Alice (2 orders), Diana (2 orders)
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "Alice");
    assert_eq!(rows[0].1, 2);
    assert!((rows[0].2 - 35.5).abs() < 0.01); // 10.50 + 25.00
    assert_eq!(rows[1].0, "Diana");
    assert_eq!(rows[1].1, 2);
    assert!((rows[1].2 - 60.5).abs() < 0.01); // 50.00 + 10.50
}
