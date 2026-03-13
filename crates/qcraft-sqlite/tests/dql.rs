use qcraft_core::ast::common::*;
use qcraft_core::ast::conditions::*;
use qcraft_core::ast::custom::CustomBinaryOp;
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
        set_op: None,
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
fn field_ref_empty_table_name() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef {
                field: FieldDef::new("price"),
                table_name: "".into(),
                namespace: None,
            },
            alias: None,
        }],
        ..simple_query()
    };
    assert_eq!(render(&stmt), r#"SELECT "price" FROM "users""#);
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
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"SELECT * FROM "users" LIMIT ? OFFSET ?"#);
    assert_eq!(params, vec![Value::BigInt(10), Value::BigInt(20)]);
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
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"SELECT * FROM "users" LIMIT ?"#);
    assert_eq!(params, vec![Value::BigInt(5)]);
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
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"SELECT * FROM "users" LIMIT ?"#);
    assert_eq!(params, vec![Value::BigInt(10)]);
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
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(sql, r#"SELECT * FROM "users" LIMIT ?"#);
    assert_eq!(params, vec![Value::BigInt(5)]);
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
fn cte_recursive_with_union_all_no_wrapper() {
    // Base case: SELECT 1 AS "n"
    let base = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: Some("n".into()),
        }],
        from: None,
        ..simple_query()
    };
    // Recursive step: SELECT "n" + 1 FROM "nums" WHERE "n" < 10
    let recursive_step = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Binary {
                left: Box::new(Expr::Field(FieldRef::new("nums", "n"))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Value(Value::Int(1))),
            },
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("nums"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison::new(
                Expr::Field(FieldRef::new("nums", "n")),
                CompareOp::Lt,
                Expr::Value(Value::Int(10)),
            ),
        ))])),
        ..simple_query()
    };
    // CTE body uses set_op field directly
    let cte_body = QueryStmt {
        set_op: Some(Box::new(SetOpDef::union_all(base, recursive_step))),
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
    // CTE body should be: (SELECT 1 AS "n" UNION ALL SELECT ...)
    // NOT: (SELECT * FROM (SELECT 1 AS "n" UNION ALL SELECT ...))
    assert!(
        !sql.contains("SELECT * FROM (SELECT"),
        "CTE body should not wrap UNION ALL in SELECT * FROM (...): {sql}"
    );
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
        set_op: None,
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"SELECT "u"."name", COUNT("o"."id") AS "order_count" FROM "users" AS "u" LEFT JOIN "orders" AS "o" ON "u"."id" = "o"."user_id" WHERE "u"."active" = ? GROUP BY "u"."name" HAVING COUNT("o"."id") > ? ORDER BY "u"."name" ASC LIMIT ? OFFSET ?"#
    );
    assert_eq!(
        params,
        vec![
            Value::Bool(true),
            Value::Int(0),
            Value::BigInt(10),
            Value::BigInt(0)
        ]
    );
}

// ---------------------------------------------------------------------------
// Contains / StartsWith / EndsWith
// ---------------------------------------------------------------------------

#[test]
fn where_contains() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::contains(FieldRef::new("users", "name"), "ali")),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "users"."name" LIKE ? ESCAPE '\'"#
    );
    assert_eq!(params, vec![Value::Str("%ali%".into())]);
}

#[test]
fn where_starts_with() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::starts_with(
            FieldRef::new("users", "name"),
            "Ali",
        )),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "users"."name" LIKE ? ESCAPE '\'"#
    );
    assert_eq!(params, vec![Value::Str("Ali%".into())]);
}

#[test]
fn where_ends_with() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::ends_with(FieldRef::new("users", "name"), "ice")),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "users"."name" LIKE ? ESCAPE '\'"#
    );
    assert_eq!(params, vec![Value::Str("%ice".into())]);
}

#[test]
fn where_icontains() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::icontains(FieldRef::new("users", "name"), "ali")),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE LOWER("users"."name") LIKE LOWER(?) ESCAPE '\'"#
    );
    assert_eq!(params, vec![Value::Str("%ali%".into())]);
}

#[test]
fn where_istarts_with() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::istarts_with(
            FieldRef::new("users", "name"),
            "ali",
        )),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE LOWER("users"."name") LIKE LOWER(?) ESCAPE '\'"#
    );
    assert_eq!(params, vec![Value::Str("ali%".into())]);
}

#[test]
fn where_iends_with() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::iends_with(
            FieldRef::new("users", "name"),
            "ICE",
        )),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE LOWER("users"."name") LIKE LOWER(?) ESCAPE '\'"#
    );
    assert_eq!(params, vec![Value::Str("%ICE".into())]);
}

#[test]
fn where_contains_escapes_special_chars() {
    let (_, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::contains(
            FieldRef::new("products", "name"),
            "50%_off\\",
        )),
        ..simple_query()
    });
    assert_eq!(params, vec![Value::Str("%50\\%\\_off\\\\%".into())]);
}

