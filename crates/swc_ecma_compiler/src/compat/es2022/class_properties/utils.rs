//! ES2022: Class Properties
//! Utility functions.

use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;

/// Create `var` declaration.
#[allow(dead_code)]
pub(super) fn create_variable_declaration(ident: &Ident, init: Box<Expr>) -> Stmt {
    let decl = VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        declare: false,
        kind: VarDeclKind::Var,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(ident.clone().into()),
            init: Some(init),
            definite: false,
        }],
    };
    Stmt::Decl(Decl::Var(Box::new(decl)))
}

/// Convert an iterator of `Expr`s into an iterator of `Stmt::Expr`s.
#[allow(dead_code)]
pub(super) fn exprs_into_stmts<E>(exprs: E) -> impl Iterator<Item = Stmt>
where
    E: IntoIterator<Item = Box<Expr>>,
{
    exprs.into_iter().map(|expr| {
        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr,
        })
    })
}

/// Create `IdentName` for `_`.
#[allow(dead_code)]
pub(super) fn create_underscore_ident_name() -> IdentName {
    IdentName::new("_".into(), DUMMY_SP)
}
