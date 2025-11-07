//! React JSX Self
//!
//! This plugin adds `__self` attribute to JSX elements.
//!
//! > This plugin is included in `preset-react`.
//!
//! ## Example
//!
//! Input:
//! ```js
//! <div>foo</div>;
//! <Bar>foo</Bar>;
//! <>foo</>;
//! ```
//!
//! Output:
//! ```js
//! <div __self={this}>foo</div>;
//! <Bar __self={this}>foo</Bar>;
//! <>foo</>;
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-react-jsx-self](https://babeljs.io/docs/babel-plugin-transform-react-jsx-self).
//!
//! ## References:
//!
//! * Babel plugin implementation: <https://github.com/babel/babel/blob/v7.26.2/packages/babel-plugin-transform-react-jsx-self/src/index.ts>

use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

use crate::compat::context::TransformCtx;

const SELF: &str = "__self";

pub struct JsxSelf<'ctx> {
    ctx: &'ctx TransformCtx,
}

impl<'ctx> JsxSelf<'ctx> {
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self { ctx }
    }

    /// Report duplicate __self prop error
    pub fn report_error(&self, _span: swc_common::Span) {
        self.ctx.error("Duplicate __self prop found.".to_string());
    }

    /// Create an object property for __self attribute (used in classic JSX
    /// mode)
    pub fn get_object_property_kind_for_jsx_plugin() -> PropOrSpread {
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(IdentName::new(SELF.into(), DUMMY_SP)),
            value: Box::new(Expr::This(ThisExpr { span: DUMMY_SP })),
        })))
    }

    /// Check if we can add __self attribute (used in oxc implementation)
    /// For now, we always return true since we don't have access to traversal
    /// context TODO: Port the actual logic from oxc that checks
    /// constructors and super classes
    pub fn can_add_self_attribute() -> bool {
        true
    }

    /// Add __self attribute to JSX opening element
    pub fn add_self_this_attribute(&self, elem: &mut JSXOpeningElement) {
        // Check if `__self` attribute already exists
        for item in &elem.attrs {
            if let JSXAttrOrSpread::JSXAttr(attr) = item {
                if let JSXAttrName::Ident(ident) = &attr.name {
                    if &*ident.sym == SELF {
                        self.report_error(ident.span);
                        return;
                    }
                }
            }
        }

        // Add the __self attribute
        let name = JSXAttrName::Ident(IdentName::new(SELF.into(), DUMMY_SP));
        let value = Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(Box::new(Expr::This(ThisExpr { span: DUMMY_SP }))),
        }));

        elem.attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: DUMMY_SP,
            name,
            value,
        }));
    }
}
