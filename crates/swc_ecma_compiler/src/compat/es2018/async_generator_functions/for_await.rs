//! This module is responsible for transforming `for await` to `for` statement.
//!
//! This implementation transforms `for await (let x of y)` statements into
//! regular `for` loops wrapped in try-catch-finally blocks for proper async
//! iteration cleanup.

use std::mem;

use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_utils::private_ident;

use super::AsyncGeneratorFunctions;
use crate::compat::common::helper_loader::Helper;

impl<'ctx> AsyncGeneratorFunctions<'ctx> {
    /// Transform a for-await statement.
    ///
    /// This method checks if a statement is a `for await` loop and transforms
    /// it into an equivalent `for` loop with proper async iteration protocol.
    pub(crate) fn transform_statement(&mut self, stmt: &mut Stmt) {
        let for_of = match stmt {
            Stmt::ForOf(for_of) if for_of.is_await => for_of,
            _ => return,
        };

        // Generate unique identifiers for iterator state management
        let step_ident = private_ident!("_step");
        let iterator_ident = private_ident!("_iterator");
        let iterator_abrupt_completion = private_ident!("_iteratorAbruptCompletion");
        let iterator_had_error = private_ident!("_didIteratorError");
        let iterator_error = private_ident!("_iteratorError");

        // Extract the loop body
        let mut body_stmts = match mem::take(&mut *for_of.body) {
            Stmt::Block(block) => block.stmts,
            other => vec![other],
        };

        // Create assignment statement: let x = _step.value
        let assignment_stmt = match &for_of.left {
            ForHead::VarDecl(var_decl) => {
                let decl = &var_decl.decls[0];
                let mut new_decl = decl.clone();
                new_decl.init = Some(Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(Expr::Ident(step_ident.clone())),
                    prop: MemberProp::Ident(IdentName::new("value".into(), DUMMY_SP)),
                })));
                Stmt::Decl(Decl::Var(Box::new(VarDecl {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    kind: var_decl.kind,
                    declare: false,
                    decls: vec![new_decl],
                })))
            }
            ForHead::Pat(pat) => {
                let target = match &**pat {
                    Pat::Ident(ident) => AssignTarget::Simple(SimpleAssignTarget::Ident(
                        BindingIdent::from(ident.id.clone()),
                    )),
                    Pat::Array(arr) => AssignTarget::Pat(AssignTargetPat::Array(ArrayPat {
                        span: arr.span,
                        elems: arr.elems.clone(),
                        optional: false,
                        type_ann: None,
                    })),
                    Pat::Object(obj) => AssignTarget::Pat(AssignTargetPat::Object(ObjectPat {
                        span: obj.span,
                        props: obj.props.clone(),
                        optional: false,
                        type_ann: None,
                    })),
                    _ => return, // Invalid pattern
                };

                Stmt::Expr(ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Assign(AssignExpr {
                        span: DUMMY_SP,
                        op: op!("="),
                        left: target,
                        right: Box::new(Expr::Member(MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(Expr::Ident(step_ident.clone())),
                            prop: MemberProp::Ident(IdentName::new("value".into(), DUMMY_SP)),
                        })),
                    })),
                })
            }
            _ => return,
        };

        // Prepend assignment to body
        body_stmts.insert(0, assignment_stmt);

        // Create the iterator expression: babelHelpers.asyncIterator(y)
        let iterator_init = Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(Ident::new(
                    "babelHelpers".into(),
                    DUMMY_SP,
                    Default::default(),
                ))),
                prop: MemberProp::Ident(IdentName::new(
                    Helper::AsyncIterator.name().into(),
                    DUMMY_SP,
                )),
            }))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(mem::take(&mut *for_of.right)),
            }],
            type_args: None,
        });

        let statements = Self::build_for_await(
            iterator_init,
            step_ident,
            iterator_ident,
            iterator_abrupt_completion,
            iterator_had_error,
            iterator_error,
            body_stmts,
        );

        // Replace the for-await statement with a block containing all the generated
        // statements
        *stmt = Stmt::Block(BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: statements,
        });
    }

    /// Build the complete for-await structure with try-catch-finally.
    ///
    /// This generates code like:
    /// ```js
    /// var _iteratorAbruptCompletion = false;
    /// var _didIteratorError = false;
    /// var _iteratorError;
    /// try {
    ///   for (
    ///     var _iterator = asyncIterator(obj), _step;
    ///     _iteratorAbruptCompletion = !(_step = await _iterator.next()).done;
    ///     _iteratorAbruptCompletion = false
    ///   ) {
    ///     // body
    ///   }
    /// } catch (err) {
    ///   _didIteratorError = true;
    ///   _iteratorError = err;
    /// } finally {
    ///   try {
    ///     if (_iteratorAbruptCompletion && _iterator.return != null) {
    ///       await _iterator.return();
    ///     }
    ///   } finally {
    ///     if (_didIteratorError) {
    ///       throw _iteratorError;
    ///     }
    ///   }
    /// }
    /// ```
    #[allow(clippy::too_many_arguments)]
    fn build_for_await(
        iterator_init: Expr,
        step_ident: Ident,
        iterator_ident: Ident,
        iterator_abrupt_completion: Ident,
        iterator_had_error: Ident,
        iterator_error: Ident,
        body: Vec<Stmt>,
    ) -> Vec<Stmt> {
        let mut statements = vec![];

        // var _iteratorAbruptCompletion = false;
        statements.push(Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(BindingIdent::from(iterator_abrupt_completion.clone())),
                init: Some(Box::new(Expr::Lit(Lit::Bool(Bool {
                    span: DUMMY_SP,
                    value: false,
                })))),
                definite: false,
            }],
        }))));

        // var _didIteratorError = false;
        statements.push(Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(BindingIdent::from(iterator_had_error.clone())),
                init: Some(Box::new(Expr::Lit(Lit::Bool(Bool {
                    span: DUMMY_SP,
                    value: false,
                })))),
                definite: false,
            }],
        }))));

        // var _iteratorError;
        statements.push(Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(BindingIdent::from(iterator_error.clone())),
                init: None,
                definite: false,
            }],
        }))));

        // Build the for loop
        let for_stmt = Stmt::For(ForStmt {
            span: DUMMY_SP,
            // var _iterator = asyncIterator(obj), _step;
            init: Some(VarDeclOrExpr::VarDecl(Box::new(VarDecl {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                kind: VarDeclKind::Var,
                declare: false,
                decls: vec![
                    VarDeclarator {
                        span: DUMMY_SP,
                        name: Pat::Ident(BindingIdent::from(iterator_ident.clone())),
                        init: Some(Box::new(iterator_init)),
                        definite: false,
                    },
                    VarDeclarator {
                        span: DUMMY_SP,
                        name: Pat::Ident(BindingIdent::from(step_ident.clone())),
                        init: None,
                        definite: false,
                    },
                ],
            }))),
            // _iteratorAbruptCompletion = !(_step = await _iterator.next()).done
            test: Some(Box::new(Expr::Assign(AssignExpr {
                span: DUMMY_SP,
                op: op!("="),
                left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent::from(
                    iterator_abrupt_completion.clone(),
                ))),
                right: Box::new(Expr::Unary(UnaryExpr {
                    span: DUMMY_SP,
                    op: op!("!"),
                    arg: Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(Expr::Paren(ParenExpr {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Assign(AssignExpr {
                                span: DUMMY_SP,
                                op: op!("="),
                                left: AssignTarget::Simple(SimpleAssignTarget::Ident(
                                    BindingIdent::from(step_ident.clone()),
                                )),
                                right: Box::new(Expr::Await(AwaitExpr {
                                    span: DUMMY_SP,
                                    arg: Box::new(Expr::Call(CallExpr {
                                        span: DUMMY_SP,
                                        ctxt: SyntaxContext::empty(),
                                        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                                            span: DUMMY_SP,
                                            obj: Box::new(Expr::Ident(iterator_ident.clone())),
                                            prop: MemberProp::Ident(IdentName::new(
                                                "next".into(),
                                                DUMMY_SP,
                                            )),
                                        }))),
                                        args: vec![],
                                        type_args: None,
                                    })),
                                })),
                            })),
                        })),
                        prop: MemberProp::Ident(IdentName::new("done".into(), DUMMY_SP)),
                    })),
                })),
            }))),
            // _iteratorAbruptCompletion = false
            update: Some(Box::new(Expr::Assign(AssignExpr {
                span: DUMMY_SP,
                op: op!("="),
                left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent::from(
                    iterator_abrupt_completion.clone(),
                ))),
                right: Box::new(Expr::Lit(Lit::Bool(Bool {
                    span: DUMMY_SP,
                    value: false,
                }))),
            }))),
            body: Box::new(Stmt::Block(BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: body,
            })),
        });

        // Build catch clause
        let err_ident = private_ident!("err");
        let catch_clause = CatchClause {
            span: DUMMY_SP,
            param: Some(Pat::Ident(BindingIdent::from(err_ident.clone()))),
            body: BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: vec![
                    // _didIteratorError = true;
                    Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(Expr::Assign(AssignExpr {
                            span: DUMMY_SP,
                            op: op!("="),
                            left: AssignTarget::Simple(SimpleAssignTarget::Ident(
                                BindingIdent::from(iterator_had_error.clone()),
                            )),
                            right: Box::new(Expr::Lit(Lit::Bool(Bool {
                                span: DUMMY_SP,
                                value: true,
                            }))),
                        })),
                    }),
                    // _iteratorError = err;
                    Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(Expr::Assign(AssignExpr {
                            span: DUMMY_SP,
                            op: op!("="),
                            left: AssignTarget::Simple(SimpleAssignTarget::Ident(
                                BindingIdent::from(iterator_error.clone()),
                            )),
                            right: Box::new(Expr::Ident(err_ident)),
                        })),
                    }),
                ],
            },
        };

        // Build finally clause with nested try-finally
        let finally_block = BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: vec![Stmt::Try(Box::new(TryStmt {
                span: DUMMY_SP,
                block: BlockStmt {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    stmts: vec![Stmt::If(IfStmt {
                        span: DUMMY_SP,
                        test: Box::new(Expr::Bin(BinExpr {
                            span: DUMMY_SP,
                            op: op!("&&"),
                            left: Box::new(Expr::Ident(iterator_abrupt_completion.clone())),
                            right: Box::new(Expr::Bin(BinExpr {
                                span: DUMMY_SP,
                                op: op!("!="),
                                left: Box::new(Expr::Member(MemberExpr {
                                    span: DUMMY_SP,
                                    obj: Box::new(Expr::Ident(iterator_ident.clone())),
                                    prop: MemberProp::Ident(IdentName::new(
                                        "return".into(),
                                        DUMMY_SP,
                                    )),
                                })),
                                right: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                            })),
                        })),
                        cons: Box::new(Stmt::Block(BlockStmt {
                            span: DUMMY_SP,
                            ctxt: SyntaxContext::empty(),
                            stmts: vec![Stmt::Expr(ExprStmt {
                                span: DUMMY_SP,
                                expr: Box::new(Expr::Await(AwaitExpr {
                                    span: DUMMY_SP,
                                    arg: Box::new(Expr::Call(CallExpr {
                                        span: DUMMY_SP,
                                        ctxt: SyntaxContext::empty(),
                                        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                                            span: DUMMY_SP,
                                            obj: Box::new(Expr::Ident(iterator_ident)),
                                            prop: MemberProp::Ident(IdentName::new(
                                                "return".into(),
                                                DUMMY_SP,
                                            )),
                                        }))),
                                        args: vec![],
                                        type_args: None,
                                    })),
                                })),
                            })],
                        })),
                        alt: None,
                    })],
                },
                handler: None,
                finalizer: Some(BlockStmt {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    stmts: vec![Stmt::If(IfStmt {
                        span: DUMMY_SP,
                        test: Box::new(Expr::Ident(iterator_had_error)),
                        cons: Box::new(Stmt::Block(BlockStmt {
                            span: DUMMY_SP,
                            ctxt: SyntaxContext::empty(),
                            stmts: vec![Stmt::Throw(ThrowStmt {
                                span: DUMMY_SP,
                                arg: Box::new(Expr::Ident(iterator_error)),
                            })],
                        })),
                        alt: None,
                    })],
                }),
            }))],
        };

        // Build the complete try-catch-finally statement
        let try_stmt = Stmt::Try(Box::new(TryStmt {
            span: DUMMY_SP,
            block: BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: vec![for_stmt],
            },
            handler: Some(catch_clause),
            finalizer: Some(finally_block),
        }));

        statements.push(try_stmt);
        statements
    }
}
