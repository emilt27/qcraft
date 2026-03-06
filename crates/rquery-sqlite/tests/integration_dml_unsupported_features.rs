//! Tests that verify SQLite renderer correctly rejects unsupported DML features
//! with meaningful errors (not silent failures).

use std::any::Any;
use rquery_core::ast::common::SchemaRef;
use rquery_core::ast::custom::CustomMutation;
use rquery_core::ast::dml::*;
use rquery_core::ast::expr::Expr;
use rquery_core::ast::value::Value;
use rquery_sqlite::SqliteRenderer;

fn render_err(stmt: &MutationStmt) -> String {
    let renderer = SqliteRenderer::new();
    renderer.render_mutation_stmt(stmt).unwrap_err().to_string()
}

// ==========================================================================
// ON CONFLICT ON CONSTRAINT — unsupported (PG-only)
// ==========================================================================

#[test]
fn on_conflict_on_constraint_unsupported() {
    let stmt = MutationStmt::Insert(InsertStmt {
        table: SchemaRef::new("users"),
        columns: Some(vec!["id".into()]),
        source: InsertSource::Values(vec![vec![Expr::Value(Value::Int(1))]]),
        on_conflict: Some(vec![OnConflictDef {
            target: Some(ConflictTarget::Constraint("pk_users".into())),
            action: ConflictAction::DoNothing,
        }]),
        returning: None,
        ctes: None,
        overriding: None,
        conflict_resolution: None,
        partition: None,
        ignore: false,
    });

    let err = render_err(&stmt);
    assert!(err.contains("ON CONSTRAINT"), "expected ON CONSTRAINT error, got: {err}");
}

// ==========================================================================
// Custom mutation — unsupported
// ==========================================================================

#[derive(Debug, Clone)]
struct DummyMutation;

impl CustomMutation for DummyMutation {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn CustomMutation> {
        Box::new(self.clone())
    }
}

#[test]
fn custom_mutation_unsupported() {
    let stmt = MutationStmt::Custom(Box::new(DummyMutation));

    let err = render_err(&stmt);
    assert!(!err.is_empty(), "expected error for Custom mutation, got empty");
}
