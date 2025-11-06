//! Utility to load helper functions.
//!
//! This module provides functionality to load helper functions in different
//! modes. It supports runtime, external, and inline (not yet implemented) modes
//! for loading helper functions.
//!
//! ## Usage
//!
//! You can call helper loading functions to load a helper function and use it
//! in your CallExpr.
//!
//! ```rs
//! let callee = helper_load(helper, &ctx);
//! let call = CallExpr { callee: Callee::Expr(callee), ... };
//! ```
//!
//! ## Modes
//!
//! ### Runtime ([`HelperLoaderMode::Runtime`])
//!
//! Uses `@swc/helpers` as a dependency, importing helper functions from the
//! runtime.
//!
//! Generated code example:
//!
//! ```js
//! import helperName from "@swc/helpers/helperName";
//! helperName(...arguments);
//! ```
//!
//! Based on [@babel/plugin-transform-runtime](https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-transform-runtime).
//!
//! ### External ([`HelperLoaderMode::External`])
//!
//! Uses helper functions from a global `babelHelpers` variable. This is the
//! default mode for testing.
//!
//! Generated code example:
//!
//! ```js
//! babelHelpers.helperName(...arguments);
//! ```
//!
//! Based on [@babel/plugin-external-helpers](https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-external-helpers).
//!
//! ### Inline ([`HelperLoaderMode::Inline`])
//!
//! > Note: This mode is not currently implemented.
//!
//! Inline helper functions are inserted directly into the top of program.
//!
//! Generated code example:
//!
//! ```js
//! function helperName(...arguments) { ... } // Inlined helper function
//! helperName(...arguments);
//! ```
//!
//! Based on [@babel/helper](https://github.com/babel/babel/tree/v7.26.2/packages/babel-helpers).
//!
//! ## Implementation
//!
//! Unlike oxc's version which integrates with `ModuleImports` transform, this
//! SWC version directly generates import statements or member expressions as
//! needed.

use std::borrow::Cow;

use rustc_hash::FxHashMap;
use swc_atoms::{Atom, Wtf8Atom};
use swc_common::{Span, SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;

/// Defines the mode for loading helper functions.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelperLoaderMode {
    /// Inline mode: Helper functions are directly inserted into the program.
    ///
    /// Note: This mode is not currently implemented.
    ///
    /// Example output:
    /// ```js
    /// function helperName(...arguments) { ... } // Inlined helper function
    /// helperName(...arguments);
    /// ```
    Inline,
    /// External mode: Helper functions are accessed from a global
    /// `babelHelpers` object.
    ///
    /// This is the default mode used in Babel tests.
    ///
    /// Example output:
    /// ```js
    /// babelHelpers.helperName(...arguments);
    /// ```
    External,
    /// Runtime mode: Helper functions are imported from a runtime package.
    ///
    /// This mode is similar to how @babel/plugin-transform-runtime works.
    /// It's the default mode for this implementation.
    ///
    /// Example output:
    /// ```js
    /// import helperName from "@swc/helpers/helperName";
    /// helperName(...arguments);
    /// ```
    #[default]
    Runtime,
}

/// Helper loader options.
#[derive(Clone, Debug)]
pub struct HelperLoaderOptions {
    /// The module name to import helper functions from.
    /// Default: `@swc/helpers`
    pub module_name: Cow<'static, str>,
    pub mode: HelperLoaderMode,
}

impl Default for HelperLoaderOptions {
    fn default() -> Self {
        Self {
            module_name: default_as_module_name(),
            mode: HelperLoaderMode::default(),
        }
    }
}

fn default_as_module_name() -> Cow<'static, str> {
    Cow::Borrowed("@swc/helpers")
}

