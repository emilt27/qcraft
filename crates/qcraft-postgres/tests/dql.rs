use qcraft_core::ast::common::{FieldRef, NullsOrder, OrderByDef, OrderDir, SchemaRef};
use qcraft_core::ast::conditions::{CompareOp, Comparison, ConditionNode, Conditions};
use qcraft_core::ast::expr::{Expr, WindowFrameBound, WindowFrameDef, WindowFrameType};
use qcraft_core::ast::query::*;
use qcraft_core::ast::value::Value;
use qcraft_postgres::PostgresRenderer;

fn render(stmt: &QueryStmt) -> String {
    let renderer = PostgresRenderer::new();
    let (sql, _) = renderer.render_query_stmt(stmt).unwrap();
    sql
}

fn render_with_params(stmt: &QueryStmt) -> (String, Vec<Value>) {
    let renderer = PostgresRenderer::new();
    renderer.render_query_stmt(stmt).unwrap()
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
    Conditions::and(vec![ConditionNode::Comparison(Box::new(Comparison {
        left,
        op: CompareOp::Eq,
        right,
        negate: false,
    }))])
}

// ==========================================================================
// SELECT columns
// ==========================================================================

#[test]
fn select_star() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users""#);
}

#[test]
fn select_table_star() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(Some("u".into()))],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT "u".* FROM "users" AS "u""#);
}

#[test]
fn select_expr_with_alias() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: Some("one".into()),
        }],
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"SELECT $1 AS "one""#);
    assert_eq!(params, vec![Value::Int(1)]);
}

#[test]
fn select_field_with_alias() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("users", "name"),
            alias: Some("user_name".into()),
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT "users"."name" AS "user_name" FROM "users""#
    );
}

#[test]
fn select_multiple_columns() {
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
            SelectColumn::Expr {
                expr: Expr::Value(Value::Int(42)),
                alias: Some("answer".into()),
            },
        ],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"SELECT "u"."id", "u"."name", $1 AS "answer" FROM "users" AS "u""#
    );
    assert_eq!(params, vec![Value::Int(42)]);
}

#[test]
fn select_no_from() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: None,
        }],
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, "SELECT $1");
    assert_eq!(params, vec![Value::Int(1)]);
}

// ==========================================================================
// DISTINCT
// ==========================================================================

#[test]
fn select_distinct() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        distinct: Some(DistinctDef::Distinct),
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT DISTINCT * FROM "users""#);
}

#[test]
fn select_distinct_on() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        distinct: Some(DistinctDef::DistinctOn(vec![Expr::Field(FieldRef::new(
            "users", "email",
        ))])),
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT DISTINCT ON ("users"."email") * FROM "users""#
    );
}

// ==========================================================================
// FROM
// ==========================================================================

#[test]
fn from_with_alias() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" AS "u""#);
}

#[test]
fn from_with_schema() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_namespace("public"),
        )]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "public"."users""#);
}

#[test]
fn from_only() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("events")),
            only: true,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM ONLY "events""#);
}

#[test]
fn from_tablesample_bernoulli() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("large_table")),
            only: false,
            sample: Some(TableSampleDef {
                method: SampleMethod::Bernoulli,
                percentage: 10.0,
                seed: None,
            }),
            index_hint: None,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "large_table" TABLESAMPLE BERNOULLI (10)"#
    );
}

#[test]
fn from_tablesample_system_with_seed() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("big_table")),
            only: false,
            sample: Some(TableSampleDef {
                method: SampleMethod::System,
                percentage: 5.5,
                seed: Some(42),
            }),
            index_hint: None,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "big_table" TABLESAMPLE SYSTEM (5.5) REPEATABLE (42)"#
    );
}

#[test]
fn from_multiple_tables() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![
            FromItem::table(SchemaRef::new("users").with_alias("u")),
            FromItem::table(SchemaRef::new("orders").with_alias("o")),
        ]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" AS "u", "orders" AS "o""#
    );
}

// ==========================================================================
// Subquery in FROM
// ==========================================================================

#[test]
fn from_subquery() {
    let inner = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::subquery(inner, "sub".into())]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM (SELECT * FROM "users") AS "sub""#
    );
}

// ==========================================================================
// VALUES in FROM
// ==========================================================================

#[test]
fn from_values() {
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
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"SELECT * FROM (VALUES ($1, $2), ($3, $4)) AS "t" ("id", "name")"#
    );
    assert_eq!(
        params,
        vec![
            Value::Int(1),
            Value::Str("a".into()),
            Value::Int(2),
            Value::Str("b".into()),
        ]
    );
}

