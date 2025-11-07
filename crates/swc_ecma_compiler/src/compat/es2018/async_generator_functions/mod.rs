//! ES2018: Async Generator Functions
//!
//! This plugin mainly does the following transformations:
//!
//! 1. transforms async generator functions (async function *name() {}) to
//!    generator functions and wraps them with `wrapAsyncGenerator` helper
//!    function.
//! 2. transforms `await expr` expression to `yield awaitAsyncGenerator(expr)`.
//! 3. transforms `yield * argument` expression to `yield
//!    asyncGeneratorDelegate(asyncIterator(argument))`.
//! 4. transforms `for await` statement to `for` statement, and inserts many
//!    code to handle async iteration.
//!
//! ## Example
//!
//! Input:
//! ```js
//! async function f() {
//!  for await (let x of y) {
//!    g(x);
//!  }
//! }
//! ```
//!
//! Output:
//! ```js
//! function f() {
//! return _f.apply(this, arguments);
//! }
//! function _f() {
//! _f = babelHelpers.asyncToGenerator(function* () {
//!     var _iteratorAbruptCompletion = false;
//!     var _didIteratorError = false;
//!     var _iteratorError;
//!     try {
//!     for (var _iterator = babelHelpers.asyncIterator(y), _step; _iteratorAbruptCompletion = !(_step = yield _iterator.next()).done; _iteratorAbruptCompletion = false) {
//!         let x = _step.value;
//!         {
//!         g(x);
//!         }
//!     }
//!     } catch (err) {
//!     _didIteratorError = true;
//!     _iteratorError = err;
//!     } finally {
//!     try {
//!         if (_iteratorAbruptCompletion && _iterator.return != null) {
//!         yield _iterator.return();
//!         }
//!     } finally {
//!         if (_didIteratorError) {
//!         throw _iteratorError;
//!         }
//!     }
//!     }
//! });
//! return _f.apply(this, arguments);
//! }
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-async-generator-functions](https://babel.dev/docs/babel-plugin-transform-async-generator-functions).
//!
//! Reference:
//! * Babel docs: <https://babeljs.io/docs/en/babel-plugin-transform-async-generator-functions>
//! * Babel implementation: <https://github.com/babel/babel/blob/v7.26.2/packages/babel-plugin-transform-async-generator-functions>
//! * Async Iteration TC39 proposal: <https://github.com/tc39/proposal-async-iteration>

#![allow(dead_code)]
mod for_await;

use std::mem;

use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::{
    common::helper_loader::Helper, context::TransformCtx, es2017::AsyncGeneratorExecutor,
};

/// Async generator functions transformer for ES2018.
///
/// Transforms async generator functions and related syntax (await in
/// generators, yield* in async generators, for-await loops) to ES5-compatible
/// code.
pub struct AsyncGeneratorFunctions<'ctx> {
    ctx: &'ctx TransformCtx,
    executor: AsyncGeneratorExecutor<'ctx>,
    /// Track the depth of async generator functions to determine context
    async_generator_depth: usize,
}

impl<'ctx> AsyncGeneratorFunctions<'ctx> {
    /// Create a new async generator functions transformer.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self {
            ctx,
            executor: AsyncGeneratorExecutor::new(Helper::WrapAsyncGenerator, ctx),
            async_generator_depth: 0,
        }
    }

    /// Transform `yield * argument` expression to `yield
    /// asyncGeneratorDelegate(asyncIterator(argument))`.
    fn transform_yield_expression(&mut self, expr: &mut YieldExpr) -> Option<Expr> {
        if !expr.delegate || self.async_generator_depth == 0 {
            return None;
        }

        expr.arg.as_mut().map(|argument| {
            let arg = mem::take(&mut **argument);
            // asyncIterator(argument)
            let arg = Expr::Call(CallExpr {
                span: DUMMY_SP,
                ctxt: Default::default(),
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
                    expr: Box::new(arg),
                }],
                type_args: None,
            });

            // asyncGeneratorDelegate(asyncIterator(argument))
            let arg = Expr::Call(CallExpr {
                span: DUMMY_SP,
                ctxt: Default::default(),
                callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(Expr::Ident(Ident::new(
                        "babelHelpers".into(),
                        DUMMY_SP,
                        Default::default(),
                    ))),
                    prop: MemberProp::Ident(IdentName::new(
                        Helper::AsyncGeneratorDelegate.name().into(),
                        DUMMY_SP,
                    )),
                }))),
                args: vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(arg),
                }],
                type_args: None,
            });

            Expr::Yield(YieldExpr {
                span: DUMMY_SP,
                arg: Some(Box::new(arg)),
                delegate: expr.delegate,
            })
        })
    }

    /// Transforms `await expr` expression to `yield awaitAsyncGenerator(expr)`.
    fn transform_await_expression(&mut self, expr: &mut AwaitExpr) -> Option<Expr> {
        if self.async_generator_depth == 0 {
            return None;
        }

        let arg = mem::take(&mut *expr.arg);
        // awaitAsyncGenerator(expr)
        let arg = Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(Ident::new(
                    "babelHelpers".into(),
                    DUMMY_SP,
                    Default::default(),
                ))),
                prop: MemberProp::Ident(IdentName::new(
                    Helper::AwaitAsyncGenerator.name().into(),
                    DUMMY_SP,
                )),
            }))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(arg),
            }],
            type_args: None,
        });

        Some(Expr::Yield(YieldExpr {
            span: DUMMY_SP,
            arg: Some(Box::new(arg)),
            delegate: false,
        }))
    }
}

impl VisitMutHook for AsyncGeneratorFunctions<'_> {
    fn enter_function(&mut self, func: &mut Function) {
        if func.is_async && func.is_generator {
            self.async_generator_depth += 1;
        }
    }

    fn exit_function(&mut self, func: &mut Function) {
        if func.is_async && func.is_generator {
            self.async_generator_depth = self.async_generator_depth.saturating_sub(1);

            // Transform async generator methods (in classes, objects, etc.)
            // Check if this is a method by seeing if it has a body
            if func.body.is_some() {
                self.executor.transform_function_for_method_definition(func);
            }
        }
    }

    fn exit_expr(&mut self, expr: &mut Expr) {
        let new_expr = match expr {
            Expr::Await(await_expr) => self.transform_await_expression(await_expr),
            Expr::Yield(yield_expr) => self.transform_yield_expression(yield_expr),
            Expr::Fn(func_expr) => {
                if func_expr.function.is_async && func_expr.function.is_generator {
                    Some(self.executor.transform_function_expression(func_expr))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(new_expr) = new_expr {
            *expr = new_expr;
        }
    }

    fn enter_stmt(&mut self, stmt: &mut Stmt) {
        // Handle for-await statements
        self.transform_statement(stmt);
    }

    fn exit_stmt(&mut self, stmt: &mut Stmt) {
        let should_transform = match stmt {
            Stmt::Decl(Decl::Fn(func)) => func.function.is_async && func.function.is_generator,
            _ => false,
        };

        if should_transform {
            if let Stmt::Decl(Decl::Fn(func)) = stmt {
                let new_statement = self.executor.transform_function_declaration(func);
                // For now, replace the statement directly
                // In a full implementation, we'd use statement injection
                *stmt = new_statement;
            }
        }
    }
}
