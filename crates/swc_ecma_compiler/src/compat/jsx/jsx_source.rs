//! React JSX Source
//!
//! This plugin adds `__source` attribute to JSX elements.
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
//! var _jsxFileName = "<CWD>/test.js";
//! <div __source={
//!     { fileName: _jsxFileName, lineNumber: 1, columnNumber: 1 }
//! }>foo</div>;
//! <Bar __source={
//!     { fileName: _jsxFileName, lineNumber: 2, columnNumber: 1 }
//! }>foo</Bar>;
//! <>foo</>;
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-react-jsx-source](https://babeljs.io/docs/babel-plugin-transform-react-jsx-source).
//!
//! ## References:
//!
//! * Babel plugin implementation: <https://github.com/babel/babel/blob/v7.26.2/packages/babel-plugin-transform-react-jsx-source/src/index.ts>

use std::cell::RefCell;

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

use crate::compat::context::TransformCtx;

const SOURCE: &str = "__source";
const FILE_NAME_VAR: &str = "jsxFileName";

pub struct JsxSource<'ctx> {
    /// Variable name for the filename declaration
    /// This is lazily initialized when first JSX element with __source is
    /// encountered
    filename_var: RefCell<Option<Atom>>,
    ctx: &'ctx TransformCtx,
}

impl<'ctx> JsxSource<'ctx> {
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self {
            filename_var: RefCell::new(None),
            ctx,
        }
    }

    /// Report duplicate __source prop error
    pub fn report_error(&self, _span: swc_common::Span) {
        self.ctx.error("Duplicate __source prop found.".to_string());
    }

    /// Get line and column from span
    /// Line number starts at 1, column number starts at 1
    /// This matches Babel's output
    pub fn get_line_column(&self, span: swc_common::Span) -> (u32, u32) {
        // For SWC, we can use the span's location directly
        // In a real implementation, we would need access to SourceMap
        // For now, we'll use dummy values
        // TODO: Get actual line/column from SourceMap
        let line = 1u32;
        let column = 1u32;
        (line, column)
    }

    /// Create the __source object expression
    /// `{ fileName: _jsxFileName, lineNumber: 1, columnNumber: 1 }`
    pub fn get_source_object(&mut self, line: u32, column: u32) -> Expr {
        let filename_var = self.get_filename_var();

        let props = vec![
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName::new("fileName".into(), DUMMY_SP)),
                value: Box::new(Expr::Ident(Ident::new(
                    filename_var.clone(),
                    DUMMY_SP,
                    Default::default(),
                ))),
            }))),
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName::new("lineNumber".into(), DUMMY_SP)),
                value: Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: line as f64,
                    raw: None,
                }))),
            }))),
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName::new("columnNumber".into(), DUMMY_SP)),
                value: Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: column as f64,
                    raw: None,
                }))),
            }))),
        ];

        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
        })
    }

    /// Create an object property for __source attribute (used in classic JSX
    /// mode)
    pub fn get_object_property_kind_for_jsx_plugin(
        &mut self,
        line: u32,
        column: u32,
    ) -> PropOrSpread {
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(IdentName::new(SOURCE.into(), DUMMY_SP)),
            value: Box::new(self.get_source_object(line, column)),
        })))
    }

    /// Add __source attribute to JSX opening element
    pub fn add_source_attribute(&mut self, elem: &mut JSXOpeningElement) {
        // Don't add __source if this node was generated (unspanned)
        if elem.span.is_dummy() {
            return;
        }

        // Check if `__source` attribute already exists
        for item in &elem.attrs {
            if let JSXAttrOrSpread::JSXAttr(attr) = item {
                if let JSXAttrName::Ident(ident) = &attr.name {
                    if &*ident.sym == SOURCE {
                        self.report_error(ident.span);
                        return;
                    }
                }
            }
        }

        let (line, column) = self.get_line_column(elem.span);
        let object = self.get_source_object(line, column);

        // Add the __source attribute
        let name = JSXAttrName::Ident(IdentName::new(SOURCE.into(), DUMMY_SP));
        let value = Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(Box::new(object)),
        }));

        elem.attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: DUMMY_SP,
            name,
            value,
        }));
    }

    /// Get the variable declarator for the filename variable
    /// `var _jsxFileName = "<CWD>/test.js";`
    pub fn get_filename_var_declarator(&self) -> Option<VarDeclarator> {
        let filename_var = self.filename_var.borrow();
        let var_name = filename_var.as_ref()?;

        let id = Pat::Ident(BindingIdent {
            id: Ident::new(var_name.clone(), DUMMY_SP, Default::default()),
            type_ann: None,
        });

        let source_path = self.ctx.source_path.to_string_lossy().to_string();
        let init = Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: source_path.into(),
            raw: None,
        }));

        Some(VarDeclarator {
            span: DUMMY_SP,
            name: id,
            init: Some(Box::new(init)),
            definite: false,
        })
    }

    /// Get or create the filename variable name
    fn get_filename_var(&self) -> Atom {
        let mut filename_var = self.filename_var.borrow_mut();
        if filename_var.is_none() {
            // Generate a unique variable name
            // TODO: Use proper UID generation
            *filename_var = Some(format!("_{}", FILE_NAME_VAR).into());
        }
        filename_var.as_ref().unwrap().clone()
    }
}
