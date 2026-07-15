use std::any::Any;
use std::fmt::Debug;

use crate::error::{RenderError, RenderResult};
use crate::render::ctx::RenderCtx;
use crate::render::renderer::Renderer;

// ---------------------------------------------------------------------------
// Macro to define Custom* traits with common boilerplate
// ---------------------------------------------------------------------------

macro_rules! define_custom_trait {
    ($trait_name:ident) => {
        pub trait $trait_name: Debug + Send + Sync {
            fn as_any(&self) -> &dyn Any;
            fn clone_box(&self) -> Box<dyn $trait_name>;

            /// Render this node. The extension lives in the node itself, not in a renderer
            /// wrapped around the dialect: rendering recurses back through `renderer`, so
            /// nested expressions, identifier quoting and parameter numbering all go
            /// through the same dialect and the same [`RenderCtx`] as everything else.
            ///
            /// The default is an error, keeping the contract that an unrenderable custom
            /// node fails loudly rather than silently dropping SQL.
            fn render(&self, renderer: &dyn Renderer, ctx: &mut RenderCtx) -> RenderResult<()> {
                let _ = (renderer, ctx);
                Err(RenderError::unsupported(
                    stringify!($trait_name),
                    concat!(
                        "implement ",
                        stringify!($trait_name),
                        "::render to teach the renderer this node"
                    ),
                ))
            }
        }

        impl Clone for Box<dyn $trait_name> {
            fn clone(&self) -> Self {
                self.clone_box()
            }
        }
    };
}

define_custom_trait!(CustomCondition);
define_custom_trait!(CustomCompareOp);
define_custom_trait!(CustomTableSource);
define_custom_trait!(CustomMutation);
define_custom_trait!(CustomSchemaMutation);
define_custom_trait!(CustomFieldType);
define_custom_trait!(CustomBinaryOp);
define_custom_trait!(CustomConstraint);
define_custom_trait!(CustomTransaction);

/// A user-defined expression node.
///
/// Defined by hand rather than through `define_custom_trait!` because, unlike the other
/// custom nodes, an expression can appear as the operand of an operator and therefore has
/// to answer whether it needs brackets there.
pub trait CustomExpr: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomExpr>;

    /// Render this node. See [`CustomCondition::render`] — same contract: the node renders
    /// itself and recurses back through `renderer` for any sub-expression it holds.
    fn render(&self, renderer: &dyn Renderer, ctx: &mut RenderCtx) -> RenderResult<()> {
        let _ = (renderer, ctx);
        Err(RenderError::unsupported(
            "CustomExpr",
            "implement CustomExpr::render to teach the renderer this node",
        ))
    }

    /// Whether this node needs brackets when it is the operand of an operator
    /// (`+`, `::`, `COLLATE`, a comparison, …). Only the author knows the shape it renders.
    ///
    /// Defaults to `true` — conservative, because a node rendering an infix form
    /// (`x AT TIME ZONE 'UTC'`) would otherwise be re-associated by the engine's
    /// precedence, which is the exact class of bug operand bracketing exists to prevent.
    /// A node rendering a self-delimiting form (`my_func(x)`) should return `false`.
    fn needs_operand_parens(&self) -> bool {
        true
    }
}

impl Clone for Box<dyn CustomExpr> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
