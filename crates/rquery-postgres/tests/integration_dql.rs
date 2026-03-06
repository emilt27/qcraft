//! Integration tests for PostgreSQL DQL (SELECT queries) rendering
//! executed against a real PostgreSQL instance via testcontainers.
//!
//! All tests are read-only (SELECT), so they share a single database
//! with seed data. No per-test DB cloning needed.

use std::sync::LazyLock;

use postgres::{Client, NoTls};
use testcontainers::runners::SyncRunner;
use testcontainers::ImageExt;
use testcontainers_modules::postgres::Postgres;

use rquery_core::ast::common::*;
use rquery_core::ast::conditions::*;
use rquery_core::ast::expr::*;
use rquery_core::ast::query::*;
use rquery_core::ast::value::Value;
use rquery_postgres::PostgresRenderer;

fn render(stmt: &QueryStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _) = renderer.render_query_stmt(stmt).unwrap();
    sql
}

fn simple_query() -> QueryStmt {
    QueryStmt {
        ctes: None,
        columns: vec![],
        distinct: None,
        from: None,
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

fn simple_cond_eq(left: Expr, right: Expr) -> Conditions {
    Conditions::and(vec![ConditionNode::Comparison(Comparison {
        left,
        op: CompareOp::Eq,
        right,
        negate: false,
    })])
}

struct TestDb {
    host: String,
    port: u16,
    _container: Box<dyn std::any::Any + Send + Sync>,
}

static TEST_DB: LazyLock<TestDb> = LazyLock::new(|| {
    let node = Postgres::default().with_tag("16-alpine").start().unwrap();
    let host = node.get_host().unwrap().to_string();
    let port = node.get_host_port_ipv4(5432).unwrap();

    let conn_str = format!(
        "host={host} port={port} user=postgres password=postgres dbname=postgres"
    );
    let mut client = Client::connect(&conn_str, NoTls).unwrap();

    client.batch_execute(
        "
        CREATE TABLE \"users\" (
            \"id\" INTEGER PRIMARY KEY,
            \"name\" TEXT NOT NULL,
            \"email\" TEXT UNIQUE,
            \"age\" INTEGER,
            \"active\" BOOLEAN NOT NULL DEFAULT TRUE,
            \"department\" TEXT
        );
        CREATE TABLE \"orders\" (
            \"id\" INTEGER PRIMARY KEY,
            \"user_id\" INTEGER NOT NULL REFERENCES \"users\"(\"id\"),
            \"product\" TEXT NOT NULL,
            \"amount\" NUMERIC(10,2) NOT NULL,
            \"created_at\" DATE NOT NULL
        );
        CREATE TABLE \"products\" (
            \"id\" INTEGER PRIMARY KEY,
            \"name\" TEXT NOT NULL,
            \"price\" NUMERIC(10,2) NOT NULL,
            \"category\" TEXT NOT NULL
        );

        INSERT INTO \"users\" VALUES (1, 'Alice', 'alice@example.com', 30, TRUE, 'engineering');
        INSERT INTO \"users\" VALUES (2, 'Bob', 'bob@example.com', 25, TRUE, 'engineering');
        INSERT INTO \"users\" VALUES (3, 'Charlie', 'charlie@example.com', 35, FALSE, 'sales');
        INSERT INTO \"users\" VALUES (4, 'Diana', 'diana@example.com', 28, TRUE, 'sales');
        INSERT INTO \"users\" VALUES (5, 'Eve', 'eve@example.com', NULL, TRUE, 'engineering');

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
    drop(client);

    TestDb {
        host,
        port,
        _container: Box::new(node),
    }
});

fn test_client() -> Client {
    let db = &*TEST_DB;

    let conn_str = format!(
        "host={} port={} user=postgres password=postgres dbname=postgres",
        db.host, db.port
    );
    Client::connect(&conn_str, NoTls).unwrap()
}

// ==========================================================================
// SELECT basics
// ==========================================================================

#[test]
fn select_star() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn select_columns() {
    let mut client = test_client();
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
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].get::<_, i32>(0), 1);
    assert_eq!(rows[0].get::<_, String>(1), "Alice");
    assert_eq!(rows[4].get::<_, i32>(0), 5);
    assert_eq!(rows[4].get::<_, String>(1), "Eve");
}

