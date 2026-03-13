//! Tests that verify PostgreSQL renderer returns errors for unsupported DML features.

use qcraft_core::ast::dml::*;
use qcraft_postgres::PostgresRenderer;

// ==========================================================================
// Custom DML — unsupported
// ==========================================================================

#[test]
fn custom_mutation_unsupported() {
    use qcraft_core::ast::custom::CustomMutation;

    #[derive(Debug, Clone)]
    struct MyCustomMutation;

    impl CustomMutation for MyCustomMutation {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn clone_box(&self) -> Box<dyn CustomMutation> {
            Box::new(self.clone())
        }
    }

    let stmt = MutationStmt::Custom(Box::new(MyCustomMutation));
    let renderer = PostgresRenderer::new();
    let result = renderer.render_mutation_stmt(&stmt);
    assert!(result.is_err(), "Custom DML should return an error");
}
