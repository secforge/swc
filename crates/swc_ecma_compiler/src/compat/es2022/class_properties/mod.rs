//! ES2022: Class Properties
//!
//! This plugin transforms class properties to initializers inside class
//! constructor.
//!
//! > This plugin is included in `preset-env`, in ES2022
//!
//! ## Example
//!
//! Input:
//! ```js
//! class C {
//!   foo = 123;
//!   #bar = 456;
//!   method() {
//!     let bar = this.#bar;
//!     this.#bar = bar + 1;
//!   }
//! }
//! ```
//!
//! Output:
//! ```js
//! var _bar = /*#__PURE__*/ new WeakMap();
//! class C {
//!   constructor() {
//!     babelHelpers.defineProperty(this, "foo", 123);
//!     babelHelpers.classPrivateFieldInitSpec(this, _bar, 456);
//!   }
//!   method() {
//!     let bar = babelHelpers.classPrivateFieldGet2(_bar, this);
//!     babelHelpers.classPrivateFieldSet2(_bar, this, bar + 1);
//!   }
//! }
//! ```
//!
//! ## Implementation Status
//!
//! This is a PARTIAL PORT from the oxc implementation. The complete port is in
//! progress. See PORTING_PLAN.md in this directory for details.
//!
//! ## References
//!
//! * Babel plugin implementation:
//!   * <https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-transform-class-properties>
//! * Class properties TC39 proposal: <https://github.com/tc39/proposal-class-fields>

use serde::Deserialize;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

mod utils;
// TODO: Uncomment as files are ported
// mod class_bindings;
// mod class_details;
// mod computed_key;
// mod prop_decl;
// mod instance_prop_init;
// mod constructor;
// mod static_block_and_prop_init;
// mod super_converter;
// mod private_method;
// mod private_field;
// mod class;

#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClassPropertiesOptions {
    pub loose: bool,
}

/// Class properties transform.
///
/// See [module docs] for details.
///
/// [module docs]: self
pub struct ClassProperties<'ctx> {
    // Options
    /// If `true`, set properties with `=`, instead of `_defineProperty` helper
    /// (loose option).
    #[allow(dead_code)]
    set_public_class_fields: bool,
    /// If `true`, store private properties as normal properties as string keys
    /// (loose option).
    #[allow(dead_code)]
    private_fields_as_properties: bool,
    /// If `true`, transform static blocks.
    #[allow(dead_code)]
    transform_static_blocks: bool,
    /// If `true`, remove class fields without initializer. Only works with
    /// `set_public_class_fields: true`.
    #[allow(dead_code)]
    remove_class_fields_without_initializer: bool,

    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,
    // TODO: Add state fields as they are ported:
    // - classes_stack: ClassesStack
    // - private_field_count: usize
    // - clashing_constructor_symbols: FxHashMap<SymbolId, Atom>
    // - insert_before: Vec<Box<Expr>>
    // - insert_after_exprs: Vec<Box<Expr>>
    // - insert_after_stmts: Vec<Stmt>
}

impl<'ctx> ClassProperties<'ctx> {
    /// Create `ClassProperties` transformer
    #[allow(dead_code)]
    pub fn new(
        options: ClassPropertiesOptions,
        transform_static_blocks: bool,
        remove_class_fields_without_initializer: bool,
        ctx: &'ctx TransformCtx,
    ) -> Self {
        let set_public_class_fields = options.loose || ctx.assumptions.set_public_class_fields;
        let private_fields_as_properties =
            options.loose || ctx.assumptions.private_fields_as_properties;

        Self {
            set_public_class_fields,
            private_fields_as_properties,
            transform_static_blocks,
            remove_class_fields_without_initializer,
            ctx,
        }
    }
}

impl VisitMutHook for ClassProperties<'_> {
    // TODO: Implement visitor methods as transformation logic is ported
    // fn enter_class(&mut self, class: &mut Class) { }
    // fn exit_class(&mut self, class: &mut Class) { }
    // fn enter_expr(&mut self, expr: &mut Expr) { }
    // etc.
}
