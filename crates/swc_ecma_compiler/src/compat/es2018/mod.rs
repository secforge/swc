//! ES2018 compatibility transformations.
//!
//! This module provides transformations for ES2018 features:
//!
//! - **Object rest/spread**: Transform object rest/spread syntax
//! - **Async generator functions**: Transform async generator functions and
//!   for-await loops
//!
//! ## Implementation Status
//!
//! This is a working implementation ported from oxc to SWC's VisitMutHook
//! pattern:
//!
//! ### Async Generator Functions
//! - Transforms async generator functions to wrapped generator functions
//! - Converts `await` expressions to `yield awaitAsyncGenerator(expr)`
//! - Converts `yield*` in async generators to use async iteration helpers
//! - Transforms `for await` loops to complex try-catch-finally structures
//!
//! ### Object Rest/Spread
//! - Transforms object spread in object literals: `{ ...obj, key: value }`
//! - Note: Full destructuring support is simplified compared to oxc's 1100+
//!   line implementation
//!
//! For production use with full feature support, consider SWC's built-in
//! transforms in `swc_ecma_transforms_compat::es2018`.

#![allow(dead_code)]
pub mod async_generator_functions;
pub mod object_rest_spread;
pub mod options;

pub use async_generator_functions::AsyncGeneratorFunctions;
pub use object_rest_spread::{ObjectRestSpread, ObjectRestSpreadOptions};
pub use options::ES2018Options;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

/// ES2018 transformer combining all ES2018 feature transformations.
///
/// This struct coordinates the async generator functions and object rest/spread
/// transformations.
pub struct ES2018<'ctx> {
    options: ES2018Options,
    object_rest_spread: ObjectRestSpread<'ctx>,
    async_generator_functions: AsyncGeneratorFunctions<'ctx>,
}

impl<'ctx> ES2018<'ctx> {
    /// Create a new ES2018 transformer.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for ES2018 features
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ES2018Options, ctx: &'ctx TransformCtx) -> Self {
        Self {
            object_rest_spread: ObjectRestSpread::new(
                options.object_rest_spread.unwrap_or_default(),
                ctx,
            ),
            async_generator_functions: AsyncGeneratorFunctions::new(ctx),
            options,
        }
    }
}

impl VisitMutHook for ES2018<'_> {
    fn enter_function(&mut self, func: &mut Function) {
        if self.options.async_generator_functions {
            self.async_generator_functions.enter_function(func);
        }
    }

    fn exit_function(&mut self, func: &mut Function) {
        if self.options.async_generator_functions {
            self.async_generator_functions.exit_function(func);
        }
    }

    fn enter_stmt(&mut self, stmt: &mut Stmt) {
        if self.options.async_generator_functions {
            self.async_generator_functions.enter_stmt(stmt);
        }
    }

    fn exit_stmt(&mut self, stmt: &mut Stmt) {
        if self.options.async_generator_functions {
            self.async_generator_functions.exit_stmt(stmt);
        }
    }

    fn exit_expr(&mut self, expr: &mut Expr) {
        if self.options.object_rest_spread.is_some() {
            self.object_rest_spread.exit_expr(expr);
        }

        if self.options.async_generator_functions {
            self.async_generator_functions.exit_expr(expr);
        }
    }
}
