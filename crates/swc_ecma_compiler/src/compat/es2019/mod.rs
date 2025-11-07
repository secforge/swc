//! ES2019 compatibility transformations.
//!
//! This module contains transformations that convert ES2019 syntax to
//! ES2018-compatible code. ES2019 introduced several features including:
//!
//! - Optional catch binding
//! - Array.prototype.{flat, flatMap}
//! - Object.fromEntries
//! - String.prototype.{trimStart, trimEnd}
//! - Symbol.prototype.description
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
//! Currently implemented:
//! - Optional catch binding
//!
//! ## References
//!
//! - ES2019 specification: <https://262.ecma-international.org/10.0/>
//! - Babel preset-env: <https://babeljs.io/docs/babel-preset-env>

#![allow(dead_code)]
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

mod optional_catch_binding;
mod options;

pub use optional_catch_binding::OptionalCatchBinding;
pub use options::ES2019Options;

/// ES2019 transformer combining all ES2019 compatibility transformations.
///
/// This struct orchestrates the various ES2019 transformations and applies
/// them in the correct order. It follows SWC's VisitMutHook pattern rather than
/// oxc's Traverse pattern.
pub struct ES2019<'ctx> {
    options: ES2019Options,

    // Plugins
    optional_catch_binding: OptionalCatchBinding<'ctx>,
}

impl<'ctx> ES2019<'ctx> {
    /// Create a new ES2019 transformer.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration for ES2019 transformations
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ES2019Options, ctx: &'ctx TransformCtx) -> Self {
        Self {
            optional_catch_binding: OptionalCatchBinding::new(ctx),
            options,
        }
    }
}

impl VisitMutHook for ES2019<'_> {
    fn enter_catch_clause(&mut self, clause: &mut CatchClause) {
        if self.options.optional_catch_binding {
            self.optional_catch_binding.enter_catch_clause(clause);
        }
    }
}