// ==========================================================================
// Table function in FROM
// ==========================================================================

#[test]
fn from_function() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem {
            source: TableSource::Function {
                name: "generate_series".into(),
                args: vec![Expr::Value(Value::Int(1)), Expr::Value(Value::Int(10))],
                alias: Some("s".into()),
            },
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"SELECT * FROM generate_series($1, $2) AS "s""#);
    assert_eq!(params, vec![Value::Int(1), Value::Int(10)]);
}

// ==========================================================================
// JOINs
// ==========================================================================

#[test]
fn inner_join() {
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
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" AS "u" INNER JOIN "orders" AS "o" ON "u"."id" = "o"."user_id""#
    );
}

#[test]
fn left_join() {
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
            join_type: JoinType::Left,
            natural: false,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" AS "u" LEFT JOIN "orders" AS "o" ON "u"."id" = "o"."user_id""#
    );
}

#[test]
fn right_join() {
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
            join_type: JoinType::Right,
            natural: false,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" AS "u" RIGHT JOIN "orders" AS "o" ON "u"."id" = "o"."user_id""#
    );
}

#[test]
fn full_join() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("a"))]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("b")),
            condition: Some(JoinCondition::On(simple_cond_eq(
                Expr::Field(FieldRef::new("a", "id")),
                Expr::Field(FieldRef::new("b", "id")),
            ))),
            join_type: JoinType::Full,
            natural: false,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "a" FULL JOIN "b" ON "a"."id" = "b"."id""#
    );
}

#[test]
fn cross_join() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("a"))]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("b")),
            condition: None,
            join_type: JoinType::Cross,
            natural: false,
        }]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "a" CROSS JOIN "b""#);
}

#[test]
fn natural_join() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("a"))]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("b")),
            condition: None,
            join_type: JoinType::Inner,
            natural: true,
        }]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "a" NATURAL INNER JOIN "b""#);
}

#[test]
fn join_using() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("a"))]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("b")),
            condition: Some(JoinCondition::Using(vec!["id".into(), "name".into()])),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "a" INNER JOIN "b" USING ("id", "name")"#
    );
}

#[test]
fn lateral_join() {
    let inner = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("orders"))]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(5),
            offset: None,
        }),
        ..simple_query()
    };
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem {
                source: TableSource::Lateral(Box::new(FromItem::subquery(
                    inner,
                    "recent_orders".into(),
                ))),
                only: false,
                sample: None,
                index_hint: None,
            },
            condition: Some(JoinCondition::On(simple_cond_eq(
                Expr::Value(Value::Bool(true)),
                Expr::Value(Value::Bool(true)),
            ))),
            join_type: JoinType::Left,
            natural: false,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("LEFT JOIN LATERAL"));
    assert!(sql.contains(r#""recent_orders""#));
}

// ==========================================================================
// WHERE
// ==========================================================================

#[test]
fn where_clause() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(simple_cond_eq(
            Expr::Field(FieldRef::new("users", "id")),
            Expr::Value(Value::Int(1)),
        )),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"SELECT * FROM "users" WHERE "users"."id" = $1"#);
    assert_eq!(params, vec![Value::Int(1)]);
}

#[test]
fn where_and_conditions() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![
            ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            })),
            ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(18)),
                negate: false,
            })),
        ])),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "users"."active" = $1 AND "users"."age" > $2"#
    );
    assert_eq!(params, vec![Value::Bool(true), Value::Int(18)]);
}

// ==========================================================================
// GROUP BY
// ==========================================================================

#[test]
fn group_by_simple() {
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "country"),
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
            "users", "country",
        )))]),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"SELECT "users"."country", COUNT($1) AS "cnt" FROM "users" GROUP BY "users"."country""#
    );
    assert_eq!(params, vec![Value::Int(1)]);
}

#[test]
fn group_by_rollup() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("sales"))]),
        group_by: Some(vec![GroupByItem::Rollup(vec![
            Expr::Field(FieldRef::new("sales", "region")),
            Expr::Field(FieldRef::new("sales", "product")),
        ])]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "sales" GROUP BY ROLLUP ("sales"."region", "sales"."product")"#
    );
}

