//! ES2021: Logical Assignment Operators
//!
//! This plugin transforms logical assignment operators (`&&=`, `||=`, `??=`)
//! to a series of logical expressions.
//!
//! > This plugin is included in `preset-env`, in ES2021
//!
//! ## Example
//!
//! Input:
//! ```js
//! a ||= b;
//! obj.a.b ||= c;
//!
//! a &&= b;
//! obj.a.b &&= c;
//! ```
//!
//! Output:
//! ```js
//! var _obj$a, _obj$a2;
//!
//! a || (a = b);
//! (_obj$a = obj.a).b || (_obj$a.b = c);
//!
//! a && (a = b);
//! (_obj$a2 = obj.a).b && (_obj$a2.b = c);
//! ```
//!
//! ### With Nullish Coalescing
//!
//! > While using the [nullish-coalescing-operator](https://github.com/oxc-project/oxc/blob/main/crates/oxc_transformer/src/es2020/nullish_coalescing_operator.rs)
//! > plugin (included in `preset-env``)
//!
//! Input:
//! ```js
//! a ??= b;
//! obj.a.b ??= c;
//! ```
//!
//! Output:
//! ```js
//! var _a, _obj$a, _obj$a$b;
//!
//! (_a = a) !== null && _a !== void 0 ? _a : (a = b);
//! (_obj$a$b = (_obj$a = obj.a).b) !== null && _obj$a$b !== void 0
//! ? _obj$a$b
//! : (_obj$a.b = c);
//! ```
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-logical-assignment-operators](https://babel.dev/docs/babel-plugin-transform-logical-assignment-operators).
//!
//! ## References:
//! * Babel plugin implementation: <https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-transform-logical-assignment-operators>
//! * Logical Assignment TC39 proposal: <https://github.com/tc39/proposal-logical-assignment>

use std::mem;

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::{common::duplicate::duplicate_expression, context::TransformCtx};

pub struct LogicalAssignmentOperators<'ctx> {
    ctx: &'ctx TransformCtx,
}

impl<'ctx> LogicalAssignmentOperators<'ctx> {
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self { ctx }
    }
}

impl VisitMutHook for LogicalAssignmentOperators<'_> {
    // `#[inline]` because this is a hot path, and most `Expr`s are not
    // `AssignExpr`s with a logical operator. So we want to bail out as fast as
    // possible for everything else, without the cost of a function call.
    #[inline]
    fn enter_expr(&mut self, expr: &mut Expr) {
        let Expr::Assign(assign_expr) = expr else {
            return;
        };

        // `&&=` `||=` `??=`
        let Some(operator) = assign_op_to_binary_op(assign_expr.op) else {
            return;
        };

        self.transform_logical_assignment(expr, operator);
    }
}

impl LogicalAssignmentOperators<'_> {
    fn transform_logical_assignment(&self, expr: &mut Expr, operator: BinaryOp) {
        let Expr::Assign(assign_expr) = expr else {
            unreachable!()
        };

        // `a &&= c` -> `a && (a = c);`
        //               ^     ^ assign_target
        //               ^ left_expr

        // TODO: Add tests, cover private identifier
        let (left_expr, assign_target) = match &mut assign_expr.left {
            // `a &&= c` -> `a && (a = c)`
            AssignTarget::Simple(SimpleAssignTarget::Ident(ident)) => {
                Self::convert_identifier(ident)
            }
            // `a.b &&= c` -> `var _a; (_a = a).b && (_a.b = c)`
            AssignTarget::Simple(SimpleAssignTarget::Member(member_expr)) => {
                self.convert_member_expression(member_expr)
            }
            // TODO
            #[expect(clippy::match_same_arms)]
            AssignTarget::Simple(SimpleAssignTarget::SuperProp(_)) => return,
            // All other are TypeScript syntax.

            // It is a Syntax Error if AssignmentTargetType of LeftHandSideExpression is not simple.
            // So safe to return here.
            _ => return,
        };

        let right = mem::replace(
            &mut assign_expr.right,
            Box::new(Expr::Invalid(Invalid { span: DUMMY_SP })),
        );

        let right = Box::new(Expr::Assign(AssignExpr {
            span: DUMMY_SP,
            op: op!("="),
            left: assign_target,
            right,
        }));

        let bin_expr = BinExpr {
            span: DUMMY_SP,
            op: operator,
            left: left_expr,
            right,
        };

        *expr = Expr::Bin(bin_expr);
    }

    fn convert_identifier(ident: &BindingIdent) -> (Box<Expr>, AssignTarget) {
        let left_expr = Box::new(Expr::Ident(ident.id.clone()));

        let assign_target = AssignTarget::Simple(SimpleAssignTarget::Ident(ident.clone()));
        (left_expr, assign_target)
    }

    fn convert_member_expression(&self, member_expr: &mut MemberExpr) -> (Box<Expr>, AssignTarget) {
        let span = member_expr.span;
        let object = mem::replace(
            &mut member_expr.obj,
            Box::new(Expr::Invalid(Invalid { span: DUMMY_SP })),
        );

        let result = duplicate_expression(object, 1, Atom::from("_obj"));
        let object = result.original;
        let object_ref = result.duplicates.into_iter().next().unwrap();

        // Take ownership of the property to avoid borrow conflicts
        let prop = mem::replace(
            &mut member_expr.prop,
            MemberProp::Ident(IdentName::new("__tmp".into(), DUMMY_SP)),
        );

        let (left_expr, assign_target_prop) = match prop {
            MemberProp::Ident(ident_name) => {
                let left = Box::new(Expr::Member(MemberExpr {
                    span,
                    obj: object,
                    prop: MemberProp::Ident(ident_name.clone()),
                }));
                let target_prop = MemberProp::Ident(ident_name);
                (left, target_prop)
            }
            MemberProp::Computed(computed_prop) => {
                let expression = computed_prop.expr;

                let result = duplicate_expression(expression, 1, Atom::from("_key"));
                let expression = result.original;
                let expression_ref = result.duplicates.into_iter().next().unwrap();

                let left = Box::new(Expr::Member(MemberExpr {
                    span,
                    obj: object,
                    prop: MemberProp::Computed(ComputedPropName {
                        span: DUMMY_SP,
                        expr: expression,
                    }),
                }));

                let target_prop = MemberProp::Computed(ComputedPropName {
                    span: DUMMY_SP,
                    expr: expression_ref,
                });

                (left, target_prop)
            }
            MemberProp::PrivateName(_) => {
                // TODO: Handle private names
                return (
                    Box::new(Expr::Invalid(Invalid { span: DUMMY_SP })),
                    AssignTarget::Simple(SimpleAssignTarget::Invalid(Invalid { span: DUMMY_SP })),
                );
            }
        };

        let assign_target = AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
            span,
            obj: object_ref,
            prop: assign_target_prop,
        }));

        (left_expr, assign_target)
    }
}

/// Convert assignment operator to binary operator if it's a logical assignment
/// operator.
///
/// Returns `Some(BinaryOp)` for `&&=`, `||=`, `??=`, and `None` for other
/// assignment operators.
fn assign_op_to_binary_op(op: AssignOp) -> Option<BinaryOp> {
    match op {
        AssignOp::AndAssign => Some(BinaryOp::LogicalAnd),
        AssignOp::OrAssign => Some(BinaryOp::LogicalOr),
        AssignOp::NullishAssign => Some(BinaryOp::NullishCoalescing),
        _ => None,
    }
}
