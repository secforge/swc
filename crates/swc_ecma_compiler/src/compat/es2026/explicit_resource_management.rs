//! Proposal: Explicit Resource Management
//!
//! This plugin transforms explicit resource management syntax into a series of
//! try-catch-finally blocks.
//!
//! ## Example
//!
//! Input:
//! ```js
//! for await (using x of y) {
//!     doSomethingWith(x);
//! }
//! ```
//!
//! Output:
//! ```js
//! for await (const _x of y)
//! try {
//!     var _usingCtx = babelHelpers.usingCtx();
//!     const x = _usingCtx.u(_x);
//!     doSomethingWith(x);
//! } catch (_) {
//!     _usingCtx.e = _;
//! } finally {
//!     _usingCtx.d();
//! }
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-explicit-resource-management](https://babeljs.io/docs/babel-plugin-transform-explicit-resource-management).
//!
//! ## References:
//! * Babel plugin implementation: <https://github.com/babel/babel/blob/v7.26.9/packages/babel-plugin-transform-explicit-resource-management>
//! * Explicit Resource Management TC39 proposal: <https://github.com/tc39/proposal-explicit-resource-management>

#![allow(dead_code)]
use std::{cell::RefCell, mem};

use rustc_hash::FxHashMap;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;
use swc_ecma_utils::private_ident;

use crate::compat::TransformCtx;

pub struct ExplicitResourceManagement<'ctx> {
    ctx: &'ctx TransformCtx,

    /// Map of top-level using declarations for program-level handling
    top_level_using: RefCell<FxHashMap<usize, bool>>,

    /// Counter for generating unique identifiers
    uid_counter: RefCell<usize>,
}