#[test]
fn select_with_alias() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: Some("user_name".into()),
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].get::<_, String>(0), "Alice");
}

#[test]
fn select_expr() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Func {
                name: "COUNT".into(),
                args: vec![Expr::Field(FieldRef::new("users", "id"))],
            },
            alias: Some("cnt".into()),
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, i64>(0), 5);
}

#[test]
fn select_table_star() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(Some("u".into()))],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn select_no_from() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: None,
        }],
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, i32>(0), 1);
}

#[test]
fn select_distinct() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "department"),
            alias: None,
        }],
        distinct: Some(DistinctDef::Distinct),
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn select_distinct_on() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        distinct: Some(DistinctDef::DistinctOn(vec![Expr::Field(FieldRef::new(
            "users",
            "department",
        ))])),
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![
            OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "department")),
                direction: OrderDir::Asc,
                nulls: None,
            },
            OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "id")),
                direction: OrderDir::Asc,
                nulls: None,
            },
        ]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // 2 departments → 2 rows (first row per department)
    assert_eq!(rows.len(), 2);
}

// ==========================================================================
// FROM
// ==========================================================================

#[test]
fn from_with_schema() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_namespace("public"),
        )]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn from_multiple_tables() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![
            FromItem::table(SchemaRef::new("users").with_alias("u")),
            FromItem::table(SchemaRef::new("orders").with_alias("o")),
        ]),
        where_clause: Some(simple_cond_eq(
            Expr::Field(FieldRef::new("u", "id")),
            Expr::Field(FieldRef::new("o", "user_id")),
        )),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Same as inner join: 5 matching order rows
    assert_eq!(rows.len(), 5);
}

#[test]
fn from_subquery() {
    let mut client = test_client();
    let inner = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::subquery(inner, "sub".into())]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn from_function() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Function {
                name: "generate_series".into(),
                args: vec![Expr::Value(Value::Int(1)), Expr::Value(Value::Int(5))],
                alias: Some("t".into()),
            },
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn from_values() {
    let mut client = test_client();
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
                column_aliases: Some(vec!["id".into(), "name".into()]),
            },
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<_, i32>(0), 1);
    assert_eq!(rows[0].get::<_, &str>(1), "a");
}

#[test]
fn from_only() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("users")),
            only: true,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    // ONLY on non-inherited table just returns the same rows
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn from_tablesample() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("users")),
            only: false,
            sample: Some(TableSampleDef {
                method: SampleMethod::Bernoulli,
                percentage: 100.0,
                seed: None,
            }),
            index_hint: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // BERNOULLI(100) should return all rows
    assert_eq!(rows.len(), 5);
}

// ==========================================================================
// WHERE
// ==========================================================================

#[test]
fn where_simple() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn where_and() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![
            ConditionNode::Comparison(Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            }),
            ConditionNode::Comparison(Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("engineering".into())),
                negate: false,
            }),
        ])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Alice, Bob, Eve are engineering + active
    assert_eq!(rows.len(), 3);
}

#[test]
fn where_or() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::or(vec![
            ConditionNode::Comparison(Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("engineering".into())),
                negate: false,
            }),
            ConditionNode::Comparison(Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("sales".into())),
                negate: false,
            }),
        ])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn where_comparison() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(28)),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Alice (30), Charlie (35)
    assert_eq!(rows.len(), 2);
}

#[test]
fn where_is_null() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::IsNull,
                right: Expr::Value(Value::Null),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Eve has NULL age
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, String>(1), "Eve");
}

#[test]
fn where_like() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "name")),
                op: CompareOp::Like,
                right: Expr::Value(Value::Str("A%".into())),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, String>(1), "Alice");
}

#[test]
fn where_between() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::Between,
                right: Expr::Raw {
                    sql: "25 AND 30".into(),
                    params: vec![],
                },
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Bob (25), Diana (28), Alice (30)
    assert_eq!(rows.len(), 3);
}