#[test]
fn group_by_cube() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("sales"))]),
        group_by: Some(vec![GroupByItem::Cube(vec![
            Expr::Field(FieldRef::new("sales", "region")),
            Expr::Field(FieldRef::new("sales", "product")),
        ])]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "sales" GROUP BY CUBE ("sales"."region", "sales"."product")"#
    );
}

#[test]
fn group_by_grouping_sets() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("sales"))]),
        group_by: Some(vec![GroupByItem::GroupingSets(vec![
            vec![
                Expr::Field(FieldRef::new("sales", "region")),
                Expr::Field(FieldRef::new("sales", "product")),
            ],
            vec![Expr::Field(FieldRef::new("sales", "region"))],
            vec![],
        ])]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("GROUPING SETS ("));
    assert!(sql.contains(r#""sales"."region", "sales"."product")"#));
    assert!(sql.contains("()"));
}

// ==========================================================================
// HAVING
// ==========================================================================

#[test]
fn having_clause() {
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "country"),
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
            "users", "country",
        )))]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Value(Value::Int(1))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(5)),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    // SELECT has COUNT($1), HAVING has COUNT($2) > $3
    assert!(sql.contains("HAVING COUNT($2) > $3"), "sql: {sql}");
    assert_eq!(params, vec![Value::Int(1), Value::Int(1), Value::Int(5)]);
}

// ==========================================================================
// WINDOW clause
// ==========================================================================

#[test]
fn window_clause() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("sales"))]),
        window: Some(vec![WindowNameDef {
            name: "w".into(),
            base_window: None,
            partition_by: Some(vec![Expr::Field(FieldRef::new("sales", "region"))]),
            order_by: Some(vec![OrderByDef {
                expr: Expr::Field(FieldRef::new("sales", "amount")),
                direction: OrderDir::Desc,
                nulls: None,
            }]),
            frame: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains(
        r#"WINDOW "w" AS (PARTITION BY "sales"."region" ORDER BY "sales"."amount" DESC)"#
    ));
}

#[test]
fn window_clause_with_base() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("t"))]),
        window: Some(vec![
            WindowNameDef {
                name: "w1".into(),
                base_window: None,
                partition_by: Some(vec![Expr::Field(FieldRef::new("t", "a"))]),
                order_by: None,
                frame: None,
            },
            WindowNameDef {
                name: "w2".into(),
                base_window: Some("w1".into()),
                partition_by: None,
                order_by: Some(vec![OrderByDef {
                    expr: Expr::Field(FieldRef::new("t", "b")),
                    direction: OrderDir::Asc,
                    nulls: None,
                }]),
                frame: None,
            },
        ]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains(r#""w1" AS (PARTITION BY "t"."a")"#));
    assert!(sql.contains(r#""w2" AS ("w1" ORDER BY "t"."b" ASC)"#));
}

#[test]
fn window_clause_with_frame() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("t"))]),
        window: Some(vec![WindowNameDef {
            name: "w".into(),
            base_window: None,
            partition_by: None,
            order_by: Some(vec![OrderByDef {
                expr: Expr::Field(FieldRef::new("t", "id")),
                direction: OrderDir::Asc,
                nulls: None,
            }]),
            frame: Some(WindowFrameDef {
                frame_type: WindowFrameType::Rows,
                start: WindowFrameBound::Preceding(Some(1)),
                end: Some(WindowFrameBound::Following(Some(1))),
            }),
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING"));
}

// ==========================================================================
// ORDER BY
// ==========================================================================

#[test]
fn order_by_simple() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" ORDER BY "users"."name" ASC"#
    );
}

#[test]
fn order_by_nulls_first() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "score")),
            direction: OrderDir::Desc,
            nulls: Some(NullsOrder::First),
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" ORDER BY "users"."score" DESC NULLS FIRST"#
    );
}

#[test]
fn order_by_nulls_last() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "score")),
            direction: OrderDir::Asc,
            nulls: Some(NullsOrder::Last),
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" ORDER BY "users"."score" ASC NULLS LAST"#
    );
}

#[test]
fn order_by_multiple() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![
            OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "name")),
                direction: OrderDir::Asc,
                nulls: None,
            },
            OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "id")),
                direction: OrderDir::Desc,
                nulls: None,
            },
        ]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" ORDER BY "users"."name" ASC, "users"."id" DESC"#
    );
}

// ==========================================================================
// LIMIT / OFFSET
// ==========================================================================

#[test]
fn limit_only() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(10),
            offset: None,
        }),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" LIMIT 10"#);
}

