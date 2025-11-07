//! ES2021 compatibility transformations.
//!
//! This module contains transformations that convert ES2021 syntax
//! to ES5-compatible code. ES2021 introduced:
//!
//! - Logical assignment operators (`&&=`, `||=`, `??=`)
//! - Numeric separators
//! - String.prototype.replaceAll
//! - Promise.any
//! - WeakRefs
//!
//! ## Architecture
//!
//! Unlike the oxc implementation which uses the `Traverse` trait with arena
//! allocation, this SWC-based implementation uses the `VisitMutHook` trait with
//! owned types. Each transformation is implemented as a separate visitor that
//! can be composed together.
//!
//! ## Current Status
//!
//! This is a port from the oxc architecture. Currently implemented:
//! - Logical assignment operators
//!
//! ## References
//!
//! - ES2021 specification: <https://262.ecma-international.org/12.0/>
//! - Babel preset-env: <https://babeljs.io/docs/babel-preset-env>

use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

mod logical_assignment_operators;
mod options;

pub use logical_assignment_operators::LogicalAssignmentOperators;
pub use options::ES2021Options;

/// ES2021 transformer combining all ES2021 compatibility transformations.
///
/// This struct orchestrates the various ES2021 transformations and applies
/// them in the correct order. It follows SWC's VisitMutHook pattern rather than
/// oxc's Traverse pattern.
pub struct ES2021<'ctx> {
    options: ES2021Options,

    // Plugins
    logical_assignment_operators: LogicalAssignmentOperators<'ctx>,
}

impl<'ctx> ES2021<'ctx> {
    /// Create a new ES2021 transformer.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration for ES2021 transformations
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ES2021Options, ctx: &'ctx TransformCtx) -> Self {
        Self {
            logical_assignment_operators: LogicalAssignmentOperators::new(ctx),
            options,
        }
    }
}

impl VisitMutHook for ES2021<'_> {
    fn enter_expr(&mut self, expr: &mut swc_ecma_ast::Expr) {
        if self.options.logical_assignment_operators {
            self.logical_assignment_operators.enter_expr(expr);
        }
    }
}