/// Available helpers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Helper {
    AwaitAsyncGenerator,
    AsyncGeneratorDelegate,
    AsyncIterator,
    AsyncToGenerator,
    ObjectSpread2,
    WrapAsyncGenerator,
    Extends,
    ObjectDestructuringEmpty,
    ObjectWithoutProperties,
    ToPropertyKey,
    DefineProperty,
    ClassPrivateFieldInitSpec,
    ClassPrivateMethodInitSpec,
    ClassPrivateFieldGet2,
    ClassPrivateFieldSet2,
    AssertClassBrand,
    ToSetter,
    ClassPrivateFieldLooseKey,
    ClassPrivateFieldLooseBase,
    SuperPropGet,
    SuperPropSet,
    ReadOnlyError,
    WriteOnlyError,
    CheckInRHS,
    Decorate,
    DecorateParam,
    DecorateMetadata,
    UsingCtx,
}

impl Helper {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AwaitAsyncGenerator => "awaitAsyncGenerator",
            Self::AsyncGeneratorDelegate => "asyncGeneratorDelegate",
            Self::AsyncIterator => "asyncIterator",
            Self::AsyncToGenerator => "asyncToGenerator",
            Self::ObjectSpread2 => "objectSpread2",
            Self::WrapAsyncGenerator => "wrapAsyncGenerator",
            Self::Extends => "extends",
            Self::ObjectDestructuringEmpty => "objectDestructuringEmpty",
            Self::ObjectWithoutProperties => "objectWithoutProperties",
            Self::ToPropertyKey => "toPropertyKey",
            Self::DefineProperty => "defineProperty",
            Self::ClassPrivateFieldInitSpec => "classPrivateFieldInitSpec",
            Self::ClassPrivateMethodInitSpec => "classPrivateMethodInitSpec",
            Self::ClassPrivateFieldGet2 => "classPrivateFieldGet2",
            Self::ClassPrivateFieldSet2 => "classPrivateFieldSet2",
            Self::AssertClassBrand => "assertClassBrand",
            Self::ToSetter => "toSetter",
            Self::ClassPrivateFieldLooseKey => "classPrivateFieldLooseKey",
            Self::ClassPrivateFieldLooseBase => "classPrivateFieldLooseBase",
            Self::SuperPropGet => "superPropGet",
            Self::SuperPropSet => "superPropSet",
            Self::ReadOnlyError => "readOnlyError",
            Self::WriteOnlyError => "writeOnlyError",
            Self::CheckInRHS => "checkInRHS",
            Self::Decorate => "decorate",
            Self::DecorateParam => "decorateParam",
            Self::DecorateMetadata => "decorateMetadata",
            Self::UsingCtx => "usingCtx",
        }
    }

    pub const fn pure(self) -> bool {
        matches!(self, Self::ClassPrivateFieldLooseKey)
    }
}

/// Stores the state of the helper loader.
pub struct HelperLoaderStore {
    module_name: Cow<'static, str>,
    mode: HelperLoaderMode,
    /// Loaded helpers, determines what helpers are loaded and what imports
    /// should be added.
    loaded_helpers: FxHashMap<Helper, Ident>,
    pub(crate) used_helpers: FxHashMap<Helper, String>,
}

impl HelperLoaderStore {
    pub fn new(options: &HelperLoaderOptions) -> Self {
        Self {
            module_name: options.module_name.clone(),
            mode: options.mode,
            loaded_helpers: FxHashMap::default(),
            used_helpers: FxHashMap::default(),
        }
    }

    /// Load and call a helper function and return a `CallExpr`.
    pub fn helper_call(&mut self, helper: Helper, span: Span, args: Vec<ExprOrSpread>) -> CallExpr {
        let callee = self.helper_load(helper, span);
        CallExpr {
            span,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(callee),
            args,
            type_args: None,
        }
    }

    /// Same as [`HelperLoaderStore::helper_call`], but returns a `CallExpr`
    /// wrapped in an `Expr`.
    pub fn helper_call_expr(
        &mut self,
        helper: Helper,
        span: Span,
        args: Vec<ExprOrSpread>,
    ) -> Expr {
        Expr::Call(self.helper_call(helper, span, args))
    }

