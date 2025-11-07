//! ES2019: Optional Catch Binding
//!
//! This plugin transforms catch clause without parameter to add a parameter
//! called `unused` in catch clause.
//!
//! > This plugin is included in `preset-env`, in ES2019
//!
//! ## Example
//!
//! Input:
//! ```js
//! try {
//!   throw 0;
//! } catch {
//!   doSomethingWhichDoesNotCareAboutTheValueThrown();
//! }
//! ```
//!
//! Output:
//! ```js
//! try {
//!   throw 0;
//! } catch (_unused) {
//!   doSomethingWhichDoesNotCareAboutTheValueThrown();
//! }
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-optional-catch-binding](https://babel.dev/docs/babel-plugin-transform-optional-catch-binding).
//!
//! ## References:
//! * Babel plugin implementation: <https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-transform-optional-catch-binding>
//! * Optional catch binding TC39 proposal: <https://github.com/tc39/proposal-optional-catch-binding>

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

pub struct OptionalCatchBinding<'ctx> {
    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,
}

impl<'ctx> OptionalCatchBinding<'ctx> {
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self { ctx }
    }
}

impl VisitMutHook for OptionalCatchBinding<'_> {
    /// If CatchClause has no param, add a parameter called `_unused`.
    fn enter_catch_clause(&mut self, clause: &mut CatchClause) {
        if clause.param.is_some() {
            return;
        }

        // Create a simple identifier binding for the unused parameter
        // Using `_unused` to follow the naming convention and avoid conflicts
        let ident = Ident::new(Atom::from("_unused"), DUMMY_SP, Default::default());

        let binding = BindingIdent {
            id: ident,
            type_ann: None,
        };

        clause.param = Some(Pat::Ident(binding));
    }
}

#[cfg(test)]
mod tests {
    use swc_common::DUMMY_SP;
    use swc_ecma_ast::*;

    use super::*;

    /// Helper to create a test TransformCtx
    fn create_test_ctx() -> TransformCtx {
        use std::path::Path;

        use crate::Config;

        TransformCtx::new(Path::new("test.js"), &Config::default())
    }

    #[test]
    fn test_optional_catch_binding_adds_param() {
        let ctx = create_test_ctx();
        let mut transform = OptionalCatchBinding::new(&ctx);

        // Create a catch clause without param
        let mut clause = CatchClause {
            span: DUMMY_SP,
            param: None,
            body: BlockStmt {
                span: DUMMY_SP,
                stmts: vec![],
                ..Default::default()
            },
        };

        // Transform should add the _unused param
        transform.enter_catch_clause(&mut clause);

        // Verify param was added
        assert!(clause.param.is_some());

        // Verify it's an identifier binding with name "_unused"
        if let Some(Pat::Ident(binding)) = &clause.param {
            assert_eq!(binding.id.sym.as_ref(), "_unused");
        } else {
            panic!("Expected Pat::Ident binding");
        }
    }

    #[test]
    fn test_optional_catch_binding_preserves_existing_param() {
        let ctx = create_test_ctx();
        let mut transform = OptionalCatchBinding::new(&ctx);

        // Create a catch clause with an existing param
        let original_ident = Ident::new(Atom::from("err"), DUMMY_SP, Default::default());
        let original_binding = BindingIdent {
            id: original_ident.clone(),
            type_ann: None,
        };

        let mut clause = CatchClause {
            span: DUMMY_SP,
            param: Some(Pat::Ident(original_binding)),
            body: BlockStmt {
                span: DUMMY_SP,
                stmts: vec![],
                ..Default::default()
            },
        };

        // Transform should not modify existing param
        transform.enter_catch_clause(&mut clause);

        // Verify param was preserved
        if let Some(Pat::Ident(binding)) = &clause.param {
            assert_eq!(binding.id.sym.as_ref(), "err");
        } else {
            panic!("Expected Pat::Ident binding");
        }
    }
}
