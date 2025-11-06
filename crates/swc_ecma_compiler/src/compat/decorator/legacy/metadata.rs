//! Emitting decorator metadata
//!
//! This plugin is used to emit decorator metadata for legacy decorators by
//! the `__metadata` helper.
//!
//! ## Example
//!
//! Input:
//! ```ts
//! class Demo {
//!   @LogMethod
//!   public foo(bar: number) {}
//!
//!   @Prop
//!   prop: string = "hello";
//! }
//! ```
//!
//! Output:
//! ```js
//! class Demo {
//!   foo(bar) {}
//!   prop = "hello";
//! }
//! babelHelpers.decorate([
//!   LogMethod,
//!   babelHelpers.decorateParam(0, babelHelpers.decorateMetadata("design:type", Function)),
//!   babelHelpers.decorateParam(0, babelHelpers.decorateMetadata("design:paramtypes", [Number])),
//!   babelHelpers.decorateParam(0, babelHelpers.decorateMetadata("design:returntype", void 0))
//! ], Demo.prototype, "foo", null);
//! babelHelpers.decorate([Prop, babelHelpers.decorateMetadata("design:type", String)], Demo.prototype, "prop", void 0);
//! ```
//!
//! ## Implementation
//!
//! This is a port of the oxc implementation based on
//! <https://github.com/microsoft/TypeScript/blob/d85767abfd83880cea17cea70f9913e9c4496dcc/src/compiler/transformers/ts.ts#L1119-L1136>
//!
//! ## Limitations
//!
//! ### Compared to TypeScript
//!
//! We lack the type inference ability that TypeScript has, so we cannot
//! determine the exact type of type references. For example:
//!
//! Input:
//! ```ts
//! type Foo = string;
//! class Cls {
//!   @dec
//!   p: Foo = ""
//! }
//! ```
//!
//! TypeScript Output:
//! ```js
//! class Cls {
//!   constructor() {
//!     this.p = "";
//!   }
//! }
//! __decorate([
//!   dec,
//!   __metadata("design:type", String) // Infers that Foo is String
//! ], Cls.prototype, "p", void 0);
//! ```
//!
//! Our Output:
//! ```js
//! var _ref;
//! class Cls {
//!     p = "";
//! }
//! babelHelpers.decorate([
//!   dec,
//!   babelHelpers.decorateMetadata("design:type", typeof (_ref = typeof Foo === "undefined" && Foo) === "function" ? _ref : Object)
//! ],
//! Cls.prototype, "p", void 0);
//! ```
//!
//! ### Compared to SWC
//!
//! SWC also has the above limitation. SWC provides additional support for
//! inferring enum members, which we currently do not have. The limitation may
//! not be a problem as SWC has been adopted in [NestJS](https://docs.nestjs.com/recipes/swc#jest--swc).
//!
//! ## Porting Status
//!
//! **TODO**: This is a stub implementation. The full porting requires:
//!
//! 1. Type serialization logic for all TypeScript types
//! 2. Enum type inference and tracking
//! 3. Metadata generation for:
//!    - `design:type` - Type of the member
//!    - `design:paramtypes` - Types of parameters
//!    - `design:returntype` - Return type (for methods)
//! 4. Integration with helper loader for `_metadata` helper
//! 5. Proper handling of type annotations and their serialization
//!
//! The original oxc implementation (~800 lines) includes:
//! - Type node serialization (void, function, array, boolean, string, number,
//!   bigint, symbol, etc.)
//! - Type reference resolution with fallback for unavailable types
//! - Union/intersection type handling
//! - Enum type inference (string, number, mixed)
//! - Entity name serialization with runtime checks
//! - Metadata stacks for methods and constructors
//!
//! ## References
//! * TypeScript's [emitDecoratorMetadata](https://www.typescriptlang.org/tsconfig#emitDecoratorMetadata)

use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

/// Type of an enum inferred from its members
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnumType {
    /// All members are string literals or template literals with string-only
    /// expressions
    String,
    /// All members are numeric, bigint, unary numeric, or auto-incremented
    Number,
    /// Mixed types or computed values
    Object,
}

