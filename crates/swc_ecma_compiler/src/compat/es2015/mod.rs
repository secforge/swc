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
//! allocation, this SWC-based implementation uses the `VisitMut` trait with
//! owned types. Each transformation is implemented as a separate visitor that
//! can be composed together.
//!
//! ## Current Status
//!
//! This is an initial port from the oxc architecture. Currently implemented:
//! - Arrow functions (structure only, implementation pending)
//!
//! ## References
//!
//! - ES2015 specification: <https://262.ecma-international.org/6.0/>
//! - Babel preset-env: <https://babeljs.io/docs/babel-preset-env>

use swc_ecma_visit::VisitMut;

use crate::compat::context::TransformCtx;

mod arrow_functions;
mod options;

pub use arrow_functions::{ArrowFunctions, ArrowFunctionsOptions};
pub use options::ES2015Options;

/// ES2015 transformer combining all ES2015 compatibility transformations.
///
/// This struct orchestrates the various ES2015 transformations and applies
/// them in the correct order. It follows SWC's VisitMut pattern rather than
/// oxc's Traverse pattern.
pub struct ES2015<'ctx> {
    #[allow(dead_code)]
    options: ES2015Options,

    // Plugins
    #[allow(dead_code)]
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

impl VisitMut for ES2015<'_> {
    // TODO: Implement ES2015 transformations using SWC's VisitMut pattern.
    // For now, this is a stub that does nothing.
    // The actual implementation should delegate to the individual transformation
    // plugins (arrow_functions, etc.) in the correct order.
}
