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
use swc_ecma_visit::{VisitMut, VisitMutWith};

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

impl VisitMut for ES2020<'_> {
    fn visit_mut_module(&mut self, module: &mut Module) {
        // Apply export namespace from transformation first (module-level)
        if self.options.export_namespace_from {
            self.export_namespace_from.visit_mut_module(module);
        }

        // Then visit children for expression-level transformations
        module.visit_mut_children_with(self);
    }

    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        // Visit children first
        expr.visit_mut_children_with(self);

        // Apply nullish coalescing operator transformation
        if self.options.nullish_coalescing_operator {
            self.nullish_coalescing_operator.visit_mut_expr(expr);
        }

        // Apply optional chaining transformation
        if self.options.optional_chaining {
            self.optional_chaining.visit_mut_expr(expr);
        }
    }

    fn visit_mut_param(&mut self, param: &mut Param) {
        if self.options.optional_chaining {
            self.optional_chaining.visit_mut_param(param);
        } else {
            param.visit_mut_children_with(self);
        }
    }

    fn visit_mut_big_int(&mut self, big_int: &mut BigInt) {
        if self.options.big_int {
            self.ctx.error(format!(
                "Big integer literals are not available in the configured target environment. \
                 Found at span: {:?}",
                big_int.span
            ));
        }
    }

    fn visit_mut_import_named_specifier(&mut self, specifier: &mut ImportNamedSpecifier) {
        if self.options.arbitrary_module_namespace_names {
            if let Some(ModuleExportName::Str(_)) = &specifier.imported {
                self.ctx.error(format!(
                    "Arbitrary module namespace identifier names are not available in the \
                     configured target environment. Found at span: {:?}",
                    specifier.span
                ));
            }
        }
        specifier.visit_mut_children_with(self);
    }

    fn visit_mut_export_named_specifier(&mut self, specifier: &mut ExportNamedSpecifier) {
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
        specifier.visit_mut_children_with(self);
    }

    fn visit_mut_export_all(&mut self, export_all: &mut ExportAll) {
        // TODO: Add proper warning for arbitrary module namespace names if needed
        export_all.visit_mut_children_with(self);
    }
}
