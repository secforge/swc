//! Legacy decorator transformation
//!
//! This module transforms legacy decorators by calling `_decorate` and
//! `_decorateParam` helpers to apply decorators.
//!
//! ## Examples
//!
//! Input:
//! ```ts
//! @dec
//! class Class {
//!   @dec
//!   prop = 0;
//!
//!   @dec
//!   method(@dec param) {}
//! }
//! ```
//!
//! Output:
//! ```js
//! let Class = class Class {
//!   prop = 0;
//!   method(param) {}
//! };
//!
//! _decorate([dec], Class.prototype, "method", null);
//!
//! _decorate([
//!   _decorateParam(0, dec)
//! ], Class.prototype, "method", null);
//!
//! Class = _decorate([dec], Class);
//! ```
//!
//! ## Implementation
//!
//! This is a port of the oxc legacy decorator implementation to SWC.
//! The original implementation is based on [TypeScript Experimental Decorators](https://github.com/microsoft/TypeScript/blob/d85767abfd83880cea17cea70f9913e9c4496dcc/src/compiler/transformers/legacyDecorators.ts).
//!
//! ## Porting Status
//!
//! **TODO**: This is a stub implementation. The full porting requires:
//!
//! 1. Converting oxc's arena-allocated AST types to SWC's owned types
//! 2. Adapting oxc's Traverse pattern to SWC's VisitMut pattern
//! 3. Porting helper infrastructure:
//!    - BoundIdentifier management
//!    - Statement injection
//!    - Variable declaration hoisting
//!    - Module imports
//! 4. Porting metadata emission (see metadata.rs)
//! 5. Adapting symbol/scope management from oxc_semantic to SWC
//!
//! The original oxc implementation (~1250 lines) includes:
//! - Class decoration state tracking with stack-based management
//! - Transform of class declarations, exports (default and named)
//! - Method, property, and accessor decorations
//! - Parameter decorations
//! - Private-in-expression detection for static block placement
//! - Class reference alias handling to avoid double-binding issues
//!
//! ## References
//! * TypeScript Experimental Decorators documentation: <https://www.typescriptlang.org/docs/handbook/decorators.html>

mod metadata;

use metadata::LegacyDecoratorMetadata;
use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

/// Legacy decorator transformer
///
/// Handles transformation of legacy (experimental) decorators
pub struct LegacyDecorator {
    emit_decorator_metadata: bool,
    metadata: LegacyDecoratorMetadata,
}

impl LegacyDecorator {
    /// Create a new legacy decorator transformer
    pub fn new(emit_decorator_metadata: bool) -> Self {
        Self {
            emit_decorator_metadata,
            metadata: LegacyDecoratorMetadata::new(),
        }
    }
}

impl VisitMut for LegacyDecorator {
    fn visit_mut_module(&mut self, n: &mut Module) {
        // TODO: Implement legacy decorator transformation
        // This should:
        // 1. Visit all classes and collect decorators
        // 2. Transform decorated classes to variable declarations
        // 3. Generate _decorate helper calls
        // 4. Handle exports properly
        n.visit_mut_children_with(self);

        if self.emit_decorator_metadata {
            self.metadata.visit_mut_module(n);
        }
    }

    fn visit_mut_script(&mut self, n: &mut Script) {
        // TODO: Implement legacy decorator transformation for scripts
        n.visit_mut_children_with(self);

        if self.emit_decorator_metadata {
            self.metadata.visit_mut_script(n);
        }
    }

    fn visit_mut_class(&mut self, n: &mut Class) {
        // TODO: Implement class-level decorator transformation
        // This should:
        // 1. Check if class has decorators or decorated members
        // 2. Transform class structure if needed
        // 3. Generate decorator application statements
        // 4. Handle class references and aliasing
        n.visit_mut_children_with(self);

        if self.emit_decorator_metadata {
            self.metadata.visit_mut_class(n);
        }
    }

    fn visit_mut_class_method(&mut self, n: &mut ClassMethod) {
        // TODO: Implement method decorator transformation
        n.visit_mut_children_with(self);

        if self.emit_decorator_metadata {
            self.metadata.visit_mut_class_method(n);
        }
    }

    fn visit_mut_class_prop(&mut self, n: &mut ClassProp) {
        // TODO: Implement property decorator transformation
        n.visit_mut_children_with(self);

        if self.emit_decorator_metadata {
            self.metadata.visit_mut_class_prop(n);
        }
    }

    fn visit_mut_private_prop(&mut self, n: &mut PrivateProp) {
        // TODO: Implement private property decorator transformation
        n.visit_mut_children_with(self);

        if self.emit_decorator_metadata {
            self.metadata.visit_mut_private_prop(n);
        }
    }

    fn visit_mut_decorator(&mut self, n: &mut Decorator) {
        // TODO: Process decorator expressions
        // Check for private-in expressions
        n.visit_mut_children_with(self);
    }
}
