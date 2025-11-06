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

use crate::compat::context::TransformCtx;

const SOURCE: &str = "__source";
const FILE_NAME_VAR: &str = "jsxFileName";

pub struct JsxSource {
    // filename_var: Option<BoundIdentifier<'a>>,
    // source_rope: Option<Rope>,
    // ctx: &'ctx TransformCtx<'a>,
}

impl JsxSource {
    pub fn new(_ctx: &TransformCtx) -> Self {
        Self {}
    }

    pub fn report_error(&self, _error_msg: String) {
        // TODO: Implement error reporting
    }

    // TODO: Port remaining methods from oxc version
    // The full implementation requires SWC's visitor pattern
}
