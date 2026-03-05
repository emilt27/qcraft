use std::any::Any;
use std::fmt::Debug;

// ---------------------------------------------------------------------------
// Macro to define Custom* traits with common boilerplate
// ---------------------------------------------------------------------------

macro_rules! define_custom_trait {
    ($trait_name:ident) => {
        pub trait $trait_name: Debug + Send + Sync {
            fn as_any(&self) -> &dyn Any;
            fn clone_box(&self) -> Box<dyn $trait_name>;
        }

        impl Clone for Box<dyn $trait_name> {
            fn clone(&self) -> Self {
                self.clone_box()
            }
        }
    };
}

define_custom_trait!(CustomExpr);
define_custom_trait!(CustomCondition);
define_custom_trait!(CustomCompareOp);
define_custom_trait!(CustomTableSource);
define_custom_trait!(CustomMutation);
define_custom_trait!(CustomSchemaMutation);
define_custom_trait!(CustomFieldType);
define_custom_trait!(CustomConstraint);
