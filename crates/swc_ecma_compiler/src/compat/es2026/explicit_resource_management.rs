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

use std::mem;

use rustc_hash::FxHashMap;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_utils::private_ident;
use swc_ecma_visit::{VisitMut, VisitMutWith};

pub struct ExplicitResourceManagement {
    /// Map of top-level using declarations (address -> is await-using)
    top_level_using: FxHashMap<usize, bool>,
    /// Counter for generating unique identifiers
    uid_counter: usize,
}

impl ExplicitResourceManagement {
    pub fn new() -> Self {
        Self {
            top_level_using: FxHashMap::default(),
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

    /// Check if a variable declaration is a using/await-using declaration
    fn is_using_declaration(decl: &VarDecl) -> bool {
        // In SWC, we check the declare field or custom attributes
        // For now, we'll need to track this through other means
        // This is a limitation we need to work around
        false
    }

    /// Create a using context helper call: `babelHelpers.usingCtx()`
    fn create_using_ctx_helper_call(&self) -> Expr {
        // babelHelpers.usingCtx()
        Expr::Call(CallExpr {
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
        })
    }

    /// Create try-catch-finally statement for using declaration
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
    fn transform_statements(&mut self, stmts: &mut Vec<Stmt>) -> Option<(Vec<Stmt>, bool, Ident)> {
        let needs_await = false;
        let using_ctx: Option<Ident> = None;
        let found_using = false;

        for stmt in stmts.iter_mut() {
            if let Stmt::Decl(Decl::Var(var_decl)) = stmt {
                // Check if this is a using/await-using declaration
                // In SWC, we need to track this through metadata or comments
                // For now, we'll skip this as it requires additional metadata
                // TODO: Implement proper using declaration detection
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
                init: Some(Box::new(self.create_using_ctx_helper_call())),
                definite: false,
            }],
        })));

        new_stmts.insert(0, helper_stmt);

        Some((new_stmts, needs_await, using_ctx))
    }

    /// Transform class declaration to variable declaration
    fn transform_class_decl(class_decl_id: &Ident, class: &mut Class) -> Stmt {
        // Convert class to expression
        let class_expr = Expr::Class(ClassExpr {
            ident: None,
            class: Box::new(class.clone()),
        });

        Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(class_decl_id.clone().into()),
                init: Some(Box::new(class_expr)),
                definite: false,
            }],
        })))
    }

    /// Move top-level using declarations into a block
    fn handle_program(&mut self, stmts: &mut Vec<ModuleItem>) {
        self.top_level_using.clear();

        // Check if there are any using declarations
        let has_using = stmts.iter().any(|item| {
            if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = item {
                // TODO: Check for using/await-using
                false
            } else {
                false
            }
        });

        if !has_using {
            return;
        }

        // TODO: Implement full program transformation
        // This requires complex handling of imports, exports, and hoisting
        // Similar to the oxc implementation's enter_program method
    }
}

impl VisitMut for ExplicitResourceManagement {
    /// Transform `for (using ... of ...)` statements
    fn visit_mut_for_of_stmt(&mut self, n: &mut ForOfStmt) {
        // Check if this is a using declaration
        if let ForHead::VarDecl(var_decl) = &mut n.left {
            // TODO: Check if var_decl is using/await-using
            // Transform: `for (using x of y) {}` -> `for (const _x of y) {
            // using x = _x; }`
        }

        n.visit_mut_children_with(self);
    }

    /// Transform block statements
    fn visit_mut_block_stmt(&mut self, n: &mut BlockStmt) {
        n.visit_mut_children_with(self);

        if let Some((new_stmts, needs_await, using_ctx)) = self.transform_statements(&mut n.stmts) {
            // Wrap in try-catch-finally
            let inner_block = BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: new_stmts,
            };

            n.stmts = vec![self.create_try_stmt(inner_block, &using_ctx, needs_await)];
        }
    }

    /// Transform try statements
    fn visit_mut_try_stmt(&mut self, n: &mut TryStmt) {
        n.visit_mut_children_with(self);

        if let Some((new_stmts, needs_await, using_ctx)) =
            self.transform_statements(&mut n.block.stmts)
        {
            // Wrap the block content in try-catch-finally
            let inner_block = BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: new_stmts,
            };

            n.block.stmts = vec![self.create_try_stmt(inner_block, &using_ctx, needs_await)];
        }
    }

    /// Transform function body
    fn visit_mut_function(&mut self, n: &mut Function) {
        n.visit_mut_children_with(self);

        if let Some(body) = &mut n.body {
            if let Some((new_stmts, needs_await, using_ctx)) =
                self.transform_statements(&mut body.stmts)
            {
                let inner_block = BlockStmt {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    stmts: new_stmts,
                };

                body.stmts = vec![self.create_try_stmt(inner_block, &using_ctx, needs_await)];
            }
        }
    }

    /// Transform arrow function body
    fn visit_mut_arrow_expr(&mut self, n: &mut ArrowExpr) {
        n.visit_mut_children_with(self);

        if let BlockStmtOrExpr::BlockStmt(body) = n.body.as_mut() {
            if let Some((new_stmts, needs_await, using_ctx)) =
                self.transform_statements(&mut body.stmts)
            {
                let inner_block = BlockStmt {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    stmts: new_stmts,
                };

                body.stmts = vec![self.create_try_stmt(inner_block, &using_ctx, needs_await)];
            }
        }
    }

    /// Transform class static blocks
    fn visit_mut_static_block(&mut self, n: &mut StaticBlock) {
        n.visit_mut_children_with(self);

        if let Some((new_stmts, needs_await, using_ctx)) =
            self.transform_statements(&mut n.body.stmts)
        {
            let inner_block = BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: new_stmts,
            };

            n.body.stmts = vec![self.create_try_stmt(inner_block, &using_ctx, needs_await)];
        }
    }

    /// Transform switch statements
    fn visit_mut_switch_stmt(&mut self, n: &mut SwitchStmt) {
        n.visit_mut_children_with(self);

        let using_ctx: Option<Ident> = None;
        let needs_await = false;

        for case in &mut n.cases {
            for stmt in &mut case.cons {
                if let Stmt::Decl(Decl::Var(var_decl)) = stmt {
                    // TODO: Check if this is using/await-using
                    // Transform: `using foo = bar;` -> `const foo =
                    // _usingCtx.u(bar);`
                }
            }
        }

        if let Some(using_ctx) = using_ctx {
            // Wrap the switch in try-catch-finally
            let switch_stmt = mem::replace(
                n,
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
                    init: Some(Box::new(self.create_using_ctx_helper_call())),
                    definite: false,
                }],
            })));

            // This needs to be restructured - placeholder for now
            // TODO: Complete switch statement transformation
        }
    }

    /// Transform module
    fn visit_mut_module(&mut self, n: &mut Module) {
        self.handle_program(&mut n.body);
        n.visit_mut_children_with(self);
    }

    /// Transform script
    fn visit_mut_script(&mut self, n: &mut Script) {
        // TODO: Handle script-level using declarations
        n.visit_mut_children_with(self);
    }
}