/// Metadata for decorated methods
#[allow(dead_code)]
pub(super) struct MethodMetadata {
    /// The `design:type` metadata expression
    pub r#type: Box<Expr>,
    /// The `design:paramtypes` metadata expression
    pub param_types: Box<Expr>,
    /// The `design:returntype` metadata expression (optional, omitted for
    /// getters/setters)
    pub return_type: Option<Box<Expr>>,
}

/// Legacy decorator metadata transformer
///
/// Emits decorator metadata for TypeScript decorators
pub struct LegacyDecoratorMetadata {
    // TODO: Add fields for tracking metadata state
    // - method_metadata_stack: Stack for method metadata
    // - constructor_metadata_stack: Stack for constructor metadata
    // - enum_types: Map of enum symbol IDs to their inferred types
}

impl LegacyDecoratorMetadata {
    /// Create a new metadata transformer
    pub fn new() -> Self {
        Self {
            // TODO: Initialize metadata tracking structures
        }
    }

    /// Infer the type of an enum based on its members
    #[allow(dead_code)]
    fn infer_enum_type(_members: &[TsEnumMember]) -> EnumType {
        // TODO: Implement enum type inference
        // Analyze members to determine if enum is String, Number, or Object type
        EnumType::Object
    }

    /// Serialize a TypeScript type node for use with decorator metadata
    ///
    /// Types are serialized as follows:
    /// - Void types -> "undefined" (e.g. "void 0")
    /// - Function and Constructor types -> global "Function"
    /// - Array and Tuple types -> global "Array"
    /// - Boolean types and type predicates -> global "Boolean"
    /// - String literal types and strings -> global "String"
    /// - Enum and number types -> global "Number"
    /// - Symbol types -> global "Symbol"
    /// - Type references to classes -> constructor for the class
    /// - Everything else -> global "Object"
    #[allow(dead_code)]
    fn serialize_type_node(&mut self, _node: &TsType) -> Box<Expr> {
        // TODO: Implement type serialization logic
        // This is the core of metadata emission
        Box::new(Expr::Ident(Ident::new_no_ctxt(
            "Object".into(),
            Default::default(),
        )))
    }

    /// Create a metadata decorator call expression
    #[allow(dead_code)]
    fn create_metadata(&self, _key: &str, _value: Box<Expr>) -> Decorator {
        // TODO: Generate `_metadata(key, value)` helper call
        Decorator {
            span: Default::default(),
            expr: Box::new(Expr::Ident(Ident::new_no_ctxt(
                "TODO".into(),
                Default::default(),
            ))),
        }
    }
}

impl VisitMut for LegacyDecoratorMetadata {
    fn visit_mut_module(&mut self, n: &mut Module) {
        // TODO: Finalize metadata emission
        n.visit_mut_children_with(self);
    }

    fn visit_mut_script(&mut self, n: &mut Script) {
        // TODO: Finalize metadata emission
        n.visit_mut_children_with(self);
    }

    fn visit_mut_ts_enum_decl(&mut self, n: &mut TsEnumDecl) {
        // TODO: Collect enum type information for metadata generation
        n.visit_mut_children_with(self);
    }

    fn visit_mut_class(&mut self, n: &mut Class) {
        // TODO: Handle constructor metadata
        n.visit_mut_children_with(self);
    }

    fn visit_mut_class_method(&mut self, n: &mut ClassMethod) {
        // TODO: Generate method metadata (design:type, design:paramtypes,
        // design:returntype)
        n.visit_mut_children_with(self);
    }

    fn visit_mut_class_prop(&mut self, n: &mut ClassProp) {
        // TODO: Generate property metadata (design:type)
        if !n.decorators.is_empty() {
            // Should add design:type metadata decorator
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_private_prop(&mut self, n: &mut PrivateProp) {
        // TODO: Generate private property metadata
        n.visit_mut_children_with(self);
    }
}
