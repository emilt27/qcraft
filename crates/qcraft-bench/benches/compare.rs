use criterion::{Criterion, black_box, criterion_group, criterion_main};
use sea_query::ExprTrait;

// ---------------------------------------------------------------------------
// qcraft helpers
// ---------------------------------------------------------------------------
use qcraft_core::ast::common::*;
use qcraft_core::ast::conditions::*;
use qcraft_core::ast::dml::*;
use qcraft_core::ast::expr::Expr;
use qcraft_core::ast::query::*;
use qcraft_core::ast::value::Value;
use qcraft_postgres::PostgresRenderer;

fn qc_empty_query() -> QueryStmt {
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

fn qc_eq(left: Expr, right: Expr) -> Conditions {
    Conditions::and(vec![ConditionNode::Comparison(Box::new(Comparison {
        left,
        op: CompareOp::Eq,
        right,
        negate: false,
    }))])
}

// ---------------------------------------------------------------------------
// sea-query helpers
// ---------------------------------------------------------------------------
use sea_query::Iden;

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Name,
    Email,
    Age,
    Active,
    Country,
}

#[derive(Iden)]
enum Orders {
    Table,
    Id,
    UserId,
    Amount,
}

// ===========================================================================
// Benchmark 1: Simple SELECT with WHERE
// ===========================================================================

fn bench_simple_select(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_select_where");

    // -- qcraft --
    let qc_stmt = QueryStmt {
        columns: vec![
            SelectColumn::Field {
                field: FieldRef::new("users", "id"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "name"),
                alias: None,
            },
            SelectColumn::Field {
                field: FieldRef::new("users", "email"),
                alias: None,
            },
        ],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(Conditions::and(vec![
            ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Field(FieldRef::new("users", "age")),
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(18)),
                negate: false,
            })),
            ConditionNode::Comparison(Box::new(Comparison {
                left: Expr::Field(FieldRef::new("users", "active")),
                op: CompareOp::Eq,
                right: Expr::Value(Value::Bool(true)),
                negate: false,
            })),
        ])),
        ..qc_empty_query()
    };
    let renderer = PostgresRenderer::new();

    group.bench_function("qcraft", |b| {
        b.iter(|| {
            let (sql, params) = renderer.render_query_stmt(black_box(&qc_stmt)).unwrap();
            black_box((sql, params));
        });
    });

    // -- sea-query --
    group.bench_function("sea_query", |b| {
        b.iter(|| {
            let query = sea_query::Query::select()
                .columns([Users::Id, Users::Name, Users::Email])
                .from(Users::Table)
                .and_where(sea_query::Expr::col(Users::Age).gt(18))
                .and_where(sea_query::Expr::col(Users::Active).eq(true))
                .to_owned();
            let (sql, values) = query.build(sea_query::PostgresQueryBuilder);
            black_box((sql, values));
        });
    });

    group.finish();
}

// ===========================================================================
// Benchmark 2: SELECT with JOIN + GROUP BY + ORDER BY
// ===========================================================================

fn bench_join_group_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("join_group_order");

    // -- qcraft --
    let qc_stmt = QueryStmt {
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
        from: Some(vec![FromItem::table(
            SchemaRef::new("users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders").with_alias("o")),
            condition: Some(JoinCondition::On(qc_eq(
                Expr::Field(FieldRef::new("u", "id")),
                Expr::Field(FieldRef::new("o", "user_id")),
            ))),
            join_type: JoinType::Left,
            natural: false,
        }]),
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(
            Box::new(Comparison {
                left: Expr::Field(FieldRef::new("o", "amount")),
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(100)),
                negate: false,
            }),
        )])),
        group_by: Some(vec![GroupByItem::Expr(Expr::Field(FieldRef::new(
            "u", "name",
        )))]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(
            Box::new(Comparison {
                left: Expr::Func {
                    name: "COUNT".into(),
                    args: vec![Expr::Field(FieldRef::new("o", "id"))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(5)),
                negate: false,
            }),
        )])),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("u", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(10),
            offset: Some(20),
        }),
        ..qc_empty_query()
    };
    let renderer = PostgresRenderer::new();

    group.bench_function("qcraft", |b| {
        b.iter(|| {
            let (sql, params) = renderer.render_query_stmt(black_box(&qc_stmt)).unwrap();
            black_box((sql, params));
        });
    });

    // -- sea-query --
    group.bench_function("sea_query", |b| {
        b.iter(|| {
            let query = sea_query::Query::select()
                .column((Users::Table, Users::Name))
                .expr_as(
                    sea_query::Func::count(sea_query::Expr::col((Orders::Table, Orders::Id))),
                    sea_query::Alias::new("order_count"),
                )
                .from(Users::Table)
                .left_join(
                    Orders::Table,
                    sea_query::Expr::col((Users::Table, Users::Id))
                        .equals((Orders::Table, Orders::UserId)),
                )
                .and_where(sea_query::Expr::col((Orders::Table, Orders::Amount)).gt(100))
                .group_by_col((Users::Table, Users::Name))
                .and_having(
                    sea_query::Func::count(sea_query::Expr::col((Orders::Table, Orders::Id)))
                        .gt(5),
                )
                .order_by((Users::Table, Users::Name), sea_query::Order::Asc)
                .limit(10)
                .offset(20)
                .to_owned();
            let (sql, values) = query.build(sea_query::PostgresQueryBuilder);
            black_box((sql, values));
        });
    });

    group.finish();
}

// ===========================================================================
// Benchmark 3: INSERT with multiple rows
// ===========================================================================

