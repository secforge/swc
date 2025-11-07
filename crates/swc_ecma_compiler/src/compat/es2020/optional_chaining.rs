//! ES2020: Optional Chaining
//!
//! This plugin transforms optional chaining expressions into conditional
//! expressions with null/undefined checks.
//!
//! > This plugin is included in `preset-env`, in ES2020.
//!
//! ## Example
//!
//! Input:
//! ```js
//! const foo = {};
//! // Read
//! foo?.bar?.baz;
//! // Call
//! foo?.bar?.baz?.();
//! // Delete
//! delete foo?.bar?.baz;
//! ```
//!
//! Output:
//! ```js
//! var _foo$bar, _foo$bar2, _foo$bar2$baz, _foo$bar3;
//! const foo = {};
//! // Read
//! foo === null || foo === void 0 || (_foo$bar = foo.bar) === null ||
//!   _foo$bar === void 0 ? void 0 : _foo$bar.baz;
//! // Call
//! foo === null || foo === void 0 || (_foo$bar2 = foo.bar) === null ||
//!   _foo$bar2 === void 0 || (_foo$bar2$baz = _foo$bar2.baz) === null ||
//!   _foo$bar2$baz === void 0 ? void 0 : _foo$bar2$baz.call(_foo$bar2);
//! // Delete
//! foo === null || foo === void 0 || (_foo$bar3 = foo.bar) === null ||
//!   _foo$bar3 === void 0 ? true : delete _foo$bar3.baz;
//! ```
//!
//! ## Implementation
//!
//! This is a simplified SWC-based implementation. The full implementation would
//! require more sophisticated handling of nested optional chains and call
//! contexts.
//!
//! ## References
//!
//! * Babel docs: <https://babeljs.io/docs/en/babel-plugin-proposal-optional-chaining>
//! * Babel implementation: <https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-transform-optional-chaining>
//! * Optional chaining TC39 proposal: <https://github.com/tc39/proposal-optional-chaining>

use std::mem;

use swc_atoms::Atom;
use swc_common::{util::take::Take, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::{
    common::var_declarations::VarDeclarationsStore, context::TransformCtx,
    utils::ast_builder::wrap_expression_in_arrow_function_iife,
};

/// Optional chaining transformer.
///
/// Transforms optional chaining syntax (`?.`) to conditional expressions with
/// null/undefined checks.
pub struct OptionalChaining<'ctx> {
    ctx: &'ctx TransformCtx,
    var_declarations: VarDeclarationsStore,
    uid_counter: usize,
    /// Track if we're inside a function parameter (requires special handling)
    is_inside_function_parameter: bool,
}

