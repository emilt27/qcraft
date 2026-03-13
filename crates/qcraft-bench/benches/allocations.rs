use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Tracking allocator
// ---------------------------------------------------------------------------

struct TrackingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

struct AllocStats {
    count: usize,
    bytes: usize,
}

fn reset_stats() {
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
}

fn snapshot() -> AllocStats {
    AllocStats {
        count: ALLOC_COUNT.load(Ordering::Relaxed),
        bytes: ALLOC_BYTES.load(Ordering::Relaxed),
    }
}

/// Run `f` N times and return average allocations per call.
fn measure_avg(n: usize, f: impl Fn()) -> AllocStats {
    // Warm up
    f();

    reset_stats();
    for _ in 0..n {
        f();
    }
    let stats = snapshot();
    AllocStats {
        count: stats.count / n,
        bytes: stats.bytes / n,
    }
}

// ---------------------------------------------------------------------------
// qcraft imports
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
        set_op: None,
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
// sea-query imports
// ---------------------------------------------------------------------------
use sea_query::{ExprTrait, Iden};

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Name,
    Email,
    Age,
    Active,
}

#[derive(Iden)]
enum Orders {
    Table,
    Id,
    UserId,
    Amount,
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

fn qc_simple_select() {
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
    let _ = renderer.render_query_stmt(&stmt).unwrap();
}

fn sq_simple_select() {
    let query = sea_query::Query::select()
        .columns([Users::Id, Users::Name, Users::Email])
        .from(Users::Table)
        .and_where(sea_query::Expr::col(Users::Age).gt(18))
        .and_where(sea_query::Expr::col(Users::Active).eq(true))
        .to_owned();
    let _ = query.build(sea_query::PostgresQueryBuilder);
}

fn qc_join_group_order() {
    let stmt = QueryStmt {
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
        where_clause: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Field(FieldRef::new("o", "amount")),
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(100)),
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
                right: Expr::Value(Value::Int(5)),
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
            offset: Some(20),
        }),
        ..qc_empty_query()
    };
    let renderer = PostgresRenderer::new();
    let _ = renderer.render_query_stmt(&stmt).unwrap();
}

fn sq_join_group_order() {
    let query = sea_query::Query::select()
        .column((Users::Table, Users::Name))
        .expr_as(
            sea_query::Func::count(sea_query::Expr::col((Orders::Table, Orders::Id))),
            sea_query::Alias::new("order_count"),
        )
        .from(Users::Table)
        .left_join(
            Orders::Table,
            sea_query::Expr::col((Users::Table, Users::Id)).equals((Orders::Table, Orders::UserId)),
        )
        .and_where(sea_query::Expr::col((Orders::Table, Orders::Amount)).gt(100))
        .group_by_col((Users::Table, Users::Name))
        .and_having(sea_query::Func::count(sea_query::Expr::col((Orders::Table, Orders::Id))).gt(5))
        .order_by((Users::Table, Users::Name), sea_query::Order::Asc)
        .limit(10)
        .offset(20)
        .to_owned();
    let _ = query.build(sea_query::PostgresQueryBuilder);
}

fn qc_insert_multi_row() {
    let stmt = MutationStmt::Insert(InsertStmt {
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
    let _ = renderer.render_mutation_stmt(&stmt).unwrap();
}

fn sq_insert_multi_row() {
    let query = sea_query::Query::insert()
        .into_table(Users::Table)
        .columns([Users::Name, Users::Email, Users::Age])
        .values_panic(["Alice".into(), "alice@example.com".into(), 30i32.into()])
        .values_panic(["Bob".into(), "bob@example.com".into(), 25i32.into()])
        .values_panic(["Charlie".into(), "charlie@example.com".into(), 35i32.into()])
        .to_owned();
    let _ = query.build(sea_query::PostgresQueryBuilder);
}

fn qc_complex_cte() {
    let cte_query = QueryStmt {
        columns: vec![SelectColumn::Star(None)],
        from: Some(vec![FromItem::table(SchemaRef::new("users"))]),
        where_clause: Some(qc_eq(
            Expr::Field(FieldRef::new("users", "active")),
            Expr::Value(Value::Bool(true)),
        )),
        ..qc_empty_query()
    };
    let stmt = QueryStmt {
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
        having: Some(Conditions::and(vec![ConditionNode::Comparison(Box::new(
            Comparison {
                left: Expr::Func {
                    name: "SUM".into(),
                    args: vec![Expr::Field(FieldRef::new("o", "amount"))],
                },
                op: CompareOp::Gt,
                right: Expr::Value(Value::Int(1000)),
                negate: false,
            },
        ))])),
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
    let _ = renderer.render_query_stmt(&stmt).unwrap();
}

fn sq_complex_cte() {
    let cte = sea_query::Query::select()
        .column(sea_query::Asterisk)
        .from(Users::Table)
        .and_where(sea_query::Expr::col(Users::Active).eq(true))
        .to_owned();

    let query = sea_query::Query::select()
        .column((sea_query::Alias::new("u"), Users::Id))
        .column((sea_query::Alias::new("u"), Users::Name))
        .expr_as(
            sea_query::Func::sum(sea_query::Expr::col((Orders::Table, Orders::Amount))),
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
            sea_query::Func::sum(sea_query::Expr::col((Orders::Table, Orders::Amount))).gt(1000),
        )
        .order_by(
            (sea_query::Alias::new("u"), Users::Name),
            sea_query::Order::Asc,
        )
        .limit(50)
        .to_owned();

    let common_table = sea_query::CommonTableExpression::new()
        .query(cte)
        .table_name(sea_query::Alias::new("active_users"))
        .to_owned();

    let with_clause = sea_query::WithClause::new().cte(common_table).to_owned();

    let final_query = query.with(with_clause).to_owned();
    let _ = final_query.build(sea_query::PostgresQueryBuilder);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const ITERATIONS: usize = 1000;

fn main() {
    println!("Memory allocation benchmark (averaged over {ITERATIONS} iterations)");
    println!();
    println!("{:<40} {:>8} {:>12}", "Scenario", "Allocs", "Bytes");
    println!("{}", "-".repeat(62));

    type Scenario<'a> = (&'a str, &'a dyn Fn(), &'a dyn Fn());
    let scenarios: Vec<Scenario<'_>> = vec![
        (
            "Simple SELECT + WHERE",
            &qc_simple_select,
            &sq_simple_select,
        ),
        (
            "JOIN + GROUP BY + ORDER BY",
            &qc_join_group_order,
            &sq_join_group_order,
        ),
        (
            "INSERT (3 rows)",
            &qc_insert_multi_row,
            &sq_insert_multi_row,
        ),
        ("Complex CTE + JOIN", &qc_complex_cte, &sq_complex_cte),
    ];

    for (name, qc_fn, sq_fn) in &scenarios {
        let qc = measure_avg(ITERATIONS, qc_fn);
        let sq = measure_avg(ITERATIONS, sq_fn);

        println!();
        println!("  {name}");
        println!("    {:<36} {:>8} {:>10} B", "qcraft", qc.count, qc.bytes);
        println!("    {:<36} {:>8} {:>10} B", "sea-query", sq.count, sq.bytes);

        let alloc_ratio = sq.count as f64 / qc.count.max(1) as f64;
        let bytes_ratio = sq.bytes as f64 / qc.bytes.max(1) as f64;
        println!(
            "    {:<36} {:>7.1}x {:>9.1}x",
            "sea-query / qcraft", alloc_ratio, bytes_ratio
        );
    }

    println!();
}
