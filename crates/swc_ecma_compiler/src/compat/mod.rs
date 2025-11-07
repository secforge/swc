// Core modules
mod common;
mod compiler_assumptions;
mod context;
mod options;
mod state;
mod utils;

// ES version modules
// mod decorator; // TODO: Fix SWC AST compatibility issues
mod decorator_stub; // Temporary stub for DecoratorOptions
mod es2015;
mod es2016;
mod es2017;
mod es2018;
mod es2019;
mod es2020;
mod es2021;
mod es2022;
mod es2026;
// mod jsx; // TODO: Fix SWC AST compatibility issues
mod jsx_stub; // Temporary stub for JSX types
mod plugins;
mod proposals;
mod regexp;
mod typescript;

// Public exports - Core
// Public exports - Helpers
pub use common::helper_loader::{Helper, HelperLoaderMode, HelperLoaderOptions};
pub use compiler_assumptions::CompilerAssumptions;
pub use context::TransformCtx;
// Public exports - Other features
pub use decorator_stub::DecoratorOptions; // Temporary stub
// Public exports - ES version options
pub use es2015::{ArrowFunctionsOptions, ES2015Options};
pub use es2016::ES2016Options;
pub use es2017::ES2017Options;
pub use es2018::ES2018Options;
pub use es2019::ES2019Options;
pub use es2020::ES2020Options;
pub use es2021::ES2021Options;
pub use es2022::{ClassPropertiesOptions, ES2022Options};
pub use es2026::ES2026Options;
pub use jsx_stub::{JsxOptions, JsxRuntime, ReactRefreshOptions}; // Temporary stub
// Public exports - Options
pub use options::{
    babel::{BabelEnvOptions, BabelOptions},
    EnvOptions, Module, TransformOptions,
};
pub use plugins::{PluginsOptions, StyledComponentsOptions};
pub use proposals::ProposalOptions;
pub use regexp::RegExpOptions;
pub use state::TransformState;
pub use typescript::TypeScriptOptions;

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
/// # Implementation Status
///
/// This is a port of oxc's transformer code to SWC. The implementation adapts
/// oxc's architecture to work with SWC's AST types and visitor patterns.
///
/// **Current Status:**
/// - ✅ ES2016: Fully ported (exponentiation operator)
/// - ✅ ES2017: Fully ported (async to generator)
/// - ⚠️  ES2018: Stub implementation (complex transforms)
/// - ✅ ES2019: Fully ported (optional catch binding)
/// - ✅ ES2020: Fully ported (nullish coalescing, optional chaining, export
///   namespace)
/// - ✅ ES2021: Fully ported (logical assignment operators)
/// - ⚠️  ES2022: Analysis only (7500+ lines, very complex class properties)
/// - ✅ ES2026: Fully ported (explicit resource management)
/// - ⚠️  ES2015: Stub implementation (use SWC's built-in transforms)
/// - ⚠️  JSX: Partial port (24% complete, use SWC's built-in transforms)
/// - ✅ Common: Fully ported (utilities and helpers)
/// - ✅ Options: Fully ported (configuration structures)
///
/// For production use, consider using SWC's mature built-in transforms for
/// ES2015, ES2018, ES2022, and JSX features.
///
/// # Note
///
/// Many modules are implemented as stubs or partial implementations due to:
/// - Complexity of the original oxc implementations (e.g., ES2022 class
///   properties: 7500+ lines)
/// - Deep integration with oxc's arena allocator and semantic analysis
/// - Availability of mature SWC implementations for the same features
///
/// The ported modules provide a foundation for understanding oxc's architecture
/// and can be incrementally enhanced as needed.
pub struct CompatCompiler {
    /// Transform context containing shared state and configuration
    pub ctx: TransformCtx,
}

impl CompatCompiler {
    /// Creates a new CompatCompiler with the given transform options.
    ///
    /// # Arguments
    ///
    /// * `options` - Transform options specifying which transforms to enable
    ///
    /// # Example
    ///
    /// ```ignore
    /// use swc_ecma_compiler::compat::{CompatCompiler, TransformOptions};
    ///
    /// let options = TransformOptions::default();
    /// let compiler = CompatCompiler::new(&options);
    /// ```
    pub fn new(options: &TransformOptions) -> Self {
        let ctx = TransformCtx::new(options);
        Self { ctx }
    }

    /// Returns a reference to the transform context.
    pub fn context(&self) -> &TransformCtx {
        &self.ctx
    }

    /// Returns a mutable reference to the transform context.
    pub fn context_mut(&mut self) -> &mut TransformCtx {
        &mut self.ctx
    }
}

impl Default for CompatCompiler {
    fn default() -> Self {
        Self::new(&TransformOptions::default())
    }
}
