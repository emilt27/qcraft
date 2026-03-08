use qcraft_core::ast::common::*;
use qcraft_core::ast::conditions::*;
use qcraft_core::ast::expr::*;
use qcraft_core::ast::query::*;
use qcraft_core::ast::value::Value;
use qcraft_sqlite::SqliteRenderer;

fn render(stmt: &QueryStmt) -> String {
    let renderer = SqliteRenderer::new();
    let (sql, _params) = renderer.render_query_stmt(stmt).unwrap();
    sql
}

fn render_with_params(stmt: &QueryStmt) -> (String, Vec<Value>) {
    let renderer = SqliteRenderer::new();
    renderer.render_query_stmt(stmt).unwrap()
}

fn render_err(stmt: &QueryStmt) -> String {
    let renderer = SqliteRenderer::new();
    renderer.render_query_stmt(stmt).unwrap_err().to_string()
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
// Basic SELECT
// ---------------------------------------------------------------------------

#[test]
fn select_star() {
    assert_eq!(render(&simple_query()), r#"SELECT * FROM "users""#);
}

#[test]
fn select_columns() {
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: Some("user_name".into()),
            },
        ],
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT "users"."id", "users"."name" AS "user_name" FROM "users""#
    );
}

#[test]
fn select_expr() {
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
    assert_eq!(
        render(&stmt),
        r#"SELECT COUNT("users"."id") AS "cnt" FROM "users""#
    );
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
fn select_no_from() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: None,
        }],
        from: None,
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, "SELECT ?");
    assert_eq!(params, vec![Value::Int(1)]);
}

// ---------------------------------------------------------------------------
// DISTINCT
// ---------------------------------------------------------------------------

#[test]
fn select_distinct() {
    let stmt = QueryStmt {
        distinct: Some(DistinctDef::Distinct),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT DISTINCT * FROM "users""#);
}

#[test]
fn distinct_on_unsupported() {
    let stmt = QueryStmt {
        distinct: Some(DistinctDef::DistinctOn(vec![Expr::Field(FieldRef::new(
            "users", "id",
        ))])),
        ..simple_query()
    };
    assert!(render_err(&stmt).contains("DISTINCT ON"));
}

// ---------------------------------------------------------------------------
// FROM
// ---------------------------------------------------------------------------

#[test]
fn from_with_namespace() {
    let stmt = QueryStmt {
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_namespace("main"),
        )]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "main"."users""#);
}

#[test]
fn from_multiple_tables() {
    let stmt = QueryStmt {
        from: Some(vec![
            FromItem::table(SchemaRef::new("t1")),
            FromItem::table(SchemaRef::new("t2")),
        ]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "t1", "t2""#);
}

#[test]
fn from_indexed_by() {
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("users")),
            only: false,
            sample: None,
            index_hint: Some(SqliteIndexHint::IndexedBy("idx_name".into())),
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" INDEXED BY "idx_name""#
    );
}

#[test]
fn from_not_indexed() {
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("users")),
            only: false,
            sample: None,
            index_hint: Some(SqliteIndexHint::NotIndexed),
        }]),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" NOT INDEXED"#);
}

#[test]
fn from_subquery() {
    let inner = simple_query();
    let stmt = QueryStmt {
        from: Some(vec![FromItem::subquery(inner, "sub".into())]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM (SELECT * FROM "users") AS "sub""#
    );
}

#[test]
fn from_table_function() {
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::Function {
                name: "json_each".into(),
                args: vec![Expr::Field(FieldRef::new("t", "data"))],
                alias: Some("j".into()),
            },
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM json_each("t"."data") AS "j""#
    );
}

#[test]
fn from_values() {
    let stmt = QueryStmt {
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
        r#"SELECT * FROM (VALUES (?, ?), (?, ?)) AS "t" ("id", "name")"#
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

#[test]
fn tablesample_unsupported() {
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("t")),
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
    assert!(render_err(&stmt).contains("TABLESAMPLE"));
}

#[test]
fn lateral_unsupported() {
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::Lateral(Box::new(FromItem::table(SchemaRef::new("t")))),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    assert!(render_err(&stmt).contains("LATERAL"));
}

// ---------------------------------------------------------------------------
// JOINs
// ---------------------------------------------------------------------------

#[test]
fn inner_join() {
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
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" INNER JOIN "orders" ON "users"."id" = "orders"."user_id""#
    );
}

#[test]
fn left_join() {
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
            join_type: JoinType::Left,
            natural: false,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" LEFT JOIN "orders" ON "users"."id" = "orders"."user_id""#
    );
}

#[test]
fn cross_join() {
    let stmt = QueryStmt {
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("colors")),
            condition: None,
            join_type: JoinType::Cross,
            natural: false,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" CROSS JOIN "colors""#
    );
}

