//! ES2018 object spread transformation.
//!
//! This plugin transforms rest properties for object destructuring assignment
//! and spread properties for object literals.
//!
//! > This plugin is included in `preset-env`, in ES2018
//!
//! ## Example
//!
//! Input:
//! ```js
//! var x = { a: 1, b: 2 };
//! var y = { ...x, c: 3 };
//! ```
//!
//! Output:
//! ```js
//! var x = { a: 1, b: 2 };
//! var y = _objectSpread({}, x, { c: 3 });
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-object-rest-spread](https://babeljs.io/docs/babel-plugin-transform-object-rest-spread).
//!
//! ## References:
//!
//! * Babel plugin implementation: <https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-transform-object-rest-spread>
//! * Object rest/spread TC39 proposal: <https://github.com/tc39/proposal-object-rest-spread>
//!
//! ## Note
//!
//! This is a complex transformation that requires extensive AST manipulation.
//! The oxc implementation uses arena allocation and the Traverse trait
//! extensively. A full port to SWC's VisitMutHook pattern would require:
//!
//! 1. Converting arena-allocated types to owned types
//! 2. Reimplementing scope management without oxc's ScopeId
//! 3. Adapting the statement injection mechanism
//! 4. Handling variable declarations differently
//! 5. Managing temporary variable creation
//!
//! For production use, consider using SWC's existing object rest/spread
//! transform at `swc_ecma_transforms_compat::es2018::object_rest_spread`.

#![allow(dead_code)]
use serde::Deserialize;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::{common::helper_loader::Helper, context::TransformCtx};

/// Options for object rest/spread transformation
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ObjectRestSpreadOptions {
    /// Use assignment instead of defineProperty for object spread
    pub loose: bool,

    /// Use Object.assign directly instead of Babel's helper
    pub use_built_ins: bool,
}

/// Object rest/spread transformer for ES2018.
///
/// Transforms object rest/spread syntax to ES5-compatible code.
///
/// # Implementation Status
///
/// This implementation currently handles:
/// - Basic object spread in object literals: `{ ...obj, key: value }`
///
/// Not yet implemented (simplified from oxc):
/// - Object rest in destructuring patterns
/// - Complex nested patterns
/// - Function parameter rest/spread
/// - Assignment expression rest/spread
/// - Variable declaration rest patterns
///
/// For production use with full feature support, consider using SWC's built-in
/// object rest/spread transform in `swc_ecma_transforms_compat`.
pub struct ObjectRestSpread<'ctx> {
    #[allow(dead_code)]
    options: ObjectRestSpreadOptions,
    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,
}

impl<'ctx> ObjectRestSpread<'ctx> {
    /// Create a new object rest/spread transformer.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the transformation
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ObjectRestSpreadOptions, ctx: &'ctx TransformCtx) -> Self {
        if options.loose {
            ctx.error("Option `loose` is not implemented for object-rest-spread.".to_string());
        }
        if options.use_built_ins {
            ctx.error(
                "Option `useBuiltIns` is not implemented for object-rest-spread.".to_string(),
            );
        }
        Self { options, ctx }
    }

    /// Transform object expressions containing spread properties.
    ///
    /// Transforms `{ ...x, a: 1 }` to `objectSpread2({}, x, { a: 1 })`.
    fn transform_object_expression(&self, obj: &mut ObjectLit) -> Option<Expr> {
        // Check if there are any spread properties
        if !obj
            .props
            .iter()
            .any(|prop| matches!(prop, PropOrSpread::Spread(_)))
        {
            return None;
        }

        let mut arguments = vec![];
        let mut current_props = vec![];

        for prop in obj.props.drain(..) {
            match prop {
                PropOrSpread::Spread(spread) => {
                    // Flush current props as an object literal
                    if !current_props.is_empty() {
                        arguments.push(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Object(ObjectLit {
                                span: DUMMY_SP,
                                props: std::mem::take(&mut current_props),
                            })),
                        });
                    }
                    // Add the spread expression
                    arguments.push(ExprOrSpread {
                        spread: None,
                        expr: spread.expr,
                    });
                }
                prop => {
                    current_props.push(prop);
                }
            }
        }

        // Flush remaining props
        if !current_props.is_empty() {
            arguments.push(ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: current_props,
                })),
            });
        }

        // If first argument is not an empty object, prepend one
        let first_is_empty_object = arguments.first().is_some_and(|arg| {
            matches!(
                &*arg.expr,
                Expr::Object(ObjectLit { props, .. }) if props.is_empty()
            )
        });

        if !first_is_empty_object {
            arguments.insert(
                0,
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![],
                    })),
                },
            );
        }

        // Create helper call: babelHelpers.objectSpread2({}, x, { a: 1 })
        Some(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(Ident::new(
                    "babelHelpers".into(),
                    DUMMY_SP,
                    Default::default(),
                ))),
                prop: MemberProp::Ident(IdentName::new(
                    Helper::ObjectSpread2.name().into(),
                    DUMMY_SP,
                )),
            }))),
            args: arguments,
            type_args: None,
        }))
    }
}

impl VisitMutHook for ObjectRestSpread<'_> {
    fn exit_expr(&mut self, expr: &mut Expr) {
        if let Expr::Object(obj) = expr {
            if let Some(new_expr) = self.transform_object_expression(obj) {
                *expr = new_expr;
            }
        }
    }

    // TODO: Implement additional transformations:
    //
    // - exit_pat: Transform patterns with rest in destructuring
    // - exit_param: Transform function parameters with rest
    // - exit_stmt: Handle variable declarations with rest patterns
    //
    // These would require:
    // 1. Detecting rest patterns in various contexts
    // 2. Creating temporary variables as needed
    // 3. Generating helper calls for objectWithoutProperties
    // 4. Managing excluded keys for rest patterns
    //
    // The full oxc implementation is over 1100 lines handling all these cases.
    // For now, this simplified version handles the most common use case:
    // object spread in object literals.
}
