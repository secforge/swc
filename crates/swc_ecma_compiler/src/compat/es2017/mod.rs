#![allow(dead_code)]
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

pub mod async_to_generator;
mod options;

pub use async_to_generator::{AsyncGeneratorExecutor, AsyncToGenerator};
pub use options::ES2017Options;

/// ES2017 transformation handler.
///
/// This struct orchestrates all ES2017 transformations including:
/// - Async/await to generator transformation
pub struct ES2017<'ctx> {
    options: ES2017Options,

    // Plugins
    async_to_generator: AsyncToGenerator<'ctx>,
}

impl<'ctx> ES2017<'ctx> {
    /// Create a new ES2017 transformer with the given options.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for ES2017 transformations
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ES2017Options, ctx: &'ctx TransformCtx) -> Self {
        Self {
            options,
            async_to_generator: AsyncToGenerator::new(ctx),
        }
    }
}

impl VisitMutHook for ES2017<'_> {
    fn exit_expr(&mut self, expr: &mut Expr) {
        if self.options.async_to_generator {
            self.async_to_generator.exit_expr(expr);
        }
    }

    fn exit_function(&mut self, func: &mut Function) {
        if self.options.async_to_generator {
            self.async_to_generator.exit_function(func);
        }
    }

    fn exit_stmt(&mut self, stmt: &mut Stmt) {
        if self.options.async_to_generator {
            self.async_to_generator.exit_stmt(stmt);
        }
    }
}
