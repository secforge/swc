// Core modules
mod common;
mod compiler_assumptions;
mod context;
mod state;
mod utils;

// ES version modules
mod decorator;
mod es2015;
mod es2016;
mod es2017;
mod es2018;
mod es2019;
mod es2020;
mod es2021;
mod es2022;
mod es2026;
mod jsx;
mod plugins;
mod proposals;
mod regexp;
mod typescript;

// Public exports
pub use compiler_assumptions::CompilerAssumptions;
pub use context::TransformCtx;
pub use state::TransformState;

/// CompatCompiler is a compatibility layer that ports oxc's transformer
/// architecture to SWC's AST and Visitor API.
///
/// This provides a structured approach to ECMAScript compatibility
/// transformations, organizing transforms by ES version (ES2015-ES2026) and
/// feature categories (JSX, TypeScript, decorators, etc.).
///
/// # Architecture
///
/// Unlike oxc which uses the `Traverse` trait pattern with arena allocation,
/// SWC uses the `VisitMut` trait with owned types. This compiler provides:
///
/// - **Common utilities**: Helper functions for AST manipulation, module
///   imports, statement injection, and variable declarations
/// - **Transform context**: Shared state and configuration for transformations
/// - **ES version transforms**: Organized by ECMAScript version for clarity
///
/// # Usage
///
/// The compat layer is designed to be used as building blocks for creating
/// custom transformation pipelines. Each ES version module exports specific
/// transform passes that can be composed as needed.
///
/// # Note
///
/// This is a port of oxc's transformer code to SWC. The implementation adapts
/// oxc's architecture to work with SWC's AST types and visitor patterns while
/// maintaining similar functionality and organization.
pub struct CompatCompiler {
    /// Transform context containing shared state and configuration
    pub ctx: TransformCtx,
}
