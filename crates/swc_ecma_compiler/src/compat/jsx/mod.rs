//! JSX transformation module for SWC
//!
//! This module provides JSX transformation functionality compatible with React,
//! including support for both classic and automatic runtimes.
//!
//! ## Modules
//!
//! - `comments`: Handle JSX pragma comments (@jsx, @jsxRuntime, etc.)
//! - `diagnostics`: Error messages for JSX transformations
//! - `display_name`: Add displayName to React.createClass
//! - `jsx_impl`: Core JSX transformation logic
//! - `jsx_self`: Add __self attribute in development mode
//! - `jsx_source`: Add __source attribute in development mode
//! - `options`: Configuration options for JSX transformations
//! - `refresh`: React Fast Refresh support
//!
//! ## Features
//!
//! This is a port of the oxc JSX transformation to work with SWC's AST and
//! visitor pattern. The original oxc implementation is located at:
//! `/Users/kdy1/projects/chores/crates/swc_ecma_compiler/src/oxc/jsx/`
//!
//! ### Included Plugins
//!
//! * [plugin-transform-react-jsx](https://babeljs.io/docs/babel-plugin-transform-react-jsx)
//! * [plugin-transform-react-jsx-self](https://babeljs.io/docs/babel-plugin-transform-react-jsx-self)
//! * [plugin-transform-react-jsx-source](https://babeljs.io/docs/babel-plugin-transform-react-jsx-source)
//! * [plugin-transform-react-display-name](https://babeljs.io/docs/babel-plugin-transform-react-display-name)
//! * [plugin-transform-react-refresh](https://github.com/facebook/react/tree/main/packages/react-refresh)

mod comments;
mod diagnostics;
mod display_name;
mod jsx_impl;
mod jsx_self;
mod jsx_source;
mod options;
mod refresh;

#[allow(unused_imports)]
pub use comments::update_options_with_comments;
use display_name::ReactDisplayName;
use jsx_impl::JsxImpl;
pub use options::JsxOptions;
#[allow(unused_imports)]
pub use options::JsxRuntime;
use refresh::ReactRefresh;

use crate::compat::context::TransformCtx;

/// [Preset React](https://babel.dev/docs/babel-preset-react)
///
/// This preset includes the following plugins:
///
/// * [plugin-transform-react-jsx](https://babeljs.io/docs/babel-plugin-transform-react-jsx)
/// * [plugin-transform-react-jsx-self](https://babeljs.io/docs/babel-plugin-transform-react-jsx-self)
/// * [plugin-transform-react-jsx-source](https://babel.dev/docs/babel-plugin-transform-react-jsx-source)
/// * [plugin-transform-react-display-name](https://babeljs.io/docs/babel-plugin-transform-react-display-name)
pub struct Jsx {
    implementation: JsxImpl,
    display_name: ReactDisplayName,
    refresh: ReactRefresh,
    enable_jsx_plugin: bool,
    display_name_plugin: bool,
    self_plugin: bool,
    source_plugin: bool,
    refresh_plugin: bool,
}

impl Jsx {
    /// Create a new JSX transformation instance
    ///
    /// # Arguments
    ///
    /// * `options` - JSX configuration options
    /// * `ctx` - Transform context
    pub fn new(mut options: JsxOptions, ctx: &TransformCtx) -> Self {
        if options.jsx_plugin || options.development {
            options.conform();
        }
        let JsxOptions {
            jsx_plugin,
            display_name_plugin,
            jsx_self_plugin,
            jsx_source_plugin,
            ..
        } = options;
        let refresh = options.refresh.clone();
        Self {
            implementation: JsxImpl::new(options, ctx),
            display_name: ReactDisplayName::new(ctx),
            enable_jsx_plugin: jsx_plugin,
            display_name_plugin,
            self_plugin: jsx_self_plugin,
            source_plugin: jsx_source_plugin,
            refresh_plugin: refresh.is_some(),
            refresh: ReactRefresh::new(&refresh.unwrap_or_default(), ctx),
        }
    }

    // TODO: Implement visitor methods for SWC's VisitMut trait
    // The full implementation will require:
    // - impl VisitMut for Jsx
    // - visit_program, visit_expr, visit_jsx_element, etc.
    // - Proper AST node transformations using SWC's AST types
}
