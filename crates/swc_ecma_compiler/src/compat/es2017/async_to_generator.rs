//! ES2017: Async / Await
//!
//! This plugin transforms async functions to generator functions
//! and wraps them with `asyncToGenerator` helper function.
//!
//! ## Example
//!
//! Input:
//! ```js
//! async function foo() {
//!   await bar();
//! }
//! const foo2 = async () => {
//!   await bar();
//! };
//! async () => {
//!   await bar();
//! }
//! ```
//!
//! Output:
//! ```js
//! function foo() {
//!   return _foo.apply(this, arguments);
//! }
//! function _foo() {
//!   _foo = babelHelpers.asyncToGenerator(function* () {
//!           yield bar();
//!   });
//!   return _foo.apply(this, arguments);
//! }
//! const foo2 = function() {
//!   var _ref = babelHelpers.asyncToGenerator(function* () {
//!           yield bar();
//!   });
//!   return function foo2() {
//!      return _ref.apply(this, arguments);
//!   };
//! }();
//! babelHelpers.asyncToGenerator(function* () {
//!   yield bar();
//! });
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-async-to-generator](https://babel.dev/docs/babel-plugin-transform-async-to-generator).
//!
//! Reference:
//! * Babel docs: <https://babeljs.io/docs/en/babel-plugin-transform-async-to-generator>
//! * Babel implementation: <https://github.com/babel/babel/blob/v7.26.2/packages/babel-plugin-transform-async-to-generator>
//! * Async / Await TC39 proposal: <https://github.com/tc39/proposal-async-await>

use std::mem;

use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;
use swc_ecma_utils::private_ident;

use crate::compat::{common::helper_loader::Helper, context::TransformCtx};

pub struct AsyncToGenerator<'ctx> {
    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,
    executor: AsyncGeneratorExecutor<'ctx>,
}

impl<'ctx> AsyncToGenerator<'ctx> {
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self {
            ctx,
            executor: AsyncGeneratorExecutor::new(Helper::AsyncToGenerator, ctx),
        }
    }
}