#[test]
fn where_in_list() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::In,
                right: Expr::Raw {
                    sql: "('engineering')".into(),
                    params: vec![],
                },
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Alice, Bob, Eve
    assert_eq!(rows.len(), 3);
}

#[test]
fn where_negated() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(
            Conditions::and(vec![ConditionNode::Comparison(Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(false)),
                negate: false,
            })])
            .negated(),
        ),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // NOT (active = FALSE) → 4 rows (all except Charlie)
    assert_eq!(rows.len(), 4);
}

// ==========================================================================
// JOINs
// ==========================================================================

#[test]
fn inner_join() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders").with_alias("o")),
            condition: Some(JoinCondition::On(simple_cond_eq(
                Expr::Field(FieldRef::new("u", "id")),
                Expr::Field(FieldRef::new("o", "user_id")),
            ))),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn left_join() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("u", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("u", "name"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("o", "id"),
                alias: Some("order_id".into()),
            },
        ],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders").with_alias("o")),
            condition: Some(JoinCondition::On(simple_cond_eq(
                Expr::Field(FieldRef::new("u", "id")),
                Expr::Field(FieldRef::new("o", "user_id")),
            ))),
            join_type: JoinType::Left,
            natural: false,
        }]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("u", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // 5 orders + Charlie (no orders) + Eve (no orders) = 7
    assert_eq!(rows.len(), 7);
    // Last rows should be users with no orders (NULL order_id)
    let null_order_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.get::<_, Option<i32>>(2).is_none())
        .collect();
    assert_eq!(null_order_rows.len(), 2);
}

#[test]
fn right_join() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("orders").with_alias("o"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("users").with_alias("u")),
            condition: Some(JoinCondition::On(simple_cond_eq(
                Expr::Field(FieldRef::new("o", "user_id")),
                Expr::Field(FieldRef::new("u", "id")),
            ))),
            join_type: JoinType::Right,
            natural: false,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // All users appear; users without orders get NULLs for order cols
    assert_eq!(rows.len(), 7);
}

#[test]
fn full_join() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders").with_alias("o")),
            condition: Some(JoinCondition::On(simple_cond_eq(
                Expr::Field(FieldRef::new("u", "id")),
                Expr::Field(FieldRef::new("o", "user_id")),
            ))),
            join_type: JoinType::Full,
            natural: false,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // All matched + unmatched from both sides = 7
    assert_eq!(rows.len(), 7);
}

#[test]
fn cross_join() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("products").with_alias("p")),
            condition: None,
            join_type: JoinType::Cross,
            natural: false,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // 5 users * 4 products = 20
    assert_eq!(rows.len(), 20);
}

#[test]
fn natural_join() {
    let mut client = test_client();
    // NATURAL JOIN on products and orders — they share "id" column
    // We use a subquery to control column overlap
    let left = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: Some("key".into()),
        }],
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: Some("key".into()),
        }],
        ..simple_query()
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::subquery(left, "a".into())]),
        joins: Some(vec![JoinDef {
            source: FromItem::subquery(right, "b".into()),
            condition: None,
            join_type: JoinType::Inner,
            natural: true,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Both produce key=1, NATURAL JOIN matches → 1 row
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, i32>(0), 1);
}

#[test]
fn join_using() {
    let mut client = test_client();
    // Rename user_id to id via subquery, then JOIN USING (id)
    let orders_sub = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("orders", "user_id"),
                alias: Some("id".into()),
            },
            SelectColumn::Field {
                field: FieldRef::new("orders", "product"),
                alias: None,
            },
        ],
        from: Some(vec![FromItem::table(SchemaRef::new("orders"))]),
        ..simple_query()
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::subquery(orders_sub, "o".into()),
            condition: Some(JoinCondition::Using(vec!["id".into()])),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn lateral_join() {
    let mut client = test_client();
    let inner = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("orders"))]),
        where_clause: Some(simple_cond_eq(
            Expr::Field(FieldRef::new("orders", "user_id")),
            Expr::Field(FieldRef::new("u", "id")),
        )),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(1),
            offset: None,
        }),
        ..simple_query()
    };
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("u", "name"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("lo", "product"),
                alias: None,
            },
        ],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem {
                source: TableSource::Lateral(Box::new(FromItem::subquery(
                    inner,
                    "lo".into(),
                ))),
                only: false,
                sample: None,
                index_hint: None,
            },
            condition: Some(JoinCondition::On(Conditions::and(vec![
                ConditionNode::Comparison(Comparison {
                    left: Expr::Value(Value::Bool(true)),
                    op: CompareOp::Eq,
                    right: Expr::Value(Value::Bool(true)),
                    negate: false,
                }),
            ]))),
            join_type: JoinType::Left,
            natural: false,
        }]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("u", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // All 5 users appear (LEFT JOIN), some with NULL product
    assert_eq!(rows.len(), 5);
}