impl<'ctx> ExplicitResourceManagement<'ctx> {
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self {
            ctx,
            top_level_using: RefCell::new(FxHashMap::default()),
            uid_counter: RefCell::new(0),
        }
    }

    /// Generate a unique identifier with the given name prefix
    fn generate_uid(&self, name: &str) -> Ident {
        let mut counter = self.uid_counter.borrow_mut();
        *counter += 1;
        private_ident!(format!("_{}{}", name, *counter))
    }

    /// Generate a unique identifier based on an existing identifier
    fn generate_uid_based_on(&self, ident: &Ident) -> Ident {
        let mut counter = self.uid_counter.borrow_mut();
        *counter += 1;
        private_ident!(format!("_{}{}", ident.sym, *counter))
    }

    /// Create a using context helper call: `babelHelpers.usingCtx()`
    fn create_using_ctx_helper_call(&self) -> Box<Expr> {
        // TODO: Integrate with SWC's helper loading system
        // For now, use babelHelpers.usingCtx()
        Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(Ident::new(
                    "babelHelpers".into(),
                    DUMMY_SP,
                    SyntaxContext::empty(),
                ))),
                prop: MemberProp::Ident(IdentName::new("usingCtx".into(), DUMMY_SP)),
            }))),
            args: vec![],
            type_args: None,
        }))
    }

    /// Create try-catch-finally statement for using declaration
    ///
    /// ```js
    /// try {
    ///   // body
    /// } catch (_) {
    ///   _usingCtx.e = _;
    /// } finally {
    ///   _usingCtx.d(); // or: await _usingCtx.d();
    /// }
    /// ```
    fn create_try_stmt(&self, body: BlockStmt, using_ctx: &Ident, needs_await: bool) -> Stmt {
        let catch = self.create_catch_clause(using_ctx);
        let finally = self.create_finally_block(using_ctx, needs_await);

        Stmt::Try(Box::new(TryStmt {
            span: DUMMY_SP,
            block: body,
            handler: Some(catch),
            finalizer: Some(finally),
        }))
    }

    /// Create catch clause: `catch (_) { _usingCtx.e = _; }`
    fn create_catch_clause(&self, using_ctx: &Ident) -> CatchClause {
        let error_ident = private_ident!("_");

        CatchClause {
            span: DUMMY_SP,
            param: Some(Pat::Ident(error_ident.clone().into())),
            body: BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: vec![Stmt::Expr(ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Assign(AssignExpr {
                        span: DUMMY_SP,
                        op: op!("="),
                        left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(Expr::Ident(using_ctx.clone())),
                            prop: MemberProp::Ident(IdentName::new("e".into(), DUMMY_SP)),
                        })),
                        right: Box::new(Expr::Ident(error_ident)),
                    })),
                })],
            },
        }
    }

    /// Create finally block: `finally { _usingCtx.d(); }` or `finally { await
    /// _usingCtx.d(); }`
    fn create_finally_block(&self, using_ctx: &Ident, needs_await: bool) -> BlockStmt {
        let call_expr = Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(using_ctx.clone())),
                prop: MemberProp::Ident(IdentName::new("d".into(), DUMMY_SP)),
            }))),
            args: vec![],
            type_args: None,
        });

        let expr = if needs_await {
            Expr::Await(AwaitExpr {
                span: DUMMY_SP,
                arg: Box::new(call_expr),
            })
        } else {
            call_expr
        };

        BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: vec![Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(expr),
            })],
        }
    }

    /// Transform using declarations in a list of statements
    ///
    /// Returns `Some((new_stmts, needs_await, using_ctx))` if transformation
    /// occurred, `None` otherwise.
    fn transform_statements(&self, stmts: &mut Vec<Stmt>) -> Option<(Vec<Stmt>, bool, Ident)> {
        let mut needs_await = false;
        let mut using_ctx: Option<Ident> = None;
        let mut found_using = false;

        // First pass: detect using declarations and transform them
        for stmt in stmts.iter_mut() {
            if let Stmt::Decl(Decl::Using(using_decl)) = stmt {
                found_using = true;
                needs_await = needs_await || using_decl.is_await;

                // Create using context identifier if not already created
                if using_ctx.is_none() {
                    using_ctx = Some(self.generate_uid("usingCtx"));
                }

                // Transform: `using foo = bar;` -> `const foo = _usingCtx.u(bar);`
                // Transform: `await using foo = bar;` -> `const foo = _usingCtx.a(bar);`
                let const_decl = self.transform_using_decl(using_decl, using_ctx.as_ref().unwrap());
                *stmt = Stmt::Decl(Decl::Var(const_decl));
            }
        }

        if !found_using {
            return None;
        }

        let using_ctx = using_ctx.unwrap();
        let mut new_stmts = mem::take(stmts);

        // Insert helper at the beginning: `var _usingCtx = babelHelpers.usingCtx();`
        let helper_stmt = Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(using_ctx.clone().into()),
                init: Some(self.create_using_ctx_helper_call()),
                definite: false,
            }],
        })));

        new_stmts.insert(0, helper_stmt);

        Some((new_stmts, needs_await, using_ctx))
    }

    /// Transform a UsingDecl into a VarDecl with usingCtx helper call
    fn transform_using_decl(&self, using_decl: &UsingDecl, using_ctx: &Ident) -> Box<VarDecl> {
        let method_name = if using_decl.is_await { "a" } else { "u" };

        let decls = using_decl
            .decls
            .iter()
            .map(|decl| VarDeclarator {
                span: decl.span,
                name: decl.name.clone(),
                init: decl.init.as_ref().map(|init| {
                    Box::new(Expr::Call(CallExpr {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(Expr::Ident(using_ctx.clone())),
                            prop: MemberProp::Ident(IdentName::new(method_name.into(), DUMMY_SP)),
                        }))),
                        args: vec![ExprOrSpread {
                            spread: None,
                            expr: init.clone(),
                        }],
                        type_args: None,
                    }))
                }),
                definite: false,
            })
            .collect();

        Box::new(VarDecl {
            span: using_decl.span,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Const,
            declare: false,
            decls,
        })
    }

    /// Transform class declaration to variable declaration
    fn transform_class_decl(&self, class_decl: &mut ClassDecl) -> Stmt {
        let id = mem::replace(
            &mut class_decl.ident,
            Ident::new("__tmp".into(), DUMMY_SP, SyntaxContext::empty()),
        );

        let class = mem::replace(
            &mut class_decl.class,
            Box::new(Class {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                decorators: vec![],
                body: vec![],
                super_class: None,
                is_abstract: false,
                type_params: None,
                super_type_params: None,
                implements: vec![],
            }),
        );

        // Convert class to expression
        let class_expr = Expr::Class(ClassExpr { ident: None, class });

        Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(id.into()),
                init: Some(Box::new(class_expr)),
                definite: false,
            }],
        })))
    }
}