#[test]
fn limit_with_offset() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(10),
            offset: Some(20),
        }),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" LIMIT 10 OFFSET 20"#);
}

#[test]
fn fetch_first_rows_only() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        limit: Some(LimitDef {
            kind: LimitKind::FetchFirst {
                count: 5,
                with_ties: false,
                percent: false,
            },
            offset: None,
        }),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" FETCH FIRST 5 ROWS ONLY"#
    );
}

#[test]
fn fetch_first_with_ties() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "score")),
            direction: OrderDir::Desc,
            nulls: None,
        }]),
        limit: Some(LimitDef {
            kind: LimitKind::FetchFirst {
                count: 10,
                with_ties: true,
                percent: false,
            },
            offset: None,
        }),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("FETCH FIRST 10 ROWS WITH TIES"));
}

#[test]
fn fetch_first_with_offset() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        limit: Some(LimitDef {
            kind: LimitKind::FetchFirst {
                count: 5,
                with_ties: false,
                percent: false,
            },
            offset: Some(10),
        }),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("OFFSET 10 ROWS"));
    assert!(sql.contains("FETCH FIRST 5 ROWS ONLY"));
}

#[test]
fn fetch_first_percent() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        limit: Some(LimitDef {
            kind: LimitKind::FetchFirst {
                count: 10,
                with_ties: false,
                percent: true,
            },
            offset: None,
        }),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("FETCH FIRST 10 PERCENT ROWS ONLY"));
}

#[test]
fn top_converts_to_limit() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        limit: Some(LimitDef {
            kind: LimitKind::Top {
                count: 10,
                with_ties: false,
                percent: false,
            },
            offset: None,
        }),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" LIMIT 10"#);
}

// ==========================================================================
// CTE (WITH clause)
// ==========================================================================

#[test]
fn cte_simple() {
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(simple_cond_eq(
            Expr::Field(FieldRef::new("users", "active")),
            Expr::Value(Value::Bool(true)),
        )),
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
    assert!(sql.starts_with(r#"WITH "active_users" AS (SELECT"#));
    assert!(sql.ends_with(r#"FROM "active_users""#));
}

#[test]
fn cte_recursive() {
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: Some("n".into()),
        }],
        ..simple_query()
    };
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "nums".into(),
            query: Box::new(cte_query),
            recursive: true,
            column_names: None,
            materialized: None,
        }]),
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("nums"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.starts_with(r#"WITH RECURSIVE "nums" AS ("#));
}

#[test]
fn cte_with_column_names() {
    let cte_query = QueryStmt {
        columns: vec![
            SelectColumn::Expr {
                expr: Expr::Value(Value::Int(1)),
                alias: None,
            },
            SelectColumn::Expr {
                expr: Expr::Value(Value::Str("Alice".into())),
                alias: None,
            },
        ],
        ..simple_query()
    };
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "data".into(),
            query: Box::new(cte_query),
            recursive: false,
            column_names: Some(vec!["id".into(), "name".into()]),
            materialized: None,
        }]),
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("data"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains(r#""data" ("id", "name") AS ("#));
}

#[test]
fn cte_materialized() {
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
    assert!(sql.contains(r#""cached" AS MATERIALIZED (SELECT"#));
}

#[test]
fn cte_not_materialized() {
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        ..simple_query()
    };
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "inlined".into(),
            query: Box::new(cte_query),
            recursive: false,
            column_names: None,
            materialized: Some(CteMaterialized::NotMaterialized),
        }]),
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("inlined"))]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains(r#""inlined" AS NOT MATERIALIZED (SELECT"#));
}

// ==========================================================================
// FOR UPDATE / SHARE (row locking)
// ==========================================================================

#[test]
fn for_update() {
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
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" FOR UPDATE"#);
}

#[test]
fn for_share_of_table() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::Share,
            of: Some(vec![SchemaRef::new("users")]),
            nowait: false,
            skip_locked: false,
            wait: None,
        }]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains(r#"FOR SHARE OF "users""#));
}

#[test]
fn for_update_nowait() {
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
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" FOR UPDATE NOWAIT"#);
}

#[test]
fn for_update_skip_locked() {
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
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" FOR UPDATE SKIP LOCKED"#
    );
}

#[test]
fn for_no_key_update() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::NoKeyUpdate,
            of: None,
            nowait: false,
            skip_locked: false,
            wait: None,
        }]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" FOR NO KEY UPDATE"#);
}

