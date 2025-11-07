//! ES2015 (ES6) compatibility transformations.
//!
//! This module contains transformations that convert ES2015 (ES6) syntax
//! to ES5-compatible code. ES2015 introduced many significant features to
//! JavaScript including:
//!
//! - Arrow functions
//! - Classes
//! - Template literals
//! - Destructuring
//! - Default parameters
//! - Rest/spread operators
//! - Block-scoped declarations (let/const)
//! - Modules (import/export)
//! - And more
//!
//! ## Architecture
//!
//! Unlike the oxc implementation which uses the `Traverse` trait with arena
//! allocation, this SWC-based implementation uses the `VisitMutHook` trait with
//! owned types. Each transformation is implemented as a separate visitor that
//! can be composed together using the hook composition system.
//!
//! ## Current Status
//!
//! This is a port from the oxc architecture. Currently implemented:
//! - Arrow functions (basic transformation complete)
//!
//! ## References
//!
//! - ES2015 specification: <https://262.ecma-international.org/6.0/>
//! - Babel preset-env: <https://babeljs.io/docs/babel-preset-env>

#![allow(dead_code)]
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

mod arrow_functions;
mod options;

pub use arrow_functions::{ArrowFunctions, ArrowFunctionsOptions};
pub use options::ES2015Options;

/// ES2015 transformer combining all ES2015 compatibility transformations.
///
/// This struct orchestrates the various ES2015 transformations and applies
/// them in the correct order. It follows SWC's VisitMutHook pattern rather than
/// oxc's Traverse pattern.
pub struct ES2015<'ctx> {
    #[allow(dead_code)]
    options: ES2015Options,

    // Plugins
    arrow_functions: ArrowFunctions<'ctx>,
}

impl<'ctx> ES2015<'ctx> {
    /// Create a new ES2015 transformer.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration for ES2015 transformations
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ES2015Options, ctx: &'ctx TransformCtx) -> Self {
        Self {
            arrow_functions: ArrowFunctions::new(options.arrow_function.unwrap_or_default(), ctx),
            options,
        }
    }
}

impl VisitMutHook for ES2015<'_> {
    // Delegate to arrow functions transformer

    fn exit_expr(&mut self, expr: &mut swc_ecma_ast::Expr) {
        self.arrow_functions.exit_expr(expr);
    }

    fn enter_fn_decl(&mut self, func: &mut swc_ecma_ast::FnDecl) {
        self.arrow_functions.enter_fn_decl(func);
    }

    fn exit_fn_decl(&mut self, func: &mut swc_ecma_ast::FnDecl) {
        self.arrow_functions.exit_fn_decl(func);
    }

    fn enter_fn_expr(&mut self, func: &mut swc_ecma_ast::FnExpr) {
        self.arrow_functions.enter_fn_expr(func);
    }

    fn exit_fn_expr(&mut self, func: &mut swc_ecma_ast::FnExpr) {
        self.arrow_functions.exit_fn_expr(func);
    }

    fn enter_this_expr(&mut self, this_expr: &mut swc_ecma_ast::ThisExpr) {
        self.arrow_functions.enter_this_expr(this_expr);
    }

    fn enter_module(&mut self, module: &mut swc_ecma_ast::Module) {
        self.arrow_functions.enter_module(module);
    }

    fn exit_module(&mut self, module: &mut swc_ecma_ast::Module) {
        self.arrow_functions.exit_module(module);
    }

    fn enter_block_stmt(&mut self, block: &mut swc_ecma_ast::BlockStmt) {
        self.arrow_functions.enter_block_stmt(block);
    }

    fn exit_block_stmt(&mut self, block: &mut swc_ecma_ast::BlockStmt) {
        self.arrow_functions.exit_block_stmt(block);
    }
}
