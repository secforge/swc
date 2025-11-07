//! Decorator transformation support
//!
//! This module contains decorator transformation logic ported from oxc.
//!
//! ## Implementation Status
//!
//! This is a port of oxc's decorator transformation to work with SWC's AST
//! types. The main differences from the oxc implementation:
//!
//! - Uses `swc_ecma_ast` types instead of `oxc_ast`
//! - Uses `swc_ecma_hooks::VisitMutHook` instead of `oxc_traverse::Traverse`
//! - Uses owned types (Box/Vec) instead of arena allocation with lifetimes
//!
//! ## Architecture
//!
//! The decorator module is organized into:
//! - `options.rs` - Configuration for decorator transformation
//! - `legacy/` - Legacy (experimental) decorator implementation
//!   - `mod.rs` - Main transformation logic
//!   - `metadata.rs` - TypeScript metadata emission support
//!
//! ## Key Features
//!
//! ### Legacy Decorators
//!
//! Transforms legacy (experimental) decorators to `_decorate` and
//! `_decorateParam` helper calls. This includes:
//! - Class decorators
//! - Method decorators (including getters/setters)
//! - Property decorators
//! - Parameter decorators
//! - TypeScript decorator metadata emission (`emitDecoratorMetadata`)
//!
//! ### TypeScript Metadata
//!
//! When `emit_decorator_metadata` is enabled, the transformer emits design-time
//! type information for:
//! - `design:type` - The type of a property or method
//! - `design:paramtypes` - Parameter types of a method
//! - `design:returntype` - Return type of a method
//!
//! ## Usage
//!
//! ```ignore
//! use swc_ecma_compiler::compat::{Decorator, DecoratorOptions, TransformCtx};
//!
//! let options = DecoratorOptions {
//!     legacy: true,
//!     emit_decorator_metadata: true,
//! };
//!
//! let ctx = TransformCtx::new(&transform_options);
//! let mut decorator = Decorator::new(options, &ctx);
//!
//! // Use as a VisitMutHook
//! ```
//!
//! ## Limitations
//!
//! Compared to TypeScript's decorator metadata emission:
//! - We cannot infer the exact type of type aliases (e.g., `type Foo = string`)
//! - Type references generate runtime checks instead of compile-time resolution
//!
//! These limitations are shared with other transpilers like SWC and are
//! generally acceptable for frameworks like NestJS that rely on decorator
//! metadata.

mod legacy;
mod options;

use legacy::LegacyDecorator;
pub use options::DecoratorOptions;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::TransformCtx;

/// Decorator transformation
///
/// This struct manages decorator transformations, delegating to the appropriate
/// implementation based on options (currently only legacy decorators are
/// supported).
pub struct Decorator<'ctx> {
    options: DecoratorOptions,
    legacy_decorator: LegacyDecorator<'ctx>,
}

impl<'ctx> Decorator<'ctx> {
    /// Create a new Decorator transformer with the given options
    ///
    /// # Arguments
    ///
    /// * `options` - Decorator transformation options
    /// * `ctx` - Transform context providing shared state and utilities
    ///
    /// # Example
    ///
    /// ```ignore
    /// let options = DecoratorOptions {
    ///     legacy: true,
    ///     emit_decorator_metadata: true,
    /// };
    /// let decorator = Decorator::new(options, &ctx);
    /// ```
    pub fn new(options: DecoratorOptions, ctx: &'ctx TransformCtx) -> Self {
        Self {
            legacy_decorator: LegacyDecorator::new(options.emit_decorator_metadata, ctx),
            options,
        }
    }
}

impl VisitMutHook for Decorator<'_> {
    fn enter_stmt(&mut self, stmt: &mut Stmt) {
        if self.options.legacy {
            self.legacy_decorator.enter_stmt(stmt);
        }
    }

    fn enter_class(&mut self, class: &mut Class) {
        if self.options.legacy {
            self.legacy_decorator.enter_class(class);
        }
    }

    fn exit_class(&mut self, class: &mut Class) {
        if self.options.legacy {
            self.legacy_decorator.exit_class(class);
        }
    }

    fn enter_class_method(&mut self, method: &mut ClassMethod) {
        if self.options.legacy {
            self.legacy_decorator.enter_class_method(method);
        }
    }

    fn exit_class_method(&mut self, method: &mut ClassMethod) {
        if self.options.legacy {
            self.legacy_decorator.exit_class_method(method);
        }
    }

    fn enter_class_prop(&mut self, prop: &mut ClassProp) {
        if self.options.legacy {
            self.legacy_decorator.enter_class_prop(prop);
        }
    }

    fn exit_class_prop(&mut self, prop: &mut ClassProp) {
        if self.options.legacy {
            self.legacy_decorator.exit_class_prop(prop);
        }
    }

    fn enter_private_prop(&mut self, prop: &mut PrivateProp) {
        if self.options.legacy {
            self.legacy_decorator.enter_private_prop(prop);
        }
    }

    fn exit_private_prop(&mut self, prop: &mut PrivateProp) {
        if self.options.legacy {
            self.legacy_decorator.exit_private_prop(prop);
        }
    }
}