#[test]
fn for_key_share() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::KeyShare,
            of: None,
            nowait: false,
            skip_locked: false,
            wait: None,
        }]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" FOR KEY SHARE"#);
}

#[test]
fn multiple_locks() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![
            FromItem::table(SchemaRef::new("users").with_alias("u")),
            FromItem::table(SchemaRef::new("orders").with_alias("o")),
        ]),
        lock: Some(vec![
            SelectLockDef {
                strength: LockStrength::Update,
                of: Some(vec![SchemaRef::new("users")]),
                nowait: false,
                skip_locked: false,
                wait: None,
            },
            SelectLockDef {
                strength: LockStrength::Share,
                of: Some(vec![SchemaRef::new("orders")]),
                nowait: false,
                skip_locked: false,
                wait: None,
            },
        ]),
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains(r#"FOR UPDATE OF "users""#));
    assert!(sql.contains(r#"FOR SHARE OF "orders""#));
}

// ==========================================================================
// Set operations
// ==========================================================================

#[test]
fn union_query() {
    let left = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("a", "id"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("a"))]),
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef::new("b", "id"),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("b"))]),
        ..simple_query()
    };
    // Set ops as top-level source
    let set_op = SetOpDef {
        left: Box::new(left),
        right: Box::new(right),
        operation: SetOperationType::Union,
    };
    // Wrap in a subquery for FROM
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
    assert!(sql.contains("UNION"));
    assert!(sql.contains(r#""a"."id""#));
    assert!(sql.contains(r#""b"."id""#));
}

#[test]
fn union_all_query() {
    let left = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("a"))]),
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("b"))]),
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
    assert!(sql.contains("UNION ALL"));
}

#[test]
fn intersect_query() {
    let left = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("a"))]),
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("b"))]),
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
    assert!(sql.contains("INTERSECT"));
}

#[test]
fn except_query() {
    let left = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("a"))]),
        ..simple_query()
    };
    let right = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("b"))]),
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
    assert!(sql.contains("EXCEPT"));
}

// ==========================================================================
// Complex / integration tests
// ==========================================================================

#[test]
fn full_pipeline() {
    // WITH active AS (SELECT * FROM users WHERE active = TRUE)
    // SELECT u.id, u.name FROM active AS u
    // INNER JOIN orders AS o ON u.id = o.user_id
    // WHERE o.amount > 100
    // GROUP BY u.id, u.name
    // HAVING COUNT(1) > 2
    // ORDER BY u.name ASC
    // LIMIT 10 OFFSET 5
    // FOR UPDATE
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(simple_cond_eq(
            Expr::Field(FieldRef::new("users", "active")),
            Expr::Value(Value::Bool(true)),
        )),
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
                field: FieldRef::new("u", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("u", "name"),
                alias: None,
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
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("o", "amount")),
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(100)),
                negate: false,
            },
        ))])),
        group_by: Some(vec![
            GroupByItem::Expr(Expr::Field(FieldRef::new("u", "id"))),
            GroupByItem::Expr(Expr::Field(FieldRef::new("u", "name"))),
        ]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Value(Value::Int(1))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(2)),
                negate: false,
            },
        ))])),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("u", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(10),
            offset: Some(5),
        }),
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::Update,
            of: None,
            nowait: false,
            skip_locked: false,
            wait: None,
        }]),
        distinct: None,
        window: None,
    };
    let (sql, params) = render_with_params(&stmt);
    assert!(sql.starts_with("WITH"));
    assert!(sql.contains(r#""active" AS (SELECT"#));
    assert!(sql.contains(r#"SELECT "u"."id", "u"."name""#));
    assert!(sql.contains(r#"FROM "active" AS "u""#));
    assert!(sql.contains(r#"INNER JOIN "orders" AS "o" ON "u"."id" = "o"."user_id""#));
    assert!(sql.contains(r#"WHERE "o"."amount" > $2"#), "sql: {sql}");
    assert!(sql.contains(r#"GROUP BY "u"."id", "u"."name""#));
    assert!(sql.contains("HAVING COUNT($3) > $4"), "sql: {sql}");
    assert!(sql.contains(r#"ORDER BY "u"."name" ASC"#));
    assert!(sql.contains("LIMIT 10 OFFSET 5"));
    assert!(sql.contains("FOR UPDATE"));
    assert_eq!(
        params,
        vec![
            Value::Bool(true),
            Value::Int(100),
            Value::Int(1),
            Value::Int(2),
        ]
    );
}