impl<'ctx> OptionalChaining<'ctx> {
    /// Create a new OptionalChaining transformer.
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self {
            ctx,
            var_declarations: VarDeclarationsStore::new(),
            uid_counter: 0,
            is_inside_function_parameter: false,
        }
    }

    /// Generate a unique identifier.
    fn generate_uid(&mut self, base_name: &str) -> BindingIdent {
        self.uid_counter += 1;
        let name = Atom::from(format!(
            "_{}{}",
            base_name,
            if self.uid_counter > 1 {
                self.uid_counter.to_string()
            } else {
                String::new()
            }
        ));
        BindingIdent {
            id: Ident::new(name, DUMMY_SP, Default::default()),
            type_ann: None,
        }
    }

    /// Create `void 0` expression.
    fn create_void_0() -> Expr {
        Expr::Unary(UnaryExpr {
            span: DUMMY_SP,
            op: op!("void"),
            arg: Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: 0.0,
                raw: None,
            }))),
        })
    }

    /// Create null check: `expr === null`
    fn create_null_check(&self, expr: Expr) -> Expr {
        let op = if self.ctx.assumptions.no_document_all {
            op!("==")
        } else {
            op!("===")
        };

        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op,
            left: Box::new(expr),
            right: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
        })
    }

    /// Create void 0 check: `expr === void 0`
    fn create_void_0_check(expr: Expr) -> Expr {
        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: op!("==="),
            left: Box::new(expr),
            right: Box::new(Self::create_void_0()),
        })
    }

    /// Create combined nullish check: `expr === null || expr === void 0`
    fn create_nullish_check(&self, expr1: Expr, expr2: Expr) -> Expr {
        let null_check = self.create_null_check(expr1);
        let void_check = Self::create_void_0_check(expr2);

        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: op!("||"),
            left: Box::new(null_check),
            right: Box::new(void_check),
        })
    }

    /// Transform an optional member expression.
    ///
    /// `obj?.prop` -> `obj === null || obj === void 0 ? void 0 : obj.prop`
    fn transform_optional_member(&mut self, expr: &mut Expr) -> bool {
        let mut transformed = false;

        match expr {
            Expr::OptChain(opt_chain) => {
                // This is an optional chaining expression
                let base = mem::take(&mut *opt_chain.base);

                match base {
                    OptChainBase::Member(mut member) => {
                        // Transform the object recursively
                        self.transform_optional_member(&mut member.obj);

                        if member.obj.is_ident() {
                            // Simple case: `foo?.bar`
                            let ident = member.obj.as_ident().unwrap().clone();
                            let obj_expr = Expr::Ident(ident.clone());

                            // Create the non-optional member expression
                            let member_expr = Expr::Member(MemberExpr {
                                span: member.span,
                                obj: Box::new(Expr::Ident(ident.clone())),
                                prop: member.prop,
                            });

                            // Create the conditional
                            let test = if self.ctx.assumptions.no_document_all {
                                self.create_null_check(obj_expr)
                            } else {
                                self.create_nullish_check(
                                    Expr::Ident(ident.clone()),
                                    Expr::Ident(ident),
                                )
                            };

                            *expr = Expr::Cond(CondExpr {
                                span: DUMMY_SP,
                                test: Box::new(test),
                                cons: Box::new(Self::create_void_0()),
                                alt: Box::new(member_expr),
                            });
                            transformed = true;
                        } else {
                            // Complex case: need to create a temporary variable
                            let binding = self.generate_uid("temp");
                            self.var_declarations.insert_var(&binding, None);

                            // Create assignment: `_temp = obj`
                            let assignment = Expr::Assign(AssignExpr {
                                span: DUMMY_SP,
                                op: op!("="),
                                left: AssignTarget::Simple(SimpleAssignTarget::Ident(
                                    binding.clone(),
                                )),
                                right: Box::new(*member.obj),
                            });

                            // Create member expression using the temp variable
                            let member_expr = Expr::Member(MemberExpr {
                                span: member.span,
                                obj: Box::new(Expr::Ident(binding.id.clone())),
                                prop: member.prop,
                            });

                            // Create the test
                            let test = if self.ctx.assumptions.no_document_all {
                                self.create_null_check(assignment)
                            } else {
                                self.create_nullish_check(
                                    assignment,
                                    Expr::Ident(binding.id.clone()),
                                )
                            };

                            *expr = Expr::Cond(CondExpr {
                                span: DUMMY_SP,
                                test: Box::new(test),
                                cons: Box::new(Self::create_void_0()),
                                alt: Box::new(member_expr),
                            });
                            transformed = true;
                        }
                    }
                    OptChainBase::Call(mut call) => {
                        // Handle optional call: `foo?.()`
                        // For simplicity, transform to: `foo === null || foo === void 0 ? void 0 :
                        // foo()`
                        let callee_box = mem::take(&mut call.callee);
                        let callee_expr = *callee_box;

                        match &callee_expr {
                            Expr::Ident(ident) => {
                                let test = if self.ctx.assumptions.no_document_all {
                                    self.create_null_check(Expr::Ident(ident.clone()))
                                } else {
                                    self.create_nullish_check(
                                        Expr::Ident(ident.clone()),
                                        Expr::Ident(ident.clone()),
                                    )
                                };

                                let call_expr = Expr::Call(CallExpr {
                                    span: call.span,
                                    callee: Callee::Expr(Box::new(Expr::Ident(ident.clone()))),
                                    args: call.args,
                                    ..Default::default()
                                });

                                *expr = Expr::Cond(CondExpr {
                                    span: DUMMY_SP,
                                    test: Box::new(test),
                                    cons: Box::new(Self::create_void_0()),
                                    alt: Box::new(call_expr),
                                });
                                transformed = true;
                            }
                            _ => {
                                // Complex callee - create temp variable
                                let binding = self.generate_uid("temp");
                                self.var_declarations.insert_var(&binding, None);

                                let assignment = Expr::Assign(AssignExpr {
                                    span: DUMMY_SP,
                                    op: op!("="),
                                    left: AssignTarget::Simple(SimpleAssignTarget::Ident(
                                        binding.clone(),
                                    )),
                                    right: Box::new(callee_expr),
                                });

                                let test = if self.ctx.assumptions.no_document_all {
                                    self.create_null_check(assignment)
                                } else {
                                    self.create_nullish_check(
                                        assignment,
                                        Expr::Ident(binding.id.clone()),
                                    )
                                };

                                let call_expr = Expr::Call(CallExpr {
                                    span: call.span,
                                    callee: Callee::Expr(Box::new(Expr::Ident(binding.id))),
                                    args: call.args,
                                    ..Default::default()
                                });

                                *expr = Expr::Cond(CondExpr {
                                    span: DUMMY_SP,
                                    test: Box::new(test),
                                    cons: Box::new(Self::create_void_0()),
                                    alt: Box::new(call_expr),
                                });
                                transformed = true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        transformed
    }
}

impl VisitMutHook for OptionalChaining<'_> {
    fn enter_expr(&mut self, expr: &mut Expr) {
        // Handle optional chaining in function parameters specially
        if self.is_inside_function_parameter && matches!(expr, Expr::OptChain(_)) {
            let taken = mem::replace(expr, Expr::Invalid(Invalid { span: DUMMY_SP }));
            *expr = wrap_expression_in_arrow_function_iife(Box::new(taken), DUMMY_SP);
        }
    }

    fn exit_expr(&mut self, expr: &mut Expr) {
        // Transform optional chaining after visiting children
        self.transform_optional_member(expr);
    }

    fn enter_param(&mut self, _param: &mut Param) {
        self.is_inside_function_parameter = true;
    }

    fn exit_param(&mut self, _param: &mut Param) {
        self.is_inside_function_parameter = false;
    }

    fn enter_module(&mut self, _module: &mut Module) {
        // Record entering statements
        self.var_declarations.record_entering_statements();
    }

    fn exit_module(&mut self, module: &mut Module) {
        // Insert variable declarations at the top of the module
        for item in &mut module.body {
            if let ModuleItem::Stmt(stmt) = item {
                if let Stmt::Block(block) = stmt {
                    self.var_declarations
                        .insert_into_statements(&mut block.stmts);
                    break;
                }
            }
        }
    }

    fn enter_block_stmt(&mut self, _block: &mut BlockStmt) {
        // Record entering statements
        self.var_declarations.record_entering_statements();
    }

    fn exit_block_stmt(&mut self, block: &mut BlockStmt) {
        // Insert variable declarations
        self.var_declarations
            .insert_into_statements(&mut block.stmts);
    }
}