// ==========================================================================
// GROUP BY / HAVING
// ==========================================================================

#[test]
fn group_by_simple() {
    let mut client = test_client();
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
        ],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
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
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<_, String>(0), "engineering");
    assert_eq!(rows[0].get::<_, i64>(1), 3);
    assert_eq!(rows[1].get::<_, String>(0), "sales");
    assert_eq!(rows[1].get::<_, i64>(1), 2);
}

#[test]
fn group_by_having() {
    let mut client = test_client();
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
        ],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        group_by: Some(vec![GroupByItem::Expr(Expr::Field(FieldRef::new(
            "users",
            "department",
        )))]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Value(Value::Int(1))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(2)),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Only engineering has > 2 users
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, String>(0), "engineering");
}

#[test]
fn group_by_rollup() {
    let mut client = test_client();
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
        ],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        group_by: Some(vec![GroupByItem::Rollup(vec![Expr::Field(FieldRef::new(
            "users",
            "department",
        ))])]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // 2 departments + 1 totals row = 3
    assert_eq!(rows.len(), 3);
}

#[test]
fn group_by_cube() {
    let mut client = test_client();
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
        ],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        group_by: Some(vec![GroupByItem::Cube(vec![Expr::Field(FieldRef::new(
            "users",
            "department",
        ))])]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // CUBE on single column = same as ROLLUP: 2 + 1 = 3
    assert_eq!(rows.len(), 3);
}

#[test]
fn group_by_grouping_sets() {
    let mut client = test_client();
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
        ],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        group_by: Some(vec![GroupByItem::GroupingSets(vec![
            vec![Expr::Field(FieldRef::new("users", "department"))],
            vec![], // grand total
        ])]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // 2 department groups + 1 grand total = 3
    assert_eq!(rows.len(), 3);
}

// ==========================================================================
// ORDER BY
// ==========================================================================

#[test]
fn order_by_asc() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows[0].get::<_, String>(0), "Alice");
}

#[test]
fn order_by_desc() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "name")),
            direction: OrderDir::Desc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows[0].get::<_, String>(0), "Eve");
}

#[test]
fn order_by_nulls_first() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "age"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "age")),
            direction: OrderDir::Asc,
            nulls: Some(NullsOrder::First),
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Eve's age is NULL → first
    assert!(rows[0].get::<_, Option<i32>>(0).is_none());
}

#[test]
fn order_by_nulls_last() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "age"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "age")),
            direction: OrderDir::Asc,
            nulls: Some(NullsOrder::Last),
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Eve's NULL age → last
    assert!(rows[4].get::<_, Option<i32>>(0).is_none());
    assert_eq!(rows[0].get::<_, Option<i32>>(0), Some(25));
}

// ==========================================================================
// LIMIT / OFFSET
// ==========================================================================

#[test]
fn limit_only() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(2),
            offset: None,
        }),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn limit_offset() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "id"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
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
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 2);
    // ids 3 and 4 (offset 2 skips 1, 2)
    assert_eq!(rows[0].get::<_, i32>(0), 3);
    assert_eq!(rows[1].get::<_, i32>(0), 4);
}

