//! React Display Name
//!
//! Adds `displayName` property to `React.createClass` calls.
//!
//! > This plugin is included in `preset-react`.
//!
//! ## Example
//!
//! Input:
//! ```js
//! // some_filename.jsx
//! var foo = React.createClass({}); // React <= 15
//! bar = createReactClass({}); // React 16+
//!
//! var obj = { prop: React.createClass({}) };
//! obj.prop2 = React.createClass({});
//! obj["prop 3"] = React.createClass({});
//! export default React.createClass({});
//! ```
//!
//! Output:
//! ```js
//! var foo = React.createClass({ displayName: "foo" });
//! bar = createReactClass({ displayName: "bar" });
//!
//! var obj = { prop: React.createClass({ displayName: "prop" }) };
//! obj.prop2 = React.createClass({ displayName: "prop2" });
//! obj["prop 3"] = React.createClass({ displayName: "prop 3" });
//! export default React.createClass({ displayName: "some_filename" });
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-react-display-name](https://babeljs.io/docs/babel-plugin-transform-react-display-name).
//!
//! Babel does not get the display name for this example:
//!
//! ```js
//! obj["prop 3"] = React.createClass({});
//! ```
//!
//! This implementation does, which is a divergence from Babel, but probably an
//! improvement.
//!
//! ## References:
//!
//! * Babel plugin implementation: <https://github.com/babel/babel/blob/v7.26.2/packages/babel-plugin-transform-react-display-name/src/index.ts>

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

const DISPLAY_NAME: &str = "displayName";

pub struct ReactDisplayName<'ctx> {
    ctx: &'ctx TransformCtx,
}

impl<'ctx> ReactDisplayName<'ctx> {
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self { ctx }
    }

    /// Check if a call expression is React.createClass or createReactClass
    fn is_create_class_call(callee: &Callee) -> bool {
        match callee {
            Callee::Expr(expr) => match &**expr {
                Expr::Member(member) => {
                    // React.createClass
                    if let MemberProp::Ident(prop) = &member.prop {
                        if &*prop.sym == "createClass" {
                            if let Expr::Ident(obj) = &*member.obj {
                                return &*obj.sym == "React";
                            }
                        }
                    }
                    false
                }
                Expr::Ident(ident) => {
                    // createReactClass
                    &*ident.sym == "createReactClass"
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// Get the object expression from a createClass call
    fn get_object_from_create_class<'a>(call: &'a mut CallExpr) -> Option<&'a mut ObjectLit> {
        if !Self::is_create_class_call(&call.callee) {
            return None;
        }

        // Only 1 argument being the object expression
        if call.args.len() != 1 {
            return None;
        }

        match call.args[0].expr.as_mut() {
            Expr::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Add displayName property to an object expression
    fn add_display_name(obj: &mut ObjectLit, name: Atom) {
        // Check if displayName already exists
        for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = prop.as_ref() {
                    if let PropName::Ident(ident) = &kv.key {
                        if &*ident.sym == DISPLAY_NAME {
                            // displayName already exists, don't add it
                            return;
                        }
                    }
                }
            }
        }

        // Add displayName at the beginning of the object
        let display_name_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(IdentName::new(DISPLAY_NAME.into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: name.into(),
                raw: None,
            }))),
        })));

        obj.props.insert(0, display_name_prop);
    }

    /// Handle a call expression - check if it's createClass and add displayName
    pub fn handle_call_expr(&self, call: &mut CallExpr, name: Option<Atom>) {
        if let Some(name) = name {
            if let Some(obj) = Self::get_object_from_create_class(call) {
                Self::add_display_name(obj, name);
            }
        }
    }
}

impl VisitMutHook for ReactDisplayName<'_> {
    // The actual implementation would need to track parent nodes to determine
    // the display name. For now, this is a placeholder structure.
    // A full implementation would require tracking the AST ancestry similar to
    // oxc's implementation.
}
