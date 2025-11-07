//! ES2016: Exponentiation Operator
//!
//! This plugin transforms the exponentiation operator (`**`) to `Math.pow`.
//!
//! > This plugin is included in `preset-env`, in ES2016
//!
//! ## Example
//!
//! Input:
//! ```js
//! let x = 10 ** 2;
//! x **= 3;
//! obj.prop **= 4;
//! ```
//!
//! Output:
//! ```js
//! let x = Math.pow(10, 2);
//! x = Math.pow(x, 3);
//! obj["prop"] = Math.pow(obj["prop"], 4);
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-exponentiation-operator](https://babel.dev/docs/babel-plugin-transform-exponentiation-operator).
//!
//! ## References:
//!
//! * Babel plugin implementation: <https://github.com/babel/babel/blob/v7.26.2/packages/babel-plugin-transform-exponentiation-operator>
//!   <https://github.com/babel/babel/tree/v7.26.2/packages/babel-helper-builder-binary-assignment-operator-visitor>
//! * Exponentiation operator TC39 proposal: <https://github.com/tc39/proposal-exponentiation-operator>
//! * Exponentiation operator specification: <https://tc39.es/ecma262/#sec-exp-operator>

#![allow(dead_code)]
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

/// Exponentiation operator transformer.
///
/// Transforms `**` and `**=` operators to `Math.pow` calls.
pub struct ExponentiationOperator<'ctx> {
    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,
}

impl<'ctx> ExponentiationOperator<'ctx> {
    /// Create a new exponentiation operator transformer.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self { ctx }
    }
}

impl VisitMutHook for ExponentiationOperator<'_> {
    // Note: Do not transform to `Math.pow` with BigInt arguments - that's a runtime
    // error
    fn enter_expr(&mut self, expr: &mut Expr) {
        match expr {
            // `left ** right`
            Expr::Bin(bin_expr) if bin_expr.op == BinaryOp::Exp => {
                if self.is_big_int_literal(&bin_expr.left)
                    || self.is_big_int_literal(&bin_expr.right)
                {
                    return;
                }

                Self::convert_binary_expression(expr);
            }
            // `left **= right`
            Expr::Assign(assign_expr) if assign_expr.op == AssignOp::ExpAssign => {
                if self.is_big_int_literal(&assign_expr.right) {
                    return;
                }

                match &assign_expr.left {
                    AssignTarget::Simple(SimpleAssignTarget::Ident(_)) => {
                        self.convert_identifier_assignment(expr);
                    }
                    AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                        prop: MemberProp::Ident(_),
                        ..
                    })) => {
                        self.convert_static_member_expression_assignment(expr);
                    }
                    AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                        prop: MemberProp::Computed(_),
                        ..
                    })) => {
                        self.convert_computed_member_expression_assignment(expr);
                    }
                    AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                        prop: MemberProp::PrivateName(_),
                        ..
                    })) => {
                        self.convert_private_field_assignment(expr);
                    }
                    AssignTarget::Simple(SimpleAssignTarget::SuperProp(_))
                    | AssignTarget::Simple(SimpleAssignTarget::OptChain(_))
                    | AssignTarget::Simple(SimpleAssignTarget::Paren(_))
                    | AssignTarget::Simple(SimpleAssignTarget::TsAs(_))
                    | AssignTarget::Simple(SimpleAssignTarget::TsSatisfies(_))
                    | AssignTarget::Simple(SimpleAssignTarget::TsNonNull(_))
                    | AssignTarget::Simple(SimpleAssignTarget::TsTypeAssertion(_))
                    | AssignTarget::Simple(SimpleAssignTarget::TsInstantiation(_))
                    | AssignTarget::Simple(SimpleAssignTarget::Invalid(_))
                    | AssignTarget::Pat(_) => {
                        // These assignment targets are not supported for
                        // transformation
                        // Skip transformation
                    }
                }
            }
            _ => {}
        }
    }
}

