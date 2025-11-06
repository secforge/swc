use swc_atoms::Atom;
use swc_common::{Span, SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;

/// `object` -> `object.call`.
pub fn create_member_callee(object: Box<Expr>, property: &'static str) -> Box<Expr> {
    Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: object,
        prop: MemberProp::Ident(IdentName {
            span: DUMMY_SP,
            sym: Atom::from(property),
        }),
    }))
}

/// `object` -> `object.bind(this)`.
pub fn create_bind_call(callee: Box<Expr>, this: Box<Expr>, span: Span) -> Expr {
    let callee = create_member_callee(callee, "bind");
    CallExpr {
        span,
        callee: Callee::Expr(callee),
        args: vec![ExprOrSpread {
            spread: None,
            expr: this,
        }],
        ..Default::default()
    }
    .into()
}

/// `object` -> `object.call(...arguments)`.
pub fn create_call_call(callee: Box<Expr>, this: Box<Expr>, span: Span) -> Expr {
    let callee = create_member_callee(callee, "call");
    CallExpr {
        span,
        callee: Callee::Expr(callee),
        args: vec![ExprOrSpread {
            spread: None,
            expr: this,
        }],
        ..Default::default()
    }
    .into()
}

/// Wrap an `Expr` in an arrow function IIFE (immediately invoked function
/// expression) with a body block.
///
/// `expr` -> `(() => { return expr; })()`
pub fn wrap_expression_in_arrow_function_iife(expr: Box<Expr>, span: Span) -> Expr {
    let stmts = vec![Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(expr),
    })];
    wrap_statements_in_arrow_function_iife(stmts, span)
}

/// Wrap statements in an IIFE (immediately invoked function expression).
///
/// `x; y; z;` -> `(() => { x; y; z; })()`
pub fn wrap_statements_in_arrow_function_iife(stmts: Vec<Stmt>, span: Span) -> Expr {
    let arrow = ArrowExpr {
        span: DUMMY_SP,
        params: vec![],
        body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
            span: DUMMY_SP,
            stmts,
            ..Default::default()
        })),
        is_async: false,
        is_generator: false,
        ..Default::default()
    };

    CallExpr {
        span,
        callee: Callee::Expr(Box::new(Expr::Arrow(arrow))),
        args: vec![],
        ..Default::default()
    }
    .into()
}

/// `object` -> `object.prototype`.
pub fn create_prototype_member(object: Box<Expr>) -> Expr {
    Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: object,
        prop: MemberProp::Ident(IdentName {
            span: DUMMY_SP,
            sym: Atom::from("prototype"),
        }),
    })
}

/// `object` -> `object.a`.
pub fn create_property_access(span: Span, object: Box<Expr>, property: &str) -> Expr {
    Expr::Member(MemberExpr {
        span,
        obj: object,
        prop: MemberProp::Ident(IdentName {
            span: DUMMY_SP,
            sym: Atom::from(property),
        }),
    })
}

/// `this.property`
#[inline]
pub fn create_this_property_access(span: Span, property: Atom) -> MemberExpr {
    MemberExpr {
        span,
        obj: Box::new(Expr::This(ThisExpr { span })),
        prop: MemberProp::Ident(IdentName {
            span: DUMMY_SP,
            sym: property,
        }),
    }
}

/// `this.property`
#[inline]
pub fn create_this_property_assignment(span: Span, property: Atom) -> AssignTarget {
    AssignTarget::Simple(SimpleAssignTarget::Member(create_this_property_access(
        span, property,
    )))
}

/// Create assignment to an identifier.
pub fn create_assignment(binding: &Ident, value: Box<Expr>) -> Expr {
    AssignExpr {
        span: DUMMY_SP,
        op: op!("="),
        left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent {
            id: binding.clone(),
            type_ann: None,
        })),
        right: value,
    }
    .into()
}

/// `super(...args);`
pub fn create_super_call(args_ident: &Ident) -> Expr {
    CallExpr {
        span: DUMMY_SP,
        callee: Callee::Super(Super { span: DUMMY_SP }),
        args: vec![ExprOrSpread {
            spread: Some(DUMMY_SP),
            expr: Box::new(Expr::Ident(args_ident.clone())),
        }],
        ..Default::default()
    }
    .into()
}

/// * With super class: `constructor(..._args) { super(..._args); statements }`
/// * Without super class: `constructor() { statements }`
pub fn create_class_constructor<'c>(
    stmts_iter: impl IntoIterator<Item = Stmt> + 'c,
    has_super_class: bool,
    args_ident: Option<&Ident>,
) -> ClassMember {
    // Add `super(..._args);` statement and `..._args` param if class has a super
    // class.
    let mut params_rest = None;
    let stmts: Vec<Stmt> = if has_super_class {
        let args_ident = args_ident.expect("args_ident is required when has_super_class is true");
        params_rest = Some(Pat::Rest(RestPat {
            span: DUMMY_SP,
            dot3_token: DUMMY_SP,
            arg: Box::new(Pat::Ident(BindingIdent {
                id: args_ident.clone(),
                type_ann: None,
            })),
            type_ann: None,
        }));
        std::iter::once(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(create_super_call(args_ident)),
        }))
        .chain(stmts_iter)
        .collect()
    } else {
        stmts_iter.into_iter().collect()
    };

    let params = vec![];

    create_class_constructor_with_params(stmts, params, params_rest)
}

/// `constructor(params) { statements }`
pub fn create_class_constructor_with_params(
    stmts: Vec<Stmt>,
    params: Vec<Param>,
    _params_rest: Option<Pat>,
) -> ClassMember {
    ClassMember::Constructor(Constructor {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        key: PropName::Ident(IdentName {
            span: DUMMY_SP,
            sym: Atom::from("constructor"),
        }),
        params: params.into_iter().map(ParamOrTsParamProp::Param).collect(),
        body: Some(BlockStmt {
            span: DUMMY_SP,
            stmts,
            ..Default::default()
        }),
        accessibility: None,
        is_optional: false,
    })
}