// ---------------------------------------------------------------------------
// COLLATE
// ---------------------------------------------------------------------------

#[test]
fn collate_in_order_by() {
    assert_eq!(
        render(&QueryStmt {
            order_by: Some(vec![OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "name")).collate("NOCASE"),
                direction: OrderDir::Asc,
                nulls: None,
            }]),
            ..simple_query()
        }),
        r#"SELECT * FROM "users" ORDER BY "users"."name" COLLATE NOCASE ASC"#
    );
}

#[test]
fn collate_in_where() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("users", "name")).collate("NOCASE"),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Str("alice".into())),
                negate: false,
            },
        ))])),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "users"."name" COLLATE NOCASE = ?"#
    );
    assert_eq!(params, vec![Value::Str("alice".into())]);
}

#[test]
fn collate_binary() {
    assert_eq!(
        render(&QueryStmt {
            order_by: Some(vec![OrderByDef {
                expr: Expr::Field(FieldRef::new("users", "name")).collate("BINARY"),
                direction: OrderDir::Desc,
                nulls: None,
            }]),
            ..simple_query()
        }),
        r#"SELECT * FROM "users" ORDER BY "users"."name" COLLATE BINARY DESC"#
    );
}

// ---------------------------------------------------------------------------
// Custom BinaryOp — SQLite rejects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct DummyOp;

impl CustomBinaryOp for DummyOp {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn clone_box(&self) -> Box<dyn CustomBinaryOp> {
        Box::new(*self)
    }
}

#[test]
fn custom_binary_op_unsupported() {
    let err = render_err(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Binary {
                left: Box::new(Expr::Field(FieldRef::new("t", "a"))),
                op: BinaryOp::Custom(Box::new(DummyOp)),
                right: Box::new(Expr::Field(FieldRef::new("t", "b"))),
            },
            alias: None,
        }],
        ..simple_query()
    });
    assert!(err.contains("CustomBinaryOp"));
}

// ==========================================================================
// Range operators unsupported
// ==========================================================================

#[test]
fn range_strictly_left_unsupported() {
    let err = render_err(&QueryStmt {
        columns: vec![SelectColumn::all()],
        from: Some(vec![FromItem::table(SchemaRef::new("events"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison::new(
                Expr::Field(FieldRef::new("events", "period")),
                CompareOp::RangeStrictlyLeft,
                Expr::raw("'[1,10)'::int4range"),
            ),
        ))])),
        ..simple_query()
    });
    assert!(err.contains("range"));
}

#[test]
fn range_adjacent_unsupported() {
    let err = render_err(&QueryStmt {
        columns: vec![SelectColumn::all()],
        from: Some(vec![FromItem::table(SchemaRef::new("events"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison::new(
                Expr::Field(FieldRef::new("events", "period")),
                CompareOp::RangeAdjacent,
                Expr::raw("'[1,10)'::int4range"),
            ),
        ))])),
        ..simple_query()
    });
    assert!(err.contains("range"));
}

// ==========================================================================
// IN clause — array expansion
// ==========================================================================

#[test]
fn in_expands_array_to_separate_params() {
    let (sql, params) = render_with_params(&QueryStmt {
        columns: vec![SelectColumn::all()],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison::new(
                Expr::Field(FieldRef::new("users", "status")),
                CompareOp::In,
                Expr::Value(Value::Array(vec![
                    Value::Str("active".into()),
                    Value::Str("pending".into()),
                ])),
            ),
        ))])),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "users"."status" IN (?, ?)"#
    );
    assert_eq!(
        params,
        vec![Value::Str("active".into()), Value::Str("pending".into())]
    );
}

// ==========================================================================
// BETWEEN — two-value expansion
// ==========================================================================

#[test]
fn between_expands_array_to_and() {
    let (sql, params) = render_with_params(&QueryStmt {
        columns: vec![SelectColumn::all()],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison::new(
                Expr::Field(FieldRef::new("users", "age")),
                CompareOp::Between,
                Expr::Value(Value::Array(vec![Value::Int(18), Value::Int(65)])),
            ),
        ))])),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "users"."age" BETWEEN ? AND ?"#
    );
    assert_eq!(params, vec![Value::Int(18), Value::Int(65)]);
}

// ==========================================================================
// NULL parameterization
// ==========================================================================

#[test]
fn null_is_parameterized_in_insert() {
    use qcraft_core::ast::dml::*;
    let renderer = SqliteRenderer::new();
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into()]),
        source: InsertSource::Values(vec![vec![
            Expr::Value(Value::Str("Alice".into())),
            Expr::Value(Value::Null),
        ]]),
        ..InsertStmt::default()
    });
    let (sql, params) = renderer.render_mutation_stmt(&stmt).unwrap();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("name", "email") VALUES (?, ?)"#
    );
    assert_eq!(params, vec![Value::Str("Alice".into()), Value::Null]);
}