impl VisitMutHook for ExplicitResourceManagement<'_> {
    /// Transform `for (using ... of ...)`, ready for other transformations to
    /// handle.
    ///
    /// * `for (using x of y) {}` -> `for (const _x of y) { using x = _x; }`
    /// * `for await (using x of y) {}` -> `for await (const _x of y) { await
    ///   using x = _x; }`
    fn enter_for_of_stmt(&mut self, for_of_stmt: &mut ForOfStmt) {
        // Check if the left side is a using declaration
        if let ForHead::UsingDecl(using_decl) = &mut for_of_stmt.left {
            let is_await = using_decl.is_await;

            // Get the first (and should be only) declarator
            if let Some(declarator) = using_decl.decls.first() {
                // Create a temp identifier
                let temp_id = if let Pat::Ident(ident) = &declarator.name {
                    self.generate_uid_based_on(&ident.id)
                } else {
                    self.generate_uid("x")
                };

                let original_pattern = declarator.name.clone();

                // Replace with: `for (const _x of y)`
                for_of_stmt.left = ForHead::VarDecl(Box::new(VarDecl {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    kind: VarDeclKind::Const,
                    declare: false,
                    decls: vec![VarDeclarator {
                        span: DUMMY_SP,
                        name: Pat::Ident(temp_id.clone().into()),
                        init: None,
                        definite: false,
                    }],
                }));

                // Create: `using x = _x;` or `await using x = _x;`
                let using_stmt = Stmt::Decl(Decl::Using(Box::new(UsingDecl {
                    span: DUMMY_SP,
                    is_await,
                    decls: vec![VarDeclarator {
                        span: DUMMY_SP,
                        name: original_pattern,
                        init: Some(Box::new(Expr::Ident(temp_id))),
                        definite: false,
                    }],
                })));

                // Prepend to the body
                match for_of_stmt.body.as_mut() {
                    Stmt::Block(block) => {
                        block.stmts.insert(0, using_stmt);
                    }
                    _ => {
                        // Wrap non-block body in a block
                        let old_body = mem::replace(
                            &mut for_of_stmt.body,
                            Box::new(Stmt::Empty(EmptyStmt { span: DUMMY_SP })),
                        );
                        for_of_stmt.body = Box::new(Stmt::Block(BlockStmt {
                            span: DUMMY_SP,
                            ctxt: SyntaxContext::empty(),
                            stmts: vec![using_stmt, *old_body],
                        }));
                    }
                }
            }
        }
    }

    /// Transform class static block.
    ///
    /// ```js
    /// class C { static { using x = y(); } }
    /// ```
    /// ->
    /// ```js
    /// class C {
    ///   static {
    ///     try {
    ///       var _usingCtx = babelHelpers.usingCtx();
    ///       const x = _usingCtx.u(y());
    ///     } catch (_) {
    ///       _usingCtx.e = _;
    ///     } finally {
    ///       _usingCtx.d();
    ///     }
    ///   }
    /// }
    /// ```
    fn exit_static_block(&mut self, block: &mut StaticBlock) {
        if let Some((new_stmts, needs_await, using_ctx)) =
            self.transform_statements(&mut block.body.stmts)
        {
            block.body.stmts = vec![self.create_try_stmt(
                BlockStmt {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    stmts: new_stmts,
                },
                &using_ctx,
                needs_await,
            )];
        }
    }

    /// Transform function body.
    ///
    /// ```js
    /// function f() {
    ///   using x = y();
    /// }
    /// ```
    /// ->
    /// ```js
    /// function f() {
    ///   try {
    ///     var _usingCtx = babelHelpers.usingCtx();
    ///     const x = _usingCtx.u(y());
    ///   } catch (_) {
    ///     _usingCtx.e = _;
    ///   } finally {
    ///     _usingCtx.d();
    ///   }
    /// }
    /// ```
    fn enter_function(&mut self, func: &mut Function) {
        if let Some(body) = &mut func.body {
            if let Some((new_stmts, needs_await, using_ctx)) =
                self.transform_statements(&mut body.stmts)
            {
                body.stmts = vec![self.create_try_stmt(
                    BlockStmt {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        stmts: new_stmts,
                    },
                    &using_ctx,
                    needs_await,
                )];
            }
        }
    }

    /// Transform block statement or switch statement.
    ///
    /// For block statements:
    /// ```js
    /// { using x = y(); }
    /// ```
    /// ->
    /// ```js
    /// try {
    ///   var _usingCtx = babelHelpers.usingCtx();
    ///   const x = _usingCtx.u(y());
    /// } catch (_) {
    ///   _usingCtx.e = _;
    /// } finally {
    ///   _usingCtx.d();
    /// }
    /// ```
    fn enter_stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Block(block) => {
                if let Some((new_stmts, needs_await, using_ctx)) =
                    self.transform_statements(&mut block.stmts)
                {
                    *stmt = self.create_try_stmt(
                        BlockStmt {
                            span: DUMMY_SP,
                            ctxt: SyntaxContext::empty(),
                            stmts: new_stmts,
                        },
                        &using_ctx,
                        needs_await,
                    );
                }
            }
            Stmt::Switch(switch_stmt) => {
                // Transform switch statements with using declarations
                if let Some((using_ctx, needs_await)) = self.transform_switch_statement(switch_stmt)
                {
                    // Need to wrap the entire switch in try-catch-finally
                    let switch_taken = mem::replace(
                        switch_stmt,
                        SwitchStmt {
                            span: DUMMY_SP,
                            discriminant: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                            cases: vec![],
                        },
                    );

                    // Create helper variable
                    let helper_stmt = Stmt::Decl(Decl::Var(Box::new(VarDecl {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        kind: VarDeclKind::Var,
                        declare: false,
                        decls: vec![VarDeclarator {
                            span: DUMMY_SP,
                            name: Pat::Ident(using_ctx.clone().into()),
                            init: Some(self.create_using_ctx_helper_call()),
                            definite: false,
                        }],
                    })));

                    // Replace the statement with a try statement
                    *stmt = self.create_try_stmt(
                        BlockStmt {
                            span: DUMMY_SP,
                            ctxt: SyntaxContext::empty(),
                            stmts: vec![helper_stmt, Stmt::Switch(switch_taken)],
                        },
                        &using_ctx,
                        needs_await,
                    );
                }
            }
            _ => {}
        }
    }

    /// Transform try statement.
    ///
    /// ```js
    /// try {
    ///   using x = y();
    /// } catch (err) { }
    /// ```
    /// ->
    /// ```js
    /// try {
    ///   try {
    ///     var _usingCtx = babelHelpers.usingCtx();
    ///     const x = _usingCtx.u(y());
    ///   } catch (_) {
    ///     _usingCtx.e = _;
    ///   } finally {
    ///     _usingCtx.d();
    ///   }
    /// } catch (err) { }
    /// ```
    fn enter_try_stmt(&mut self, node: &mut TryStmt) {
        if let Some((new_stmts, needs_await, using_ctx)) =
            self.transform_statements(&mut node.block.stmts)
        {
            node.block.stmts = vec![self.create_try_stmt(
                BlockStmt {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    stmts: new_stmts,
                },
                &using_ctx,
                needs_await,
            )];
        }
    }

    /// Move any top level `using` declarations within a block statement,
    /// allowing other transformations to handle them.
    fn enter_module(&mut self, module: &mut Module) {
        self.handle_program(&mut module.body);
    }

    fn enter_script(&mut self, script: &mut Script) {
        // For scripts, we need to handle using declarations at the top level
        // This is more complex than modules because scripts don't have import/export
        // For now, we'll apply a simpler transformation

        let has_using = script
            .body
            .iter()
            .any(|item| matches!(item, Stmt::Decl(Decl::Using(_))));

        if !has_using {
            return;
        }

        // Simple approach: wrap all statements in a block with try-catch-finally
        let mut stmts = mem::take(&mut script.body);
        if let Some((new_stmts, needs_await, using_ctx)) = self.transform_statements(&mut stmts) {
            script.body = vec![self.create_try_stmt(
                BlockStmt {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    stmts: new_stmts,
                },
                &using_ctx,
                needs_await,
            )];
        } else {
            script.body = stmts;
        }
    }
}

