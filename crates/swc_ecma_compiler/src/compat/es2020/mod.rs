//! ES2020 compatibility transformations.
//!
//! This module contains transformations that convert ES2020 syntax to
//! ES5-compatible code. ES2020 introduced several important features:
//!
//! - Export namespace from (`export * as ns from "mod"`)
//! - Nullish coalescing operator (`??`)
//! - Optional chaining (`?.`)
//! - BigInt (warning only)
//! - Arbitrary module namespace names (warning only)
//!
//! ## Architecture
//!
//! This is a SWC-based implementation ported from the oxc architecture. Unlike
//! oxc which uses the `Traverse` trait with arena allocation, this
//! implementation uses SWC's `VisitMut` trait with owned types.
//!
//! ## Current Status
//!
//! This module has been ported from oxc to SWC. Key transformations
//! implemented:
//! - Export namespace from
//! - Nullish coalescing operator
//! - Optional chaining (simplified implementation)
//!
//! ## References
//!
//! - ES2020 specification: <https://262.ecma-international.org/11.0/>
//! - Babel preset-env: <https://babeljs.io/docs/babel-preset-env>

use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

mod export_namespace_from;
mod nullish_coalescing_operator;
mod optional_chaining;
mod options;

pub use export_namespace_from::ExportNamespaceFrom;
pub use nullish_coalescing_operator::NullishCoalescingOperator;
pub use optional_chaining::OptionalChaining;
pub use options::ES2020Options;

/// ES2020 transformer combining all ES2020 compatibility transformations.
///
/// This struct orchestrates the various ES2020 transformations and applies them
/// in the correct order. It follows SWC's VisitMut pattern.
pub struct ES2020<'ctx> {
    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,
    options: ES2020Options,

    // Plugins
    export_namespace_from: ExportNamespaceFrom,
    nullish_coalescing_operator: NullishCoalescingOperator<'ctx>,
    optional_chaining: OptionalChaining<'ctx>,
}

impl<'ctx> ES2020<'ctx> {
    /// Create a new ES2020 transformer.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration for ES2020 transformations
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ES2020Options, ctx: &'ctx TransformCtx) -> Self {
        Self {
            ctx,
            options,
            export_namespace_from: ExportNamespaceFrom::new(),
            nullish_coalescing_operator: NullishCoalescingOperator::new(ctx),
            optional_chaining: OptionalChaining::new(ctx),
        }
    }
}

impl VisitMutHook for ES2020<'_> {
    fn enter_module(&mut self, module: &mut Module) {
        if self.options.nullish_coalescing_operator {
            self.nullish_coalescing_operator.enter_module(module);
        }
        if self.options.optional_chaining {
            self.optional_chaining.enter_module(module);
        }
    }

    fn exit_module(&mut self, module: &mut Module) {
        // Apply exit hooks for expression-level transformations first
        if self.options.nullish_coalescing_operator {
            self.nullish_coalescing_operator.exit_module(module);
        }
        if self.options.optional_chaining {
            self.optional_chaining.exit_module(module);
        }

        // Apply export namespace from transformation last (module-level)
        if self.options.export_namespace_from {
            self.export_namespace_from.exit_module(module);
        }
    }

    fn enter_block_stmt(&mut self, block: &mut BlockStmt) {
        if self.options.nullish_coalescing_operator {
            self.nullish_coalescing_operator.enter_block_stmt(block);
        }
        if self.options.optional_chaining {
            self.optional_chaining.enter_block_stmt(block);
        }
    }

    fn exit_block_stmt(&mut self, block: &mut BlockStmt) {
        if self.options.nullish_coalescing_operator {
            self.nullish_coalescing_operator.exit_block_stmt(block);
        }
        if self.options.optional_chaining {
            self.optional_chaining.exit_block_stmt(block);
        }
    }

    fn enter_expr(&mut self, expr: &mut Expr) {
        if self.options.optional_chaining {
            self.optional_chaining.enter_expr(expr);
        }
    }

    fn exit_expr(&mut self, expr: &mut Expr) {
        // Apply nullish coalescing operator transformation first
        if self.options.nullish_coalescing_operator {
            self.nullish_coalescing_operator.exit_expr(expr);
        }

        // Apply optional chaining transformation after
        if self.options.optional_chaining {
            self.optional_chaining.exit_expr(expr);
        }
    }

    fn enter_param(&mut self, param: &mut Param) {
        if self.options.optional_chaining {
            self.optional_chaining.enter_param(param);
        }
    }

    fn exit_param(&mut self, param: &mut Param) {
        if self.options.optional_chaining {
            self.optional_chaining.exit_param(param);
        }
    }

    fn enter_big_int(&mut self, big_int: &mut BigInt) {
        if self.options.big_int {
            self.ctx.error(format!(
                "Big integer literals are not available in the configured target environment. \
                 Found at span: {:?}",
                big_int.span
            ));
        }
    }

    fn enter_import_named_specifier(&mut self, specifier: &mut ImportNamedSpecifier) {
        if self.options.arbitrary_module_namespace_names {
            if let Some(ModuleExportName::Str(_)) = &specifier.imported {
                self.ctx.error(format!(
                    "Arbitrary module namespace identifier names are not available in the \
                     configured target environment. Found at span: {:?}",
                    specifier.span
                ));
            }
        }
    }

    fn enter_export_named_specifier(&mut self, specifier: &mut ExportNamedSpecifier) {
        if self.options.arbitrary_module_namespace_names {
            if let Some(ModuleExportName::Str(_)) = &specifier.exported {
                self.ctx.error(format!(
                    "Arbitrary module namespace identifier names are not available in the \
                     configured target environment. Found at span: {:?}",
                    specifier.span
                ));
            }
            if let ModuleExportName::Str(_) = &specifier.orig {
                self.ctx.error(format!(
                    "Arbitrary module namespace identifier names are not available in the \
                     configured target environment. Found at span: {:?}",
                    specifier.span
                ));
            }
        }
    }
}