// ==========================================================================
// CROSS JOIN ignores condition
// ==========================================================================

#[test]
fn cross_join_ignores_on_condition() {
    let (sql, params) = render_with_params(&QueryStmt {
        columns: vec![SelectColumn::all()],
        from: Some(vec![FromItem::table(SchemaRef::new("products"))]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("sizes")),
            condition: Some(JoinCondition::On(Conditions::and(vec![
                ConditionNode::Comparison(Box::new(Comparison::new(
                    Expr::Value(Value::Int(1)),
                    CompareOp::Eq,
                    Expr::Value(Value::Int(1)),
                ))),
            ]))),
            join_type: JoinType::Cross,
            natural: false,
        }]),
        ..simple_query()
    });
    assert_eq!(sql, r#"SELECT * FROM "products" CROSS JOIN "sizes""#);
    assert!(params.is_empty());
}

// ==========================================================================
// FieldRef — namespace and JSON child
// ==========================================================================

#[test]
fn field_ref_with_namespace() {
    let (sql, _) = render_with_params(&QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef {
                field: qcraft_core::ast::common::FieldDef::new("id"),
                table_name: "users".into(),
                namespace: Some("main".into()),
            },
            alias: None,
        }],
        ..simple_query()
    });
    assert_eq!(sql, r#"SELECT "main"."users"."id" FROM "users""#);
}

#[test]
fn field_ref_with_json_child() {
    let (sql, _) = render_with_params(&QueryStmt {
        columns: vec![SelectColumn::Field {
            field: FieldRef {
                field: qcraft_core::ast::common::FieldDef {
                    name: "data".into(),
                    child: Some(Box::new(qcraft_core::ast::common::FieldDef {
                        name: "address".into(),
                        child: Some(Box::new(qcraft_core::ast::common::FieldDef::new("city"))),
                    })),
                },
                table_name: "users".into(),
                namespace: None,
            },
            alias: None,
        }],
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT "users"."data"->'address'->'city' FROM "users""#
    );
}

// ---------------------------------------------------------------------------
// Expr::Raw with %s parameterization
// ---------------------------------------------------------------------------

#[test]
fn raw_expr_with_params() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Raw {
                    sql: "age > %s".into(),
                    params: vec![Value::Int(18)],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            },
        ))])),
        ..simple_query()
    });
    assert!(sql.contains("age > ?"));
    assert_eq!(params[0], Value::Int(18));
}

#[test]
fn raw_expr_percent_escape() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Raw {
                    sql: "msg LIKE '%%error%%' AND level = %s".into(),
                    params: vec![Value::Str("critical".into())],
                },
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            },
        ))])),
        ..simple_query()
    });
    assert!(sql.contains("msg LIKE '%error%' AND level = ?"));
    assert_eq!(params[0], Value::Str("critical".into()));
}

#[test]
fn raw_expr_no_params_unchanged() {
    let sql = render(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Raw {
                sql: "datetime('now')".into(),
                params: vec![],
            },
            alias: Some("ts".into()),
        }],
        ..simple_query()
    });
    assert!(sql.contains("datetime('now')"));
}

// ---------------------------------------------------------------------------
// EXISTS / NOT EXISTS
// ---------------------------------------------------------------------------

#[test]
fn exists_subquery() {
    let subquery = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("orders"))]),
        ..simple_query()
    };
    let sql = render(&QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Exists(Box::new(
            subquery,
        ))])),
        ..simple_query()
    });
    assert!(sql.contains("EXISTS(SELECT"));
    assert!(!sql.contains("NOT"));
}

#[test]
fn not_exists_subquery() {
    let subquery = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Value(Value::Int(1)),
            alias: None,
        }],
        from: Some(vec![FromItem::table(SchemaRef::new("orders"))]),
        ..simple_query()
    };
    let sql = render(&QueryStmt {
        where_clause: Some(Conditions {
            children: vec![ConditionNode::Exists(Box::new(subquery))],
            connector: Connector::And,
            negated: true,
        }),
        ..simple_query()
    });
    // Must be "NOT EXISTS (" not "NOT (EXISTS ("
    assert!(sql.contains("NOT EXISTS(SELECT"));
}

// ---------------------------------------------------------------------------
// Dialect-agnostic functions (SQLite rendering)
// ---------------------------------------------------------------------------

#[test]
fn json_array_sqlite() {
    let (sql, params) = render_with_params(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::JsonArray(vec![
                Expr::Value(Value::Int(1)),
                Expr::Value(Value::Str("two".into())),
            ]),
            alias: Some("arr".into()),
        }],
        ..simple_query()
    });
    assert_eq!(sql, r#"SELECT json_array(?, ?) AS "arr" FROM "users""#);
    assert_eq!(params, vec![Value::Int(1), Value::Str("two".into())]);
}

