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
//! - Uses `swc_ecma_visit::VisitMut` instead of `oxc_traverse::Traverse`
//! - Uses owned types instead of arena allocation with lifetimes
//!
//! ## Porting Notes
//!
//! The original oxc implementation consists of:
//! - `mod.rs` (~3.9KB): Main Decorator struct with Traverse impl
//! - `options.rs` (~976 bytes): DecoratorOptions configuration
//! - `legacy/mod.rs` (~46KB): LegacyDecorator implementation
//! - `legacy/metadata.rs` (~30KB): Metadata emission for decorators
//!
//! Key challenges in porting:
//! 1. SWC's AST types have different structure and naming conventions
//! 2. SWC uses VisitMut trait which mutates in-place, vs oxc's Traverse with
//!    arena allocation
//! 3. Helper infrastructure (var_declarations, module_imports,
//!    statement_injector) needs to be adapted
//! 4. Symbol/scope management differs between oxc_semantic and swc's symbol
//!    resolution
//!
//! TODO: Complete the porting of the decorator transformation logic

mod legacy;
mod options;

use legacy::LegacyDecorator;
pub use options::DecoratorOptions;
use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::compat::{TransformCtx, TransformState};

/// Decorator transformation
///
/// This struct manages decorator transformations, delegating to the appropriate
/// implementation based on options (currently only legacy decorators are
/// supported).
pub struct Decorator {
    options: DecoratorOptions,
    legacy_decorator: LegacyDecorator,
}

impl Decorator {
    /// Create a new Decorator transformer with the given options
    pub fn new(options: DecoratorOptions, _ctx: &TransformCtx) -> Self {
        Self {
            legacy_decorator: LegacyDecorator::new(options.emit_decorator_metadata),
            options,
        }
    }
}

impl VisitMut for Decorator {
    fn visit_mut_module(&mut self, n: &mut Module) {
        if self.options.legacy {
            self.legacy_decorator.visit_mut_module(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_script(&mut self, n: &mut Script) {
        if self.options.legacy {
            self.legacy_decorator.visit_mut_script(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_class(&mut self, n: &mut Class) {
        if self.options.legacy {
            self.legacy_decorator.visit_mut_class(n);
        }
        n.visit_mut_children_with(self);
    }
}
