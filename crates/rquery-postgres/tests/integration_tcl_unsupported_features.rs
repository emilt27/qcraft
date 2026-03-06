//! Tests that verify PostgreSQL renderer returns errors for unsupported TCL features.

use rquery_core::ast::tcl::*;
use rquery_postgres::PostgresRenderer;

// ==========================================================================
// Custom TCL — unsupported
// ==========================================================================

#[test]
fn custom_transaction_unsupported() {
    use rquery_core::ast::custom::CustomTransaction;

    #[derive(Debug, Clone)]
    struct MyCustomTx;

    impl CustomTransaction for MyCustomTx {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn clone_box(&self) -> Box<dyn CustomTransaction> {
            Box::new(self.clone())
        }
    }

    let stmt = TransactionStmt::Custom(Box::new(MyCustomTx));
    let renderer = PostgresRenderer::new();
    let result = renderer.render_transaction_stmt(&stmt);
    assert!(result.is_err(), "Custom TCL should return an error");
}