fn bench_insert_multi_row(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_multi_row");

    // -- qcraft --
    let qc_stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["name".into(), "email".into(), "age".into()]),
        source: InsertSource::Values(vec![
            vec![
                Expr::Value(Value::Str("Alice".into())),
                Expr::Value(Value::Str("alice@example.com".into())),
                Expr::Value(Value::Int(30)),
            ],
            vec![
                Expr::Value(Value::Str("Bob".into())),
                Expr::Value(Value::Str("bob@example.com".into())),
                Expr::Value(Value::Int(25)),
            ],
            vec![
                Expr::Value(Value::Str("Charlie".into())),
                Expr::Value(Value::Str("charlie@example.com".into())),
                Expr::Value(Value::Int(35)),
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
    let renderer = PostgresRenderer::new();

    group.bench_function("qcraft", |b| {
        b.iter(|| {
            let (sql, params) = renderer.render_mutation_stmt(black_box(&qc_stmt)).unwrap();
            black_box((sql, params));
        });
    });

    // -- sea-query --
    group.bench_function("sea_query", |b| {
        b.iter(|| {
            let query = sea_query::Query::insert()
                .into_table(Users::Table)
                .columns([Users::Name, Users::Email, Users::Age])
                .values_panic(["Alice".into(), "alice@example.com".into(), 30i32.into()])
                .values_panic(["Bob".into(), "bob@example.com".into(), 25i32.into()])
                .values_panic(["Charlie".into(), "charlie@example.com".into(), 35i32.into()])
                .to_owned();
            let (sql, values) = query.build(sea_query::PostgresQueryBuilder);
            black_box((sql, values));
        });
    });

    group.finish();
}

// ===========================================================================
// Benchmark 4: Complex query — CTE + JOIN + GROUP BY + HAVING + ORDER + LIMIT
// ===========================================================================

fn bench_complex_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_cte_join");

    // -- qcraft --
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(qc_eq(
            Expr::Field(FieldRef::new("users", "active")),
            Expr::Value(Value::Bool(true)),
        )),
        ..qc_empty_query()
    };
    let qc_stmt = QueryStmt {
        ctes: Some(vec![CteDef {
            name: "active_users".into(),
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
            SelectColumn::Expr {
                expr: Expr::Func {
                    name: "SUM".into(),
                    args: vec![Expr::Field(FieldRef::new("o", "amount"))],
                },
                alias: Some("total".into()),
            },
        ],
        from: Some(vec![FromItem::table(
            SchemaRef::new("active_users").with_alias("u"),
        )]),
        joins: Some(vec![JoinDef {
            source: FromItem::table(SchemaRef::new("orders").with_alias("o")),
            condition: Some(JoinCondition::On(qc_eq(
                Expr::Field(FieldRef::new("u", "id")),
                Expr::Field(FieldRef::new("o", "user_id")),
            ))),
            join_type: JoinType::Inner,
            natural: false,
        }]),
        group_by: Some(vec![
            GroupByItem::Expr(Expr::Field(FieldRef::new("u", "id"))),
            GroupByItem::Expr(Expr::Field(FieldRef::new("u", "name"))),
        ]),
        having: Some(Conditions::and(vec![ConditionNode::Comparison(
            Box::new(Comparison {
                left: Expr::Func {
                    name: "SUM".into(),
                    args: vec![Expr::Field(FieldRef::new("o", "amount"))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(1000)),
                negate: false,
            }),
        )])),
        order_by: Some(vec![OrderByDef {
            expr: Expr::Field(FieldRef::new("u", "name")),
            direction: OrderDir::Asc,
            nulls: None,
        }]),
        limit: Some(LimitDef {
            kind: LimitKind::Limit(50),
            offset: None,
        }),
        ..qc_empty_query()
    };
    let renderer = PostgresRenderer::new();

    group.bench_function("qcraft", |b| {
        b.iter(|| {
            let (sql, params) = renderer.render_query_stmt(black_box(&qc_stmt)).unwrap();
            black_box((sql, params));
        });
    });

    // -- sea-query --
    group.bench_function("sea_query", |b| {
        b.iter(|| {
            let cte = sea_query::Query::select()
                .expr(sea_query::Expr::asterisk())
                .from(Users::Table)
                .and_where(sea_query::Expr::col(Users::Active).eq(true))
                .to_owned();

            let query = sea_query::Query::select()
                .column((sea_query::Alias::new("u"), Users::Id))
                .column((sea_query::Alias::new("u"), Users::Name))
                .expr_as(
                    sea_query::Func::sum(sea_query::Expr::col((
                        Orders::Table,
                        Orders::Amount,
                    ))),
                    sea_query::Alias::new("total"),
                )
                .from_as(
                    sea_query::Alias::new("active_users"),
                    sea_query::Alias::new("u"),
                )
                .inner_join(
                    Orders::Table,
                    sea_query::Expr::col((sea_query::Alias::new("u"), Users::Id))
                        .equals((Orders::Table, Orders::UserId)),
                )
                .group_by_col((sea_query::Alias::new("u"), Users::Id))
                .group_by_col((sea_query::Alias::new("u"), Users::Name))
                .and_having(
                    sea_query::Func::sum(sea_query::Expr::col((
                        Orders::Table,
                        Orders::Amount,
                    )))
                    .gt(1000),
                )
                .order_by((sea_query::Alias::new("u"), Users::Name), sea_query::Order::Asc)
                .limit(50)
                .to_owned();

            let common_table = sea_query::CommonTableExpression::new()
                .query(cte)
                .table_name(sea_query::Alias::new("active_users"))
                .to_owned();

            let with_clause = sea_query::WithClause::new()
                .cte(common_table)
                .to_owned();

            let final_query = query.with(with_clause).to_owned();
            let (sql, values) = final_query.build(sea_query::PostgresQueryBuilder);
            black_box((sql, values));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_select,
    bench_join_group_order,
    bench_insert_multi_row,
    bench_complex_query,
);
criterion_main!(benches);
