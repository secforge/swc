//! Utilities for handling computed key expressions.
//!
//! ## Note
//!
//! This module provides utilities for handling computed keys in class
//! properties and methods. The OXC version is tightly coupled with OXC's
//! TraverseCtx and symbol management. This SWC version needs to be adapted to
//! work with SWC's visitor pattern and scope analysis.

#![allow(dead_code)]

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

/// Check if a temporary variable is required for a computed key expression.
///
/// `this` needs a temp var because `this` in computed key and `this` within
/// class constructor resolve to different `this` bindings.
///
/// ## Example
/// ```js
/// class C { [this] = 1; }
/// ```
/// becomes:
/// ```js
/// let _this; _this = this; class C { constructor() { this[_this] = 1; } }
/// ```
///
/// ## Arguments
/// * `key` - The expression used as a computed key
///
/// ## Returns
/// `true` if the key may have side effects or needs special handling, `false`
/// otherwise
pub fn key_needs_temp_var(key: &Expr) -> bool {
    match key {
        // Literals cannot have side effects.
        // e.g. `let x = 'x'; class C { [x] = 1; }` or `class C { ['x'] = 1; }`.
        Expr::Lit(Lit::Bool(_))
        | Expr::Lit(Lit::Null(_))
        | Expr::Lit(Lit::Num(_))
        | Expr::Lit(Lit::BigInt(_))
        | Expr::Lit(Lit::Regex(_))
        | Expr::Lit(Lit::Str(_)) => false,
        // Template literal cannot have side effects if it has no expressions.
        // If it *does* have expressions, but they're all literals, then also cannot have side
        // effects, but don't bother checking for that as it shouldn't occur in real world
        // code. Why would you write "`x${9}z`" when you can just write "`x9z`"?
        // Note: "`x${foo}`" *can* have side effects if `foo` is an object with a `toString` method.
        Expr::Tpl(tpl) => !tpl.exprs.is_empty(),
        // IdentifierReferences can have side effects if unbound, or if the variable is mutated.
        //
        // If var is mutated, it also needs a temp var, because of cases like
        // `let x = 1; class { [x] = 1; [++x] = 2; }`
        // `++x` is hoisted to before class in output, so `x` in 1st key would get the wrong value
        // unless it's hoisted out too.
        //
        // TODO: This simplified version always returns true for identifiers.
        // A full implementation would need to check if the identifier is bound and not mutated
        // using SWC's scope analysis.
        Expr::Ident(_) => true,
        // Treat any other expression as possibly having side effects e.g. `foo()`.
        // TODO: Do fuller analysis to detect expressions which cannot have side effects.
        // e.g. `"x" + "y"`.
        _ => true,
    }
}

/// Create a temporary variable for a computed key.
///
/// This function generates:
/// 1. A `let _x;` declaration to be inserted before the class
/// 2. An assignment expression `_x = key`
/// 3. An identifier expression referencing `_x`
///
/// ## Note
///
/// The OXC version uses `TraverseCtx` to generate UIDs and manage scope.
/// This SWC version would need to be integrated with SWC's scope management
/// system.
///
/// ## Arguments
/// * `key` - The computed key expression
/// * `ident_name` - Name to use for the temporary variable
///
/// ## Returns
/// A tuple of:
/// - The assignment expression `_x = key`
/// - An identifier expression referencing the temp var
pub fn create_computed_key_temp_var(key: Box<Expr>, ident_name: Atom) -> (Box<Expr>, Box<Expr>) {
    // Create the identifier for the temp variable
    let temp_ident = Ident {
        span: DUMMY_SP,
        ctxt: Default::default(),
        sym: ident_name.clone(),
        optional: false,
    };

    // Create assignment: _x = key
    let assignment = Box::new(Expr::Assign(AssignExpr {
        span: DUMMY_SP,
        op: op!("="),
        left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent {
            id: temp_ident.clone(),
            type_ann: None,
        })),
        right: key,
    }));

    // Create identifier reference: _x
    let ident = Box::new(Expr::Ident(temp_ident));

    (assignment, ident)
}

/// Create a `let` declaration for a computed key temp variable.
///
/// ## Arguments
/// * `ident_name` - Name of the temporary variable
///
/// ## Returns
/// A `VarDecl` with `let` kind and no initializer
pub fn create_computed_key_temp_var_declaration(ident_name: Atom) -> VarDecl {
    VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Let,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: Ident {
                    span: DUMMY_SP,
                    ctxt: Default::default(),
                    sym: ident_name,
                    optional: false,
                },
                type_ann: None,
            }),
            init: None,
            definite: false,
        }],
        ..Default::default()
    }
}