#[test]
fn natural_join() {
    let stmt = QueryStmt {
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("profiles")),
            condition: None,
            join_type: JoinType::Inner,
            natural: true,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" NATURAL INNER JOIN "profiles""#
    );
}

#[test]
fn join_using() {
    let stmt = QueryStmt {
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders")),
            condition: Some(JoinCondition::Using(vec!["user_id".into()])),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" INNER JOIN "orders" USING ("user_id")"#
    );
}

#[test]
fn apply_unsupported() {
    let stmt = QueryStmt {
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("t")),
            condition: None,
            join_type: JoinType::CrossApply,
            natural: false,
        }]),
        ..simple_query()
    };
    assert!(render_err(&stmt).contains("APPLY"));
}

// ---------------------------------------------------------------------------
// WHERE
// ---------------------------------------------------------------------------

#[test]
fn where_simple() {
    let stmt = QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"SELECT * FROM "users" WHERE "users"."active" = ?"#);
    assert_eq!(params, vec![Value::Bool(true)]);
}

// ---------------------------------------------------------------------------
// GROUP BY
// ---------------------------------------------------------------------------

#[test]
fn group_by_simple() {
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "status"),
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
            "users", "status",
        )))]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT "users"."status", COUNT("users"."id") AS "cnt" FROM "users" GROUP BY "users"."status""#
    );
}

#[test]
fn rollup_unsupported() {
    let stmt = QueryStmt {
        group_by: Some(vec![GroupByItem::Rollup(vec![Expr::Field(FieldRef::new(
            "t", "a",
        ))])]),
        ..simple_query()
    };
    assert!(render_err(&stmt).contains("ROLLUP"));
}

#[test]
fn cube_unsupported() {
    let stmt = QueryStmt {
        group_by: Some(vec![GroupByItem::Cube(vec![Expr::Field(FieldRef::new(
            "t", "a",
        ))])]),
        ..simple_query()
    };
    assert!(render_err(&stmt).contains("CUBE"));
}

#[test]
fn grouping_sets_unsupported() {
    let stmt = QueryStmt {
        group_by: Some(vec![GroupByItem::GroupingSets(vec![vec![Expr::Field(
            FieldRef::new("t", "a"),
        )]])]),
        ..simple_query()
    };
    assert!(render_err(&stmt).contains("GROUPING SETS"));
}

// ---------------------------------------------------------------------------
// HAVING
// ---------------------------------------------------------------------------

#[test]
fn having() {
    let stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "status"),
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
            "users", "status",
        )))]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Field(FieldRef::new("users", "id"))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(5)),
                negate: false,
            },
        ))])),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert!(sql.contains("HAVING COUNT("));
    assert!(sql.contains("> ?"));
    assert_eq!(params, vec![Value::Int(5)]);
}

// ---------------------------------------------------------------------------
// WINDOW clause
// ---------------------------------------------------------------------------