#[test]
fn fetch_first_rows_only() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
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
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn fetch_first_with_ties() {
    let mut client = test_client();
    // All 5 users have distinct ages (or NULL), but we need WITH TIES to work
    // We use department for ordering: engineering=3, sales=2
    // FETCH FIRST 3 WITH TIES + ORDER BY department → should get at least 3
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "department")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(LimitDef {
            kind: LimitKind::FetchFirst {
                count: 3,
                with_ties: true,
                percent: false,
            },
            offset: None,
        }),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // First 3 are engineering, but the 3rd ties with others in engineering → 3 rows
    assert!(rows.len() >= 3);
}

// ==========================================================================
// CTE
// ==========================================================================

#[test]
fn cte_simple() {
    let mut client = test_client();
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "active_users".into(),
            query: Box::new(cte_query),
            recursive: false,
            column_names: None,
            materialized: None,
        }]),
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("active_users"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn cte_recursive() {
    let mut client = test_client();
    // WITH RECURSIVE nums(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM nums WHERE n < 5)
    // PG requires recursive CTE body = non-recursive UNION ALL recursive directly.
    // The AST wraps SetOp in a FROM, so we use Raw SQL for the CTE body and verify
    // the RECURSIVE keyword is rendered correctly by the renderer.
    let cte_body = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Raw {
                sql: "1 UNION ALL SELECT \"nums\".\"n\" + 1 FROM \"nums\" WHERE \"nums\".\"n\" < 5".into(),
                params: vec![],
            },
            alias: None,
        }],
        ..simple_query()
    };
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "nums".into(),
            query: Box::new(cte_body),
            recursive: true,
            column_names: Some(vec!["n".into()]),
            materialized: None,
        }]),
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("nums"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("WITH RECURSIVE"));
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn cte_materialized() {
    let mut client = test_client();
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "cached".into(),
            query: Box::new(cte_query),
            recursive: false,
            column_names: None,
            materialized: Some(CteMaterialized::Materialized),
        }]),
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("cached"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
}

// ==========================================================================
// Set Operations
// ==========================================================================

#[test]
fn union_all() {
    let mut client = test_client();
    let left = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let set_op = SetOpDef {
        left: Box::new(left),
        right: Box::new(right),
        operation: SetOperationType::UnionAll,
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::SetOp(Box::new(set_op)),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // 5 + 5 = 10
    assert_eq!(rows.len(), 10);
}

#[test]
fn union_distinct() {
    let mut client = test_client();
    let left = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let set_op = SetOpDef {
        left: Box::new(left),
        right: Box::new(right),
        operation: SetOperationType::Union,
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::SetOp(Box::new(set_op)),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Deduplicated: 5 unique names
    assert_eq!(rows.len(), 5);
}

#[test]
fn intersect() {
    let mut client = test_client();
    let left = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("engineering".into())),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let set_op = SetOpDef {
        left: Box::new(left),
        right: Box::new(right),
        operation: SetOperationType::Intersect,
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::SetOp(Box::new(set_op)),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Intersection: engineering users = 3
    assert_eq!(rows.len(), 3);
}

#[test]
fn except() {
    let mut client = test_client();
    let left = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "department")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("engineering".into())),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let set_op = SetOpDef {
        left: Box::new(left),
        right: Box::new(right),
        operation: SetOperationType::Except,
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::SetOp(Box::new(set_op)),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // All names minus engineering names = sales names = 2
    assert_eq!(rows.len(), 2);
}

// ==========================================================================
// WINDOW functions
// ==========================================================================

#[test]
fn window_row_number() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![
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
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].get::<_, i64>(1), 1);
    assert_eq!(rows[4].get::<_, i64>(1), 5);
}

#[test]
fn window_partition_by() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "department"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Window(WindowDef {
                    expression: Box::new(Expr::Func {
                        name: "ROW_NUMBER".into(),
                        args: vec![],
                    }),
                    partition_by: Some(vec![Expr::Field(FieldRef::new(
                        "users",
                        "department",
                    ))]),
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
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
    // First engineering user (Alice, id=1) should have rn=1
    assert_eq!(rows[0].get::<_, String>(0), "Alice");
    assert_eq!(rows[0].get::<_, i64>(2), 1);
}

#[test]
fn window_named() {
    let mut client = test_client();
    // Use WINDOW clause and reference it via Expr::Raw for OVER "w"
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Raw {
                    sql: "ROW_NUMBER() OVER \"w\"".into(),
                    params: vec![],
                },
                alias: Some("rn".into()),
            },
        ],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        window: Some(vec![WindowNameDef {
            name: "w".into(),
            base_window: None,
            partition_by: None,
            order_by: Some(vec![OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "id")),
                direction: OrderDir::Asc,
                nulls: None,
            }]),
            frame: None,
        }]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "id")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].get::<_, i64>(1), 1);
    assert_eq!(rows[4].get::<_, i64>(1), 5);
}