    /// Load a helper function and return a callee expression.
    pub fn helper_load(&mut self, helper: Helper, span: Span) -> Box<Expr> {
        let source = self.get_runtime_source(helper);
        self.used_helpers
            .entry(helper)
            .or_insert_with(|| source.to_string());

        match self.mode {
            HelperLoaderMode::Runtime => self.transform_for_runtime_helper(helper, span),
            HelperLoaderMode::External => Self::transform_for_external_helper(helper, span),
            HelperLoaderMode::Inline => {
                unreachable!("Inline helpers are not supported yet");
            }
        }
    }

    fn transform_for_runtime_helper(&mut self, helper: Helper, span: Span) -> Box<Expr> {
        let helper_name = helper.name();
        let ident = self
            .loaded_helpers
            .entry(helper)
            .or_insert_with(|| Ident {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                sym: Atom::from(helper_name),
                optional: false,
            })
            .clone();

        Box::new(Expr::Ident(Ident { span, ..ident }))
    }

    fn get_runtime_source(&self, helper: Helper) -> String {
        format!("{}/helpers/{}", self.module_name, helper.name())
    }

    fn transform_for_external_helper(helper: Helper, span: Span) -> Box<Expr> {
        const HELPER_VAR: &str = "babelHelpers";

        let object = Box::new(Expr::Ident(Ident {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            sym: Atom::from(HELPER_VAR),
            optional: false,
        }));

        let property = IdentName {
            span: DUMMY_SP,
            sym: Atom::from(helper.name()),
        };

        Box::new(Expr::Member(MemberExpr {
            span,
            obj: object,
            prop: MemberProp::Ident(property),
        }))
    }

    /// Get all required import declarations for loaded helpers in Runtime mode.
    ///
    /// This should be called after all transformations are complete to generate
    /// the import statements for helpers that were used.
    pub fn get_import_declarations(&self) -> Vec<ModuleDecl> {
        if self.mode != HelperLoaderMode::Runtime {
            return vec![];
        }

        let mut imports = vec![];
        for (helper, ident) in &self.loaded_helpers {
            let source = self.get_runtime_source(*helper);
            imports.push(ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                    span: DUMMY_SP,
                    local: ident.clone(),
                })],
                src: Box::new(Str {
                    span: DUMMY_SP,
                    value: Wtf8Atom::from(source.as_str()),
                    raw: None,
                }),
                type_only: false,
                with: None,
                phase: Default::default(),
            }));
        }
        imports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helper_name() {
        assert_eq!(Helper::AsyncToGenerator.name(), "asyncToGenerator");
        assert_eq!(Helper::Extends.name(), "extends");
        assert_eq!(Helper::ObjectSpread2.name(), "objectSpread2");
    }

    #[test]
    fn test_helper_pure() {
        assert!(Helper::ClassPrivateFieldLooseKey.pure());
        assert!(!Helper::AsyncToGenerator.pure());
    }

    #[test]
    fn test_default_module_name() {
        let options = HelperLoaderOptions::default();
        assert_eq!(options.module_name, "@swc/helpers");
        assert!(matches!(options.mode, HelperLoaderMode::Runtime));
    }

    #[test]
    fn test_runtime_source() {
        let options = HelperLoaderOptions::default();
        let store = HelperLoaderStore::new(&options);
        let source = store.get_runtime_source(Helper::AsyncToGenerator);
        assert_eq!(source, "@swc/helpers/helpers/asyncToGenerator");
    }

    #[test]
    fn test_external_helper() {
        let expr =
            HelperLoaderStore::transform_for_external_helper(Helper::AsyncToGenerator, DUMMY_SP);
        if let Expr::Member(member) = expr.as_ref() {
            if let Expr::Ident(obj_ident) = &*member.obj {
                assert_eq!(obj_ident.sym, "babelHelpers");
            } else {
                panic!("Expected object to be an identifier");
            }
            if let MemberProp::Ident(prop) = &member.prop {
                assert_eq!(prop.sym, "asyncToGenerator");
            } else {
                panic!("Expected property to be an identifier");
            }
        } else {
            panic!("Expected member expression");
        }
    }
}