#[test]
fn window_clause() {
    let stmt = QueryStmt {
        window: Some(vec![WindowNameDef {
            name: "w".into(),
            base_window: None,
            partition_by: Some(vec![Expr::Field(FieldRef::new("t", "dept"))]),
            order_by: Some(vec![OrderByDef {
                expr: Expr::Field(FieldRef::new("t", "salary")),
                direction: OrderDir::Desc,
                nulls: None,
            }]),
            frame: None,
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WINDOW "w" AS (PARTITION BY "t"."dept" ORDER BY "t"."salary" DESC)"#
    );
}

#[test]
fn window_with_frame() {
    let stmt = QueryStmt {
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
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" WINDOW "w" AS (ORDER BY "t"."id" ASC ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING)"#
    );
}

// ---------------------------------------------------------------------------
// ORDER BY
// ---------------------------------------------------------------------------

#[test]
fn order_by() {
    let stmt = QueryStmt {
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

#[test]
fn order_by_nulls() {
    let stmt = QueryStmt {
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("users", "name")),
            direction: OrderDir::Asc,
            nulls: Some(NullsOrder::Last),
        }]),
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT * FROM "users" ORDER BY "users"."name" ASC NULLS LAST"#
    );
}

// ---------------------------------------------------------------------------
// LIMIT / OFFSET
// ---------------------------------------------------------------------------

#[test]
fn limit_offset() {
    let stmt = QueryStmt {
        limit: Some(LimitDef {
            kind: LimitKind::Limit(10),
            offset: Some(20),
        }),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" LIMIT 10 OFFSET 20"#);
}

#[test]
fn limit_only() {
    let stmt = QueryStmt {
        limit: Some(LimitDef {
            kind: LimitKind::Limit(5),
            offset: None,
        }),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" LIMIT 5"#);
}

#[test]
fn fetch_first_converts_to_limit() {
    let stmt = QueryStmt {
        limit: Some(LimitDef {
            kind: LimitKind::FetchFirst {
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

#[test]
fn fetch_first_with_ties_unsupported() {
    let stmt = QueryStmt {
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
    assert!(render_err(&stmt).contains("WITH TIES"));
}

#[test]
fn top_converts_to_limit() {
    let stmt = QueryStmt {
        limit: Some(LimitDef {
            kind: LimitKind::Top {
                count: 5,
                with_ties: false,
                percent: false,
            },
            offset: None,
        }),
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT * FROM "users" LIMIT 5"#);
}

// ---------------------------------------------------------------------------
// CTE
// ---------------------------------------------------------------------------

#[test]
fn cte_simple() {
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "active_users".into(),
            query: Box::new(QueryStmt {
                where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
                    Comparison {
                        left: Expr::Field(FieldRef::new("users", "active")),
                        op: CompareOp::Eq,
                        right: Expr::Value(Value::Bool(true)),
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
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"WITH "active_users" AS (SELECT * FROM "users" WHERE "users"."active" = ?) SELECT * FROM "active_users""#
    );
    assert_eq!(params, vec![Value::Bool(true)]);
}

#[test]
fn cte_recursive() {
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "nums".into(),
            query: Box::new(QueryStmt {
                columns: vec![SelectColumn::Expr {
                    expr: Expr::Value(Value::Int(1)),
                    alias: Some("n".into()),
                }],
                from: None,
                ..simple_query()
            }),
            recursive: true,
            column_names: Some(vec!["n".into()]),
            materialized: None,
        }]),
        from: Some(vec![FromItem::table(SchemaRef::new("nums"))]),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"WITH RECURSIVE "nums" ("n") AS (SELECT ? AS "n") SELECT * FROM "nums""#
    );
    assert_eq!(params, vec![Value::Int(1)]);
}

#[test]
fn cte_materialized_ignored() {
    // SQLite ignores MATERIALIZED hints
    let stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "t".into(),
            query: Box::new(simple_query()),
            recursive: false,
            column_names: None,
            materialized: Some(CteMaterialized::Materialized),
        }]),
        from: Some(vec![FromItem::table(SchemaRef::new("t"))]),
        ..simple_query()
    };
    // Should render without MATERIALIZED
    let sql = render(&stmt);
    assert!(!sql.contains("MATERIALIZED"));
    assert!(sql.starts_with(r#"WITH "t" AS"#));
}

// ---------------------------------------------------------------------------
// Set operations
// ---------------------------------------------------------------------------

#[test]
fn union_all() {
    let left = simple_query();
    let right = QueryStmt {
        from: Some(vec![FromItem::table(SchemaRef::new("admins"))]),
        ..simple_query()
    };
    let stmt = QueryStmt {
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
        columns: vec![SelectColumn::Star(None)],
        ..simple_query()
    };
    let sql = render(&stmt);
    assert!(sql.contains("UNION ALL"));
}

#[test]
fn intersect_all_unsupported() {
    let left = simple_query();
    let right = simple_query();
    let set_op = SetOpDef {
        left: Box::new(left),
        right: Box::new(right),
        operation: SetOperationType::IntersectAll,
    };
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::SetOp(Box::new(set_op)),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    assert!(render_err(&stmt).contains("INTERSECT ALL"));
}

// ---------------------------------------------------------------------------
// FOR UPDATE unsupported
// ---------------------------------------------------------------------------

#[test]
fn for_update_unsupported() {
    let stmt = QueryStmt {
        lock: Some(vec![SelectLockDef {
            strength: LockStrength::Update,
            of: None,
            nowait: false,
            skip_locked: false,
            wait: None,
        }]),
        ..simple_query()
    };
    assert!(render_err(&stmt).contains("FOR UPDATE"));
}

// ---------------------------------------------------------------------------
// Full integration
// ---------------------------------------------------------------------------

#[test]
fn full_query() {
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
            join_type: JoinType::Left,
            natural: false,
        }]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("u", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
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
                right: Expr::Value(Value::Int(0)),
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
            offset: Some(0),
        }),
        lock: None,
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"SELECT "u"."name", COUNT("o"."id") AS "order_count" FROM "users" AS "u" LEFT JOIN "orders" AS "o" ON "u"."id" = "o"."user_id" WHERE "u"."active" = ? GROUP BY "u"."name" HAVING COUNT("o"."id") > ? ORDER BY "u"."name" ASC LIMIT 10 OFFSET 0"#
    );
    assert_eq!(params, vec![Value::Bool(true), Value::Int(0)]);
}