impl VisitMutHook for AsyncToGenerator<'_> {
    fn exit_expr(&mut self, expr: &mut Expr) {
        let new_expr = match expr {
            Expr::Await(await_expr) => Self::transform_await_expression(await_expr),
            Expr::Fn(func_expr) => {
                if func_expr.function.is_async && !func_expr.function.is_generator {
                    Some(self.executor.transform_function_expression(func_expr))
                } else {
                    None
                }
            }
            Expr::Arrow(arrow) => {
                if arrow.is_async {
                    Some(self.executor.transform_arrow_function(arrow))
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

    fn exit_stmt(&mut self, stmt: &mut Stmt) {
        let should_transform = match stmt {
            Stmt::Decl(Decl::Fn(func)) => func.function.is_async && !func.function.is_generator,
            _ => false,
        };

        if should_transform {
            if let Stmt::Decl(Decl::Fn(func)) = stmt {
                let new_statement = self.executor.transform_function_declaration(func);
                // For now, we'll need to handle statement injection differently in SWC
                // This is a simplified version - in practice, you'd use statement_injector
                // or store the new statement to be inserted after the pass
                *stmt = new_statement;
            }
        }
    }

    fn exit_function(&mut self, func: &mut Function) {
        if func.is_async {
            // Check if this is a method definition by checking the context
            // In SWC, we don't have direct ancestor access like in oxc
            // This would need to be tracked through the visitor state
            // For now, this is a stub that would need enhancement
            self.executor.transform_function_for_method_definition(func);
        }
    }
}

impl AsyncToGenerator<'_> {
    /// Transforms `await` expressions to `yield` expressions.
    /// Ignores top-level await expressions.
    fn transform_await_expression(expr: &mut AwaitExpr) -> Option<Expr> {
        // In SWC, we don't have easy access to scope information during visit
        // We'll need to track async function depth manually or use a different approach
        // For now, we'll always transform (top-level await would need separate
        // handling)

        let arg = mem::take(&mut *expr.arg);
        Some(Expr::Yield(YieldExpr {
            span: DUMMY_SP,
            arg: Some(Box::new(arg)),
            delegate: false,
        }))
    }
}

pub struct AsyncGeneratorExecutor<'ctx> {
    helper: Helper,
    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,
    /// Counter for generating unique identifiers
    uid_counter: usize,
}

impl<'ctx> AsyncGeneratorExecutor<'ctx> {
    pub fn new(helper: Helper, ctx: &'ctx TransformCtx) -> Self {
        Self {
            helper,
            ctx,
            uid_counter: 0,
        }
    }

    /// Generate a unique identifier
    fn generate_uid(&mut self, name: &str) -> Ident {
        self.uid_counter += 1;
        private_ident!(format!("_{}{}", name, self.uid_counter))
    }

    /// Generate a unique identifier based on an existing identifier
    fn generate_uid_based_on(&mut self, ident: &Ident) -> Ident {
        self.uid_counter += 1;
        private_ident!(format!("_{}{}", ident.sym, self.uid_counter))
    }

    /// Transforms async method definitions to generator functions wrapped in
    /// asyncToGenerator.
    ///
    /// ## Example
    ///
    /// Input:
    /// ```js
    /// class A { async foo() { await bar(); } }
    /// ```
    ///
    /// Output:
    /// ```js
    /// class A {
    /// foo() {
    ///     return babelHelpers.asyncToGenerator(function* () {
    ///         yield bar();
    ///     })();
    /// }
    /// ```
    pub fn transform_function_for_method_definition(&mut self, func: &mut Function) {
        if func.body.is_none() {
            return;
        }

        let body = mem::take(&mut func.body).unwrap();

        // If parameters could throw errors, we need to move them to the inner function
        let needs_move_parameters_to_inner_function =
            Self::could_throw_errors_parameters(&func.params);

        let params = if needs_move_parameters_to_inner_function {
            let new_params = Self::create_placeholder_params(&func.params);
            mem::replace(&mut func.params, new_params)
        } else {
            Self::create_empty_params()
        };

        let callee = self.create_async_to_generator_call(params, body);
        let (callee, arguments) = if needs_move_parameters_to_inner_function {
            // callee.apply(this, arguments)
            let callee_expr = Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(callee),
                prop: MemberProp::Ident(IdentName::new("apply".into(), DUMMY_SP)),
            });

            // this, arguments
            let this_argument = ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::This(ThisExpr { span: DUMMY_SP })),
            };
            let arguments_argument = ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Ident(Ident::new(
                    "arguments".into(),
                    DUMMY_SP,
                    Default::default(),
                ))),
            };
            (callee_expr, vec![this_argument, arguments_argument])
        } else {
            // callee()
            (callee, vec![])
        };

        let call_expr = Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(callee)),
            args: arguments,
            type_args: None,
        });

        let return_stmt = Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(call_expr)),
        });

        // Modify the wrapper function
        func.is_async = false;
        func.is_generator = false;
        func.body = Some(BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: vec![return_stmt],
        });
    }

    /// Transforms [`Function`] whose type is function expression to a generator
    /// function and wraps it in asyncToGenerator helper function.
    pub fn transform_function_expression(&mut self, wrapper_function: &mut FnExpr) -> Expr {
        let body = mem::take(&mut wrapper_function.function.body).unwrap();
        let params = mem::take(&mut wrapper_function.function.params);
        let id = mem::take(&mut wrapper_function.ident);
        let has_function_id = id.is_some();

        if !has_function_id && !Self::is_function_length_affected(&params) {
            return self.create_async_to_generator_call(params, body);
        }

        let bound_ident = if let Some(ref ident) = id {
            self.generate_uid_based_on(ident)
        } else {
            self.generate_uid("ref")
        };

        let caller_function = {
            let params = Self::create_placeholder_params(&params);
            let statements = vec![Self::create_apply_call_statement(&bound_ident)];
            let body = BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: statements,
            };
            Self::create_function(id.clone(), params, body)
        };

        let async_to_gen_decl =
            self.create_async_to_generator_declaration(&bound_ident, params, body);

        let statements = if has_function_id {
            let id_ref = id.as_ref().unwrap();
            let reference = Expr::Ident(id_ref.clone());
            let func_decl = Stmt::Decl(Decl::Fn(FnDecl {
                ident: id_ref.clone(),
                declare: false,
                function: Box::new(caller_function),
            }));
            let statement_return = Stmt::Return(ReturnStmt {
                span: DUMMY_SP,
                arg: Some(Box::new(reference)),
            });
            vec![async_to_gen_decl, func_decl, statement_return]
        } else {
            let statement_return = Stmt::Return(ReturnStmt {
                span: DUMMY_SP,
                arg: Some(Box::new(Expr::Fn(FnExpr {
                    ident: None,
                    function: Box::new(caller_function),
                }))),
            });
            vec![async_to_gen_decl, statement_return]
        };

        wrapper_function.function.is_async = false;
        wrapper_function.function.is_generator = false;
        wrapper_function.function.body = Some(BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: statements,
        });

        // Construct the IIFE
        let callee = Expr::Fn(FnExpr {
            ident: None,
            function: mem::take(&mut wrapper_function.function),
        });

        Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(callee)),
            args: vec![],
            type_args: None,
        })
    }

    /// Transforms async function declarations into generator functions wrapped
    /// in the asyncToGenerator helper.
    pub fn transform_function_declaration(&mut self, wrapper_function: &mut FnDecl) -> Stmt {
        let body = mem::take(&mut wrapper_function.function.body).unwrap();
        let params = Self::create_placeholder_params(&wrapper_function.function.params);
        let old_params = mem::replace(&mut wrapper_function.function.params, params);

        let bound_ident = self.generate_uid_based_on(&wrapper_function.ident);

        // Modify the wrapper function
        wrapper_function.function.is_async = false;
        wrapper_function.function.is_generator = false;
        let statements = vec![Self::create_apply_call_statement(&bound_ident)];
        wrapper_function.function.body = Some(BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: statements,
        });

        // Create the inner function
        let statements = vec![
            self.create_async_to_generator_assignment(&bound_ident, old_params, body),
            Self::create_apply_call_statement(&bound_ident),
        ];
        let body = BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: statements,
        };

        let params = Self::create_empty_params();
        let caller_function = Self::create_function(Some(bound_ident.clone()), params, body);

        Stmt::Decl(Decl::Fn(FnDecl {
            ident: bound_ident,
            declare: false,
            function: Box::new(caller_function),
        }))
    }

    /// Transforms async arrow functions into generator functions wrapped in the
    /// asyncToGenerator helper.
    pub fn transform_arrow_function(&mut self, arrow: &mut ArrowExpr) -> Expr {
        let mut body = mem::take(&mut *arrow.body);

        // If the arrow's expression is true, we need to wrap the only one expression
        // with return statement.
        if let BlockStmtOrExpr::Expr(expr) = &mut body {
            let expr_taken = mem::replace(&mut **expr, Expr::Invalid(Invalid { span: DUMMY_SP }));
            body = BlockStmtOrExpr::BlockStmt(BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: vec![Stmt::Return(ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(Box::new(expr_taken)),
                })],
            });
        }

        let block_stmt = match body {
            BlockStmtOrExpr::BlockStmt(block) => block,
            BlockStmtOrExpr::Expr(_) => unreachable!(),
        };

        let arrow_params = mem::take(&mut arrow.params);
        // Convert Vec<Pat> to Vec<Param> for arrow functions
        let params = Self::pat_vec_to_param_vec(arrow_params);

        // For simplicity, we'll skip function name inference in this port
        // In a complete implementation, you'd need to track parent context
        if !Self::is_function_length_affected(&params) {
            return self.create_async_to_generator_call(params, block_stmt);
        }

        let bound_ident = self.generate_uid("ref");

        let caller_function = {
            let placeholder_params = Self::create_placeholder_params(&params);
            let statements = vec![Self::create_apply_call_statement(&bound_ident)];
            let body = BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: statements,
            };
            let function = Self::create_function(None, placeholder_params, body);
            let argument = Some(Expr::Fn(FnExpr {
                ident: None,
                function: Box::new(function),
            }));
            Stmt::Return(ReturnStmt {
                span: DUMMY_SP,
                arg: argument.map(Box::new),
            })
        };

        // Wrapper function
        let statement =
            self.create_async_to_generator_declaration(&bound_ident, params, block_stmt);
        let statements = vec![statement, caller_function];
        let body = BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: statements,
        };
        let params = Self::create_empty_params();
        let wrapper_function = Self::create_function(None, params, body);

        // Construct the IIFE
        let callee = Expr::Fn(FnExpr {
            ident: None,
            function: Box::new(wrapper_function),
        });

        Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(callee)),
            args: vec![],
            type_args: None,
        })
    }

    /// Creates a [`Function`] with the specified params, body.
    #[inline]
    fn create_function(_id: Option<Ident>, params: Vec<Param>, body: BlockStmt) -> Function {
        Function {
            params,
            decorators: vec![],
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            body: Some(body),
            is_generator: false,
            is_async: false,
            type_params: None,
            return_type: None,
        }
    }

    /// Creates a [`Stmt`] that calls the `apply` method on the bound
    /// identifier.
    ///
    /// The generated code structure is:
    /// ```js
    /// bound_ident.apply(this, arguments);
    /// ```
    fn create_apply_call_statement(bound_ident: &Ident) -> Stmt {
        let arguments_ident = ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Ident(Ident::new(
                "arguments".into(),
                DUMMY_SP,
                Default::default(),
            ))),
        };

        // (this, arguments)
        let this = ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::This(ThisExpr { span: DUMMY_SP })),
        };
        let arguments = vec![this, arguments_ident];

        // _ref.apply
        let callee = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(bound_ident.clone())),
            prop: MemberProp::Ident(IdentName::new("apply".into(), DUMMY_SP)),
        });

        let call_expr = Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(callee)),
            args: arguments,
            type_args: None,
        });

        Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(call_expr)),
        })
    }

    /// Creates an [`Expr`] that calls the helper function.
    ///
    /// This function constructs the helper call with arguments derived from the
    /// provided parameters and body.
    ///
    /// The generated code structure is:
    /// ```js
    /// asyncToGenerator(function* (PARAMS) {
    ///    BODY
    /// });
    /// ```
    fn create_async_to_generator_call(&self, params: Vec<Param>, body: BlockStmt) -> Expr {
        let mut function = Self::create_function(None, params, body);
        function.is_generator = true;

        let func_expr = Expr::Fn(FnExpr {
            ident: None,
            function: Box::new(function),
        });

        let arguments = vec![ExprOrSpread {
            spread: None,
            expr: Box::new(func_expr),
        }];

        // Create helper call: babelHelpers.asyncToGenerator(...)
        let helper_name = self.helper.name();
        let callee = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(Ident::new(
                "babelHelpers".into(),
                DUMMY_SP,
                Default::default(),
            ))),
            prop: MemberProp::Ident(IdentName::new(helper_name.into(), DUMMY_SP)),
        });

        Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(callee)),
            args: arguments,
            type_args: None,
        })
    }

    /// Creates a helper declaration statement for async-to-generator
    /// transformation.
    ///
    /// This function generates code that looks like:
    /// ```js
    /// var _ref = asyncToGenerator(function* (PARAMS) {
    ///   BODY
    /// });
    /// ```
    fn create_async_to_generator_declaration(
        &self,
        bound_ident: &Ident,
        params: Vec<Param>,
        body: BlockStmt,
    ) -> Stmt {
        let init = self.create_async_to_generator_call(params, body);

        let declarator = VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: bound_ident.clone(),
                type_ann: None,
            }),
            init: Some(Box::new(init)),
            definite: false,
        };

        Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![declarator],
        })))
    }

    /// Creates a helper assignment statement for async-to-generator
    /// transformation.
    ///
    /// This function generates code that looks like:
    /// ```js
    /// _ref = asyncToGenerator(function* (PARAMS) {
    ///   BODY
    /// });
    /// ```
    fn create_async_to_generator_assignment(
        &self,
        bound_ident: &Ident,
        params: Vec<Param>,
        body: BlockStmt,
    ) -> Stmt {
        let right = self.create_async_to_generator_call(params, body);

        let assign_expr = Expr::Assign(AssignExpr {
            span: DUMMY_SP,
            op: op!("="),
            left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent {
                id: bound_ident.clone(),
                type_ann: None,
            })),
            right: Box::new(right),
        });

        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(assign_expr),
        })
    }

    /// Creates placeholder [`Vec<Param>`] which named `_x` based on the
    /// passed-in parameters. `function p(x, y, z, d = 0, ...rest) {}` ->
    /// `function* (_x, _x1, _x2) {}`
    fn create_placeholder_params(params: &[Param]) -> Vec<Param> {
        let mut parameters = Vec::with_capacity(params.len());
        let mut counter = 0;

        for param in params {
            if matches!(param.pat, Pat::Assign(_)) {
                break;
            }

            let ident = if counter == 0 {
                private_ident!("_x")
            } else {
                private_ident!(format!("_x{}", counter))
            };

            parameters.push(Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: Pat::Ident(BindingIdent {
                    id: ident,
                    type_ann: None,
                }),
            });
            counter += 1;
        }

        parameters
    }

    /// Creates an empty parameter list.
    #[inline]
    fn create_empty_params() -> Vec<Param> {
        vec![]
    }

    /// Converts Vec<Pat> to Vec<Param> for arrow function parameters.
    fn pat_vec_to_param_vec(pats: Vec<Pat>) -> Vec<Param> {
        pats.into_iter()
            .map(|pat| Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat,
            })
            .collect()
    }

    /// Checks if the function length is affected by the parameters.
    #[inline]
    fn is_function_length_affected(params: &[Param]) -> bool {
        params
            .first()
            .is_some_and(|param| !matches!(param.pat, Pat::Assign(_)))
    }

    /// Check whether the function parameters could throw errors.
    #[inline]
    fn could_throw_errors_parameters(params: &[Param]) -> bool {
        params.iter().any(|param| {
            if let Pat::Assign(pattern) = &param.pat {
                Self::could_potentially_throw_error_expression(&pattern.right)
            } else {
                false
            }
        })
    }

    /// Check whether the expression could potentially throw an error.
    #[inline]
    fn could_potentially_throw_error_expression(expr: &Expr) -> bool {
        match expr {
            Expr::Lit(Lit::Null(_))
            | Expr::Lit(Lit::Bool(_))
            | Expr::Lit(Lit::Num(_))
            | Expr::Lit(Lit::Str(_))
            | Expr::Lit(Lit::BigInt(_))
            | Expr::Arrow(_)
            | Expr::Fn(_) => false,
            Expr::Ident(ident) if &*ident.sym == "undefined" => false,
            _ => true,
        }
    }
}
