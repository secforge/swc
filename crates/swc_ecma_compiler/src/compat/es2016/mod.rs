//! ES2016 (ES7) compatibility transformations.
//!
//! This module contains transformations that convert ES2016 (ES7) syntax
//! to ES5-compatible code. ES2016 introduced:
//!
//! - Exponentiation operator (`**` and `**=`)
//! - Array.prototype.includes
//!
//! ## Architecture
//!
//! Unlike the oxc implementation which uses the `Traverse` trait with arena
//! allocation, this SWC-based implementation uses the `VisitMutHook` trait with
//! owned types. Each transformation is implemented as a separate hook that
//! can be composed together.
//!
//! ## Current Status
//!
//! This is a port from the oxc architecture. Currently implemented:
//! - Exponentiation operator transformation
//!
//! ## References
//!
//! - ES2016 specification: <https://262.ecma-international.org/7.0/>
//! - Babel preset-env: <https://babeljs.io/docs/babel-preset-env>

use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

mod exponentiation_operator;
mod options;

pub use exponentiation_operator::ExponentiationOperator;
pub use options::ES2016Options;

/// ES2016 transformer combining all ES2016 compatibility transformations.
///
/// This struct orchestrates the various ES2016 transformations and applies
/// them in the correct order. It follows SWC's VisitMutHook pattern rather than
/// oxc's Traverse pattern.
pub struct ES2016<'ctx> {
    options: ES2016Options,

    // Plugins
    exponentiation_operator: ExponentiationOperator<'ctx>,
}

impl<'ctx> ES2016<'ctx> {
    /// Create a new ES2016 transformer.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration for ES2016 transformations
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ES2016Options, ctx: &'ctx TransformCtx) -> Self {
        Self {
            exponentiation_operator: ExponentiationOperator::new(ctx),
            options,
        }
    }
}

impl VisitMutHook for ES2016<'_> {
    fn enter_expr(&mut self, expr: &mut swc_ecma_ast::Expr) {
        if self.options.exponentiation_operator {
            self.exponentiation_operator.enter_expr(expr);
        }
    }
}