#[test]
fn json_path_text_sqlite() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::json_path_text(Expr::field("events", "data"), "email"),
            alias: Some("email".into()),
        }],
        ..simple_query()
    };
    assert_eq!(
        render(&stmt),
        r#"SELECT "events"."data"->>'email' AS "email" FROM "users""#,
    );
}

#[test]
fn json_object_sqlite() {
    let sql = render(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::JsonObject(vec![
                ("name".into(), Expr::field("t", "name")),
                ("age".into(), Expr::field("t", "age")),
            ]),
            alias: Some("obj".into()),
        }],
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT json_object('name', "t"."name", 'age', "t"."age") AS "obj" FROM "users""#
    );
}

#[test]
fn json_agg_sqlite() {
    let sql = render(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::JsonAgg {
                expr: Box::new(Expr::field("t", "name")),
                distinct: false,
                filter: None,
                order_by: None,
            },
            alias: None,
        }],
        ..simple_query()
    });
    assert_eq!(sql, r#"SELECT json_group_array("t"."name") FROM "users""#);
}

#[test]
fn json_agg_distinct_sqlite() {
    let sql = render(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::JsonAgg {
                expr: Box::new(Expr::field("t", "name")),
                distinct: true,
                filter: None,
                order_by: None,
            },
            alias: None,
        }],
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT json_group_array(DISTINCT "t"."name") FROM "users""#
    );
}

#[test]
fn string_agg_sqlite() {
    let sql = render(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::StringAgg {
                expr: Box::new(Expr::field("t", "name")),
                delimiter: ", ".into(),
                distinct: false,
                filter: None,
                order_by: None,
            },
            alias: None,
        }],
        ..simple_query()
    });
    assert_eq!(sql, r#"SELECT group_concat("t"."name", ', ') FROM "users""#);
}

#[test]
fn string_agg_distinct_sqlite() {
    let sql = render(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::StringAgg {
                expr: Box::new(Expr::field("t", "name")),
                delimiter: ", ".into(),
                distinct: true,
                filter: None,
                order_by: None,
            },
            alias: None,
        }],
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT group_concat(DISTINCT "t"."name", ', ') FROM "users""#
    );
}

#[test]
fn now_sqlite() {
    let sql = render(&QueryStmt {
        columns: vec![SelectColumn::Expr {
            expr: Expr::Now,
            alias: Some("ts".into()),
        }],
        ..simple_query()
    });
    assert_eq!(sql, r#"SELECT datetime('now') AS "ts" FROM "users""#);
}

// ---------------------------------------------------------------------------
// REGEXP / IRegex
// ---------------------------------------------------------------------------

#[test]
fn regex_sqlite() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::field("users", "name"),
                op: CompareOp::Regex,
                right: Expr::Value(Value::Str("^john".into())),
                negate: false,
            },
        ))])),
        ..simple_query()
    });
    assert!(sql.contains("REGEXP"));
    assert_eq!(params, vec![Value::Str("^john".into())]);
}

#[test]
fn ilike_sqlite() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::field("users", "name"),
                op: CompareOp::ILike,
                right: Expr::Value(Value::Str("%john%".into())),
                negate: false,
            },
        ))])),
        ..simple_query()
    });
    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE LOWER("users"."name") LIKE LOWER(?)"#
    );
    assert_eq!(params, vec![Value::Str("%john%".into())]);
}

#[test]
fn iregex_sqlite() {
    let (sql, params) = render_with_params(&QueryStmt {
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::field("users", "name"),
                op: CompareOp::IRegex,
                right: Expr::Value(Value::Str("^john".into())),
                negate: false,
            },
        ))])),
        ..simple_query()
    });
    assert!(sql.contains("REGEXP"));
    assert!(sql.contains("'(?i)' ||"));
    assert_eq!(params, vec![Value::Str("^john".into())]);
}

#[test]
fn select_with_timedelta_param() {
    let stmt = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("events"))]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison::new(
                Expr::Field(FieldRef::new("events", "duration")),
                CompareOp::Eq,
                Expr::Value(Value::TimeDelta {
                    years: 0,
                    months: 0,
                    days: 0,
                    seconds: 3600,
                    microseconds: 0,
                }),
            ),
        ))])),
        ..simple_query()
    };
    let (sql, params) = render_with_params(&stmt);
    assert_eq!(
        sql,
        r#"SELECT * FROM "events" WHERE "events"."duration" = ?"#
    );
    assert_eq!(
        params,
        vec![Value::TimeDelta {
            years: 0,
            months: 0,
            days: 0,
            seconds: 3600,
            microseconds: 0,
        }]
    );
}