// ==========================================================================
// FOR UPDATE / FOR SHARE
// ==========================================================================

#[test]
fn for_update() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::Update,
            of: None,
            nowait: false,
            skip_locked: false,
            wait: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    // FOR UPDATE must be inside a transaction
    client
        .batch_execute("BEGIN")
        .unwrap();
    let rows = client.query(&sql, &[]).unwrap();
    client
        .batch_execute("COMMIT")
        .unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn for_share() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::Share,
            of: None,
            nowait: false,
            skip_locked: false,
            wait: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    client.batch_execute("BEGIN").unwrap();
    let rows = client.query(&sql, &[]).unwrap();
    client.batch_execute("COMMIT").unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn for_update_nowait() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::Update,
            of: None,
            nowait: true,
            skip_locked: false,
            wait: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("FOR UPDATE NOWAIT"));
    client.batch_execute("BEGIN").unwrap();
    let rows = client.query(&sql, &[]).unwrap();
    client.batch_execute("COMMIT").unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn for_update_skip_locked() {
    let mut client = test_client();
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::Update,
            of: None,
            nowait: false,
            skip_locked: true,
            wait: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("FOR UPDATE SKIP LOCKED"));
    client.batch_execute("BEGIN").unwrap();
    let rows = client.query(&sql, &[]).unwrap();
    client.batch_execute("COMMIT").unwrap();
    assert_eq!(rows.len(), 5);
}

// ==========================================================================
// Complex
// ==========================================================================

#[test]
fn full_pipeline() {
    let mut client = test_client();
    // WITH active AS (SELECT * FROM users WHERE active = TRUE)
    // SELECT u.name, COUNT(1) AS order_count
    // FROM active AS u
    // INNER JOIN orders AS o ON u.id = o.user_id
    // GROUP BY u.name
    // HAVING COUNT(1) >= 1
    // ORDER BY u.name ASC
    // LIMIT 10
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            },
        )])),
        ..simple_query()
    };
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "active".into(),
            query: Box::new(cte_query),
            recursive: false,
            column_names: None,
            materialized: None,
        }]),
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("u", "name"),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Value(Value::Int(1))],
                },
                alias: Some("order_count".into()),
            },
        ],
        from: Some(vec![FromItem::table(
            SchemaRef::new("active").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders").with_alias("o")),
            condition: Some(JoinCondition::On(simple_cond_eq(
                Expr::Field(FieldRef::new("u", "id")),
                Expr::Field(FieldRef::new("o", "user_id")),
            ))),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        group_by: Some(vec![GroupByItem::Expr(Expr::Field(FieldRef::new(
            "u", "name",
        )))]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(
            Comparison {
                left: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Value(Value::Int(1))],
                },
                op: CompareOp::Gte,
                right: Expr::Value(Value::Int(1)),
                negate: false,
            },
        )])),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("u", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(10),
            offset: None,
        }),
        where_clause: None,
        distinct: None,
        window: None,
        lock: None,
    };
    let sql = render(&stmt);
    let rows = client.query(&sql, &[]).unwrap();
    // Active users with orders: Alice (2 orders), Bob (1), Diana (2)
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].get::<_, String>(0), "Alice");
    assert_eq!(rows[0].get::<_, i64>(1), 2);
    assert_eq!(rows[1].get::<_, String>(0), "Bob");
    assert_eq!(rows[1].get::<_, i64>(1), 1);
    assert_eq!(rows[2].get::<_, String>(0), "Diana");
    assert_eq!(rows[2].get::<_, i64>(1), 2);
}
