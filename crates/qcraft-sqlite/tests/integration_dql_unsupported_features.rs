//! Integration tests for DQL features that return errors in the SQLite renderer.
//! These features are not supported by SQLite and the renderer should return Err.

use qcraft_core::ast::common::*;
use qcraft_core::ast::expr::*;
use qcraft_core::ast::query::*;
use qcraft_sqlite::SqliteRenderer;

// ── Helpers ──────────────────────────────────────────────────────────────────

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
// DISTINCT ON — unsupported
// ---------------------------------------------------------------------------

#[test]
fn distinct_on_unsupported() {
    let stmt = QueryStmt {
        distinct: Some(DistinctDef::DistinctOn(vec![Expr::Field(FieldRef::new(
            "users", "id",
        ))])),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("DISTINCT ON"),
        "expected DISTINCT ON error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// TABLESAMPLE — unsupported
// ---------------------------------------------------------------------------

#[test]
fn tablesample_unsupported() {
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::Table(SchemaRef::new("users")),
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
    let err = render_err(&stmt);
    assert!(
        err.contains("TABLESAMPLE"),
        "expected TABLESAMPLE error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// LATERAL — unsupported
// ---------------------------------------------------------------------------

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
    let err = render_err(&stmt);
    assert!(
        err.contains("LATERAL"),
        "expected LATERAL error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// GROUP BY ROLLUP — unsupported
// ---------------------------------------------------------------------------

#[test]
fn rollup_unsupported() {
    let stmt = QueryStmt {
        group_by: Some(vec![GroupByItem::Rollup(vec![Expr::Field(FieldRef::new(
            "t", "a",
        ))])]),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(err.contains("ROLLUP"), "expected ROLLUP error, got: {err}");
}

// ---------------------------------------------------------------------------
// GROUP BY CUBE — unsupported
// ---------------------------------------------------------------------------

#[test]
fn cube_unsupported() {
    let stmt = QueryStmt {
        group_by: Some(vec![GroupByItem::Cube(vec![Expr::Field(FieldRef::new(
            "t", "a",
        ))])]),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(err.contains("CUBE"), "expected CUBE error, got: {err}");
}

// ---------------------------------------------------------------------------
// GROUPING SETS — unsupported
// ---------------------------------------------------------------------------

#[test]
fn grouping_sets_unsupported() {
    let stmt = QueryStmt {
        group_by: Some(vec![GroupByItem::GroupingSets(vec![vec![Expr::Field(
            FieldRef::new("t", "a"),
        )]])]),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("GROUPING SETS"),
        "expected GROUPING SETS error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// FOR UPDATE — unsupported
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
    let err = render_err(&stmt);
    assert!(
        err.contains("FOR UPDATE"),
        "expected FOR UPDATE error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// CROSS APPLY — unsupported
// ---------------------------------------------------------------------------

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
    let err = render_err(&stmt);
    assert!(err.contains("APPLY"), "expected APPLY error, got: {err}");
}

// ---------------------------------------------------------------------------
// FETCH FIRST WITH TIES — unsupported
// ---------------------------------------------------------------------------

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
    let err = render_err(&stmt);
    assert!(
        err.contains("WITH TIES"),
        "expected WITH TIES error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// INTERSECT ALL — unsupported
// ---------------------------------------------------------------------------

#[test]
fn intersect_all_unsupported() {
    let left = simple_query();
    let right = simple_query();
    let stmt = QueryStmt {
        from: Some(vec![FromItem {
            source: TableSource::SetOp(Box::new(SetOpDef {
                left: Box::new(left),
                right: Box::new(right),
                operation: SetOperationType::IntersectAll,
            })),
            only: false,
            sample: None,
            index_hint: None,
        }]),
        ..simple_query()
    };
    let err = render_err(&stmt);
    assert!(
        err.contains("INTERSECT ALL"),
        "expected INTERSECT ALL error, got: {err}"
    );
}