impl ExplicitResourceManagement<'_> {
    /// Transform switch statement with using declarations.
    ///
    /// Input:
    /// ```js
    /// switch (0) {
    ///   case 1:
    ///     using foo = bar;
    ///     doSomethingWithFoo(foo)
    ///   case 2:
    ///     throw new Error('oops')
    /// }
    /// ```
    /// Output:
    /// ```js
    /// try {
    ///   var _usingCtx = babelHelpers.usingCtx();
    ///   switch (0) {
    ///     case 1:
    ///       const foo = _usingCtx.u(bar);
    ///       doSomethingWithFoo(foo);
    ///     case 2:
    ///       throw new Error('oops');
    ///   }
    /// } catch (_) {
    ///   _usingCtx.e = _;
    /// } finally {
    ///   _usingCtx.d();
    /// }
    /// ```
    fn transform_switch_statement(&self, switch_stmt: &mut SwitchStmt) -> Option<(Ident, bool)> {
        let mut using_ctx: Option<Ident> = None;
        let mut needs_await = false;

        // Transform using declarations in switch cases
        for case in &mut switch_stmt.cases {
            for stmt in &mut case.cons {
                if let Stmt::Decl(Decl::Using(using_decl)) = stmt {
                    needs_await = needs_await || using_decl.is_await;

                    if using_ctx.is_none() {
                        using_ctx = Some(self.generate_uid("usingCtx"));
                    }

                    // Transform to const declaration with helper call
                    let const_decl =
                        self.transform_using_decl(using_decl, using_ctx.as_ref().unwrap());
                    *stmt = Stmt::Decl(Decl::Var(const_decl));
                }
            }
        }

        using_ctx.map(|ctx| (ctx, needs_await))
    }

    /// Move top-level using declarations into a block for module-level
    /// handling.
    ///
    /// This handles the complex case of top-level using declarations with
    /// imports and exports.
    fn handle_program(&self, items: &mut Vec<ModuleItem>) {
        self.top_level_using.borrow_mut().clear();

        // Check if there are any using declarations at the top level
        let has_using = items.iter().any(|item| match item {
            ModuleItem::Stmt(Stmt::Decl(Decl::Using(_))) => true,
            // Also check in export declarations
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                matches!(export_decl.decl, Decl::Using(_))
            }
            _ => false,
        });

        if !has_using {
            return;
        }

        // For now, we'll implement a simplified version that wraps all statements
        // A full implementation would need to carefully handle:
        // 1. Hoisting imports
        // 2. Preserving export declarations
        // 3. Converting class/function declarations to var declarations
        // 4. Handling default exports

        let mut new_items = Vec::new();
        let mut inner_stmts = Vec::new();

        for item in mem::take(items) {
            match item {
                // Keep imports at the top
                ModuleItem::ModuleDecl(ModuleDecl::Import(_)) => {
                    new_items.push(item);
                }
                // Keep function declarations at the top (they're hoisted anyway)
                ModuleItem::Stmt(Stmt::Decl(Decl::Fn(_))) => {
                    new_items.push(item);
                }
                // Everything else goes into the inner block
                _ => {
                    inner_stmts.push(match item {
                        ModuleItem::Stmt(stmt) => stmt,
                        ModuleItem::ModuleDecl(decl) => {
                            // Convert module declarations to statements where possible
                            // This is simplified; full implementation would handle exports properly
                            match decl {
                                ModuleDecl::ExportDecl(export_decl) => Stmt::Decl(export_decl.decl),
                                ModuleDecl::ExportDefaultDecl(export_default) => {
                                    match export_default.decl {
                                        DefaultDecl::Class(class_expr) => {
                                            let ident = class_expr.ident.unwrap_or_else(|| {
                                                Ident::new(
                                                    "_default".into(),
                                                    DUMMY_SP,
                                                    SyntaxContext::empty(),
                                                )
                                            });
                                            Stmt::Decl(Decl::Class(ClassDecl {
                                                ident,
                                                declare: false,
                                                class: class_expr.class,
                                            }))
                                        }
                                        DefaultDecl::Fn(fn_expr) => {
                                            let ident = fn_expr.ident.unwrap_or_else(|| {
                                                Ident::new(
                                                    "_default".into(),
                                                    DUMMY_SP,
                                                    SyntaxContext::empty(),
                                                )
                                            });
                                            Stmt::Decl(Decl::Fn(FnDecl {
                                                ident,
                                                declare: false,
                                                function: fn_expr.function,
                                            }))
                                        }
                                        DefaultDecl::TsInterfaceDecl(_) => {
                                            // Skip TypeScript-only declarations
                                            continue;
                                        }
                                    }
                                }
                                _ => {
                                    // For other module declarations, we can't easily convert
                                    // Keep them at the top level
                                    new_items.push(ModuleItem::ModuleDecl(decl));
                                    continue;
                                }
                            }
                        }
                    });
                }
            }
        }

        if inner_stmts.is_empty() {
            *items = new_items;
            return;
        }

        // Wrap inner statements in a block
        new_items.push(ModuleItem::Stmt(Stmt::Block(BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts: inner_stmts,
        })));

        *items = new_items;
    }
}
