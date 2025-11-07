//! ES2026 transformations
//!
//! This module contains transformations for ES2026 features, ported from oxc.
//!
//! ## Features
//!
//! - Explicit Resource Management (`using` and `await using` declarations)
//!
//! ## Implementation
//!
//! This is a port of oxc's ES2026 transformation to work with SWC's AST types.
//! The main differences from the oxc implementation:
//!
//! - Uses `swc_ecma_ast` types instead of `oxc_ast`
//! - Uses `swc_ecma_hooks::VisitMutHook` instead of `oxc_traverse::Traverse`
//! - Uses owned types instead of arena allocation with lifetimes
//! - SWC has a separate `UsingDecl` AST node (better than oxc's approach)
//! - Helper loading is adapted to SWC's infrastructure
//!
//! ## References
//!
//! * Babel plugin implementation: <https://github.com/babel/babel/blob/v7.26.9/packages/babel-plugin-transform-explicit-resource-management>
//! * Explicit Resource Management TC39 proposal: <https://github.com/tc39/proposal-explicit-resource-management>

#![allow(dead_code)]
mod explicit_resource_management;
mod options;

use explicit_resource_management::ExplicitResourceManagement;
pub use options::ES2026Options;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::TransformCtx;

/// ES2026 transformation
///
/// This struct manages ES2026 feature transformations, delegating to the
/// appropriate sub-transformations based on options.
pub struct ES2026<'ctx> {
    explicit_resource_management: Option<ExplicitResourceManagement<'ctx>>,
}

impl<'ctx> ES2026<'ctx> {
    /// Create a new ES2026 transformer with the given options
    ///
    /// # Arguments
    /// * `options` - Configuration for ES2026 transformations
    /// * `ctx` - Transform context for accessing compiler state
    pub fn new(options: ES2026Options, ctx: &'ctx TransformCtx) -> Self {
        let explicit_resource_management = if options.explicit_resource_management {
            Some(ExplicitResourceManagement::new(ctx))
        } else {
            None
        };
        Self {
            explicit_resource_management,
        }
    }
}

impl VisitMutHook for ES2026<'_> {
    // Delegate to ExplicitResourceManagement if enabled
    // These methods follow the enter_/exit_ pattern from oxc's Traverse trait

    fn enter_module(&mut self, n: &mut swc_ecma_ast::Module) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.enter_module(n);
        }
    }

    fn enter_script(&mut self, n: &mut swc_ecma_ast::Script) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.enter_script(n);
        }
    }

    fn enter_for_of_stmt(&mut self, n: &mut swc_ecma_ast::ForOfStmt) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.enter_for_of_stmt(n);
        }
    }

    fn enter_stmt(&mut self, n: &mut swc_ecma_ast::Stmt) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.enter_stmt(n);
        }
    }

    fn enter_try_stmt(&mut self, n: &mut swc_ecma_ast::TryStmt) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.enter_try_stmt(n);
        }
    }

    fn enter_function(&mut self, n: &mut swc_ecma_ast::Function) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.enter_function(n);
        }
    }

    fn exit_static_block(&mut self, n: &mut swc_ecma_ast::StaticBlock) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.exit_static_block(n);
        }
    }
}
