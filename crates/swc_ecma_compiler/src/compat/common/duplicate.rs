//! Utilities for duplicating expressions.
//!
//! ## Overview
//!
//! This module provides utilities to duplicate expressions that need to be used
//! multiple times. When an expression may have side effects, a temporary
//! variable is created to avoid executing the expression multiple times.
//!
//! ## Note
//!
//! The OXC version uses arena allocation and tight integration with semantic
//! analysis. This SWC version provides similar functionality adapted to SWC's
//! architecture.

#![allow(dead_code)]

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

/// Result of duplicating an expression.
///
/// Contains the original/assignment expression that must be inserted first,
/// and one or more duplicate expressions that can be used in subsequent
/// positions.
pub struct DuplicateExprResult {
    /// The first expression, which may be an assignment if a temp var was
    /// needed
    pub original: Box<Expr>,
    /// Duplicate expressions that reference the same value
    pub duplicates: Vec<Box<Expr>>,
}

/// Duplicate an expression to be used multiple times.
///
/// If `expr` may have side effects, creates a temp var `_expr` and assigns to
/// it.
///
/// ## Examples
///
/// * `this` -> `this`, `[this, this]`
/// * Literal `123` -> `123`, `[123, 123]`
/// * Identifier `foo` -> `foo`, `[foo, foo]` (if bound and not mutated)
/// * Unbound identifier `foo` -> `_foo = foo`, `[_foo, _foo]`
/// * Function call `foo()` -> `_foo = foo()`, `[_foo, _foo]`
///
/// ## Arguments
/// * `expr` - The expression to duplicate
/// * `count` - Number of duplicate expressions needed
/// * `temp_var_name` - Name to use for temp variable if needed
///
/// ## Returns
/// `DuplicateExprResult` containing the first expression and the duplicates
pub fn duplicate_expression(
    expr: Box<Expr>,
    count: usize,
    temp_var_name: Atom,
) -> DuplicateExprResult {
    match expr.as_ref() {
        // Reading any of these cannot have side effects, so no need for temp var
        Expr::This(_)
        | Expr::SuperProp(_)
        | Expr::Lit(Lit::Bool(_))
        | Expr::Lit(Lit::Null(_))
        | Expr::Lit(Lit::Num(_))
        | Expr::Lit(Lit::BigInt(_))
        | Expr::Lit(Lit::Regex(_))
        | Expr::Lit(Lit::Str(_)) => {
            let duplicates = (0..count).map(|_| expr.clone()).collect();
            DuplicateExprResult {
                original: expr,
                duplicates,
            }
        }
        // Template literal cannot have side effects if it has no expressions
        Expr::Tpl(tpl) if tpl.exprs.is_empty() => {
            let duplicates = (0..count).map(|_| expr.clone()).collect();
            DuplicateExprResult {
                original: expr,
                duplicates,
            }
        }
        // For identifiers, we would need scope analysis to determine if they're bound and not
        // mutated. For simplicity, this version creates a temp var for all identifiers.
        // TODO: Integrate with SWC's scope analysis to avoid unnecessary temp vars.
        Expr::Ident(_) => create_temp_var_for_expr(expr, count, temp_var_name),
        // Anything else requires temp var
        _ => create_temp_var_for_expr(expr, count, temp_var_name),
    }
}

/// Create a temporary variable assignment for an expression and return
/// duplicates.
///
/// Creates:
/// - Assignment: `_temp = expr`
/// - Duplicates: `[_temp, _temp, ...]`
fn create_temp_var_for_expr(
    expr: Box<Expr>,
    count: usize,
    temp_var_name: Atom,
) -> DuplicateExprResult {
    let temp_ident = Ident {
        span: DUMMY_SP,
        ctxt: Default::default(),
        sym: temp_var_name.clone(),
        optional: false,
    };

    // Create assignment: _temp = expr
    let assignment = Box::new(Expr::Assign(AssignExpr {
        span: DUMMY_SP,
        op: op!("="),
        left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent {
            id: temp_ident.clone(),
            type_ann: None,
        })),
        right: expr,
    }));

    // Create duplicate references
    let duplicates = (0..count)
        .map(|_| {
            Box::new(Expr::Ident(Ident {
                span: DUMMY_SP,
                ctxt: temp_ident.ctxt,
                sym: temp_var_name.clone(),
                optional: false,
            }))
        })
        .collect();

    DuplicateExprResult {
        original: assignment,
        duplicates,
    }
}

/// Create a variable declaration for a temp variable.
///
/// ## Arguments
/// * `temp_var_name` - Name of the temporary variable
/// * `init` - Optional initializer expression
///
/// ## Returns
/// A `VarDecl` with `var` kind
pub fn create_temp_var_declaration(temp_var_name: Atom, init: Option<Box<Expr>>) -> VarDecl {
    VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Var,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: Ident {
                    span: DUMMY_SP,
                    ctxt: Default::default(),
                    sym: temp_var_name,
                    optional: false,
                },
                type_ann: None,
            }),
            init,
            definite: false,
        }],
        ..Default::default()
    }
}

/// Check if an expression can be safely duplicated without side effects.
///
/// ## Arguments
/// * `expr` - The expression to check
///
/// ## Returns
/// `true` if the expression can be duplicated without side effects
pub fn can_duplicate_without_side_effects(expr: &Expr) -> bool {
    match expr {
        Expr::This(_)
        | Expr::Lit(Lit::Bool(_))
        | Expr::Lit(Lit::Null(_))
        | Expr::Lit(Lit::Num(_))
        | Expr::Lit(Lit::BigInt(_))
        | Expr::Lit(Lit::Regex(_))
        | Expr::Lit(Lit::Str(_)) => true,
        Expr::Tpl(tpl) => tpl.exprs.is_empty(),
        _ => false,
    }
}