impl ExponentiationOperator<'_> {
    /// Check if an expression is a BigInt literal.
    fn is_big_int_literal(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Lit(Lit::BigInt(_)))
    }

    /// Convert `BinaryExpression`.
    ///
    /// `left ** right` -> `Math.pow(left, right)`
    #[inline]
    fn convert_binary_expression(expr: &mut Expr) {
        let Expr::Bin(bin_expr) = expr else {
            unreachable!()
        };

        let left = *bin_expr.left.clone();
        let right = *bin_expr.right.clone();

        *expr = Self::math_pow(left, right);
    }

    /// Convert `AssignmentExpression` where assignee is an identifier.
    ///
    /// `left **= right` transformed to:
    /// * If `left` is a bound symbol: -> `left = Math.pow(left, right)`
    /// * If `left` is unbound: -> `var _left; _left = left, left =
    ///   Math.pow(_left, right)`
    ///
    /// Temporary variable `_left` is to avoid side-effects of getting `left`
    /// from running twice.
    #[inline]
    fn convert_identifier_assignment(&self, expr: &mut Expr) {
        let Expr::Assign(assign_expr) = expr else {
            unreachable!()
        };
        let AssignTarget::Simple(SimpleAssignTarget::Ident(ident)) = &assign_expr.left else {
            unreachable!()
        };

        // For now, simplified version without temp variables
        // TODO: Add proper symbol tracking to determine if temp var is needed
        let pow_left = Expr::Ident(ident.id.clone());
        Self::convert_assignment(assign_expr, pow_left);
    }

    /// Convert `AssignmentExpression` where assignee is a static member
    /// expression.
    ///
    /// `obj.prop **= right` transformed to:
    /// * If `obj` is a bound symbol: -> `obj["prop"] = Math.pow(obj["prop"],
    ///   right)`
    /// * If `obj` is unbound: -> `var _obj; _obj = obj, _obj["prop"] =
    ///   Math.pow(_obj["prop"], right)`
    ///
    /// TODO(improve-on-babel): `obj.prop` does not need to be transformed to
    /// `obj["prop"]`.
    #[inline]
    fn convert_static_member_expression_assignment(&self, expr: &mut Expr) {
        let Expr::Assign(assign_expr) = expr else {
            unreachable!()
        };
        let AssignTarget::Simple(SimpleAssignTarget::Member(member_expr)) = &assign_expr.left
        else {
            unreachable!()
        };
        let MemberProp::Ident(prop_ident) = &member_expr.prop else {
            unreachable!()
        };

        // Create computed member expression for both sides
        let obj = (*member_expr.obj).clone();
        let prop_str = prop_ident.sym.to_string();

        // Left side: obj["prop"]
        let left_member = MemberExpr {
            span: DUMMY_SP,
            obj: member_expr.obj.clone(),
            prop: MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: prop_str.clone().into(),
                    raw: None,
                }))),
            }),
        };

        // Right side: Math.pow(obj["prop"], right)
        let pow_left_member = MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(obj),
            prop: MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: prop_str.into(),
                    raw: None,
                }))),
            }),
        };

        let pow_left = Expr::Member(pow_left_member);

        // Update assignment
        assign_expr.left = AssignTarget::Simple(SimpleAssignTarget::Member(left_member));
        Self::convert_assignment(assign_expr, pow_left);
    }

    /// Convert `AssignmentExpression` where assignee is a computed member
    /// expression.
    ///
    /// `obj[prop] **= right` transformed to:
    /// * If `obj` is a bound symbol: -> `var _prop; _prop = prop, obj[_prop] =
    ///   Math.pow(obj[_prop], 2)`
    /// * If `obj` is unbound: -> `var _obj, _prop; _obj = obj, _prop = prop,
    ///   _obj[_prop] = Math.pow(_obj[_prop], 2)`
    ///
    /// TODO(improve-on-babel):
    /// 1. If `prop` is bound, it doesn't need a temp variable `_prop`.
    /// 2. Temp var initializations could be inlined.
    #[inline]
    fn convert_computed_member_expression_assignment(&self, expr: &mut Expr) {
        let Expr::Assign(assign_expr) = expr else {
            unreachable!()
        };
        let AssignTarget::Simple(SimpleAssignTarget::Member(member_expr)) = &assign_expr.left
        else {
            unreachable!()
        };
        let MemberProp::Computed(computed_prop) = &member_expr.prop else {
            unreachable!()
        };

        // For now, simplified version without temp variables
        // Clone the property expression for use in Math.pow
        let pow_left = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: member_expr.obj.clone(),
            prop: MemberProp::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: computed_prop.expr.clone(),
            }),
        });

        Self::convert_assignment(assign_expr, pow_left);
    }

    /// Convert `AssignmentExpression` where assignee is a private field member
    /// expression.
    ///
    /// `obj.#prop **= right` transformed to:
    /// * If `obj` is a bound symbol: -> `obj.#prop = Math.pow(obj.#prop,
    ///   right)`
    /// * If `obj` is unbound: -> `var _obj; _obj = obj, _obj.#prop =
    ///   Math.pow(_obj.#prop, right)`
    #[inline]
    fn convert_private_field_assignment(&self, expr: &mut Expr) {
        let Expr::Assign(assign_expr) = expr else {
            unreachable!()
        };
        let AssignTarget::Simple(SimpleAssignTarget::Member(member_expr)) = &assign_expr.left
        else {
            unreachable!()
        };
        let MemberProp::PrivateName(private_name) = &member_expr.prop else {
            unreachable!()
        };

        // Create duplicate private member expression for Math.pow
        let pow_left = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: member_expr.obj.clone(),
            prop: MemberProp::PrivateName(private_name.clone()),
        });

        Self::convert_assignment(assign_expr, pow_left);
    }

    /// `x **= right` -> `x = Math.pow(pow_left, right)` (with provided
    /// `pow_left`)
    fn convert_assignment(assign_expr: &mut AssignExpr, pow_left: Expr) {
        let pow_right = (*assign_expr.right).clone();
        assign_expr.right = Box::new(Self::math_pow(pow_left, pow_right));
        assign_expr.op = AssignOp::Assign;
    }

    /// `Math.pow(left, right)`
    fn math_pow(left: Expr, right: Expr) -> Expr {
        // Create Math identifier
        let math_ident = Ident {
            span: DUMMY_SP,
            ctxt: Default::default(),
            sym: "Math".into(),
            optional: false,
        };

        // Create Math.pow member expression
        let pow_prop = IdentName {
            span: DUMMY_SP,
            sym: "pow".into(),
        };

        let callee = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(math_ident)),
            prop: MemberProp::Ident(pow_prop),
        });

        // Create call expression
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(Box::new(callee)),
            args: vec![
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(left),
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(right),
                },
            ],
            type_args: None,
        })
    }
}
