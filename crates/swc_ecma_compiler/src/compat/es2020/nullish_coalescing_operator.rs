//! ES2020: Nullish Coalescing Operator
//!
//! This plugin transforms nullish coalescing operators (`??`) to a series of
//! ternary expressions.
//!
//! > This plugin is included in `preset-env`, in ES2020
//!
//! ## Example
//!
//! Input:
//! ```js
//! var foo = object.foo ?? "default";
//! ```
//!
//! Output:
//! ```js
//! var _object$foo;
//! var foo =
//! (_object$foo = object.foo) !== null && _object$foo !== void 0
//!   ? _object$foo
//!   : "default";
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-nullish-coalescing-operator](https://babeljs.io/docs/babel-plugin-transform-nullish-coalescing-operator).
//!
//! ## References:
//! * Babel plugin implementation: <https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-transform-nullish-coalescing-operator>
//! * Nullish coalescing TC39 proposal: <https://github.com/tc39-transfer/proposal-nullish-coalescing>

#![allow(dead_code)]
use std::mem;

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::{common::var_declarations::VarDeclarationsStore, context::TransformCtx};

/// Nullish coalescing operator transformer.
///
/// Transforms `??` operator to conditional expressions with null/undefined
/// checks.
pub struct NullishCoalescingOperator<'ctx> {
    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,
    var_declarations: VarDeclarationsStore,
    uid_counter: usize,
}

impl<'ctx> NullishCoalescingOperator<'ctx> {
    /// Create a new NullishCoalescingOperator transformer.
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self {
            ctx,
            var_declarations: VarDeclarationsStore::new(),
            uid_counter: 0,
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

    /// Transform a logical expression with nullish coalescing operator.
    fn transform_logical_expression(&mut self, left: Box<Expr>, right: Expr) -> Expr {
        // Check if left is a simple identifier or this expression
        match left.as_ref() {
            Expr::This(_) => {
                // `this ?? right` -> `this !== null && this !== void 0 ? this : right`
                return self.create_conditional_expression(
                    *left.clone(),
                    *left.clone(),
                    *left,
                    right,
                );
            }
            Expr::Ident(ident) => {
                // For simple identifiers, we can reuse them
                // `foo ?? right` -> `foo !== null && foo !== void 0 ? foo : right`
                let ident_expr = Expr::Ident(ident.clone());
                return self.create_conditional_expression(
                    ident_expr.clone(),
                    ident_expr.clone(),
                    ident_expr,
                    right,
                );
            }
            _ => {}
        }

        // For complex expressions, create a temp variable
        // `foo.bar ?? right` -> `(_foo$bar = foo.bar) !== null && _foo$bar !== void 0 ?
        // _foo$bar : right`
        let binding = self.generate_uid("temp");
        self.var_declarations.insert_var(&binding, None);

        let assignment = Expr::Assign(AssignExpr {
            span: DUMMY_SP,
            op: op!("="),
            left: AssignTarget::Simple(SimpleAssignTarget::Ident(binding.clone())),
            right: left,
        });

        let reference1 = Expr::Ident(binding.id.clone());
        let reference2 = Expr::Ident(binding.id.clone());

        self.create_conditional_expression(assignment, reference1, reference2, right)
    }

    /// Create a conditional expression for nullish coalescing.
    ///
    /// ```js
    /// // Input
    /// foo ?? "default"
    ///
    /// // Output
    /// foo !== null && foo !== void 0 ? foo : "default"
    /// ```
    fn create_conditional_expression(
        &self,
        assignment: Expr,
        reference1: Expr,
        reference2: Expr,
        default: Expr,
    ) -> Expr {
        // Create `null` literal
        let null = Expr::Lit(Lit::Null(Null { span: DUMMY_SP }));

        // Create `void 0`
        let void_0 = Expr::Unary(UnaryExpr {
            span: DUMMY_SP,
            op: op!("void"),
            arg: Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: 0.0,
                raw: None,
            }))),
        });

        // Create `assignment !== null`
        let left = Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: op!("!=="),
            left: Box::new(assignment),
            right: Box::new(null),
        });

        // Create `reference1 !== void 0`
        let right = Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: op!("!=="),
            left: Box::new(reference1),
            right: Box::new(void_0),
        });

        // Create `assignment !== null && reference1 !== void 0`
        let test = Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: op!("&&"),
            left: Box::new(left),
            right: Box::new(right),
        });

        // Create `test ? reference2 : default`
        Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test: Box::new(test),
            cons: Box::new(reference2),
            alt: Box::new(default),
        })
    }
}

impl VisitMutHook for NullishCoalescingOperator<'_> {
    fn exit_expr(&mut self, expr: &mut Expr) {
        // Check if this is a nullish coalescing operator
        if let Expr::Bin(BinExpr {
            op: op!("??"),
            left,
            right,
            ..
        }) = expr
        {
            // Transform the expression
            let left = mem::take(left);
            let right = mem::take(right);
            *expr = self.transform_logical_expression(left, *right);
        }
    }

    fn enter_module(&mut self, _module: &mut Module) {
        // Record entering statements
        self.var_declarations.record_entering_statements();
    }

    fn exit_module(&mut self, module: &mut Module) {
        // Insert variable declarations at the top of the module
        for item in &mut module.body {
            if let ModuleItem::Stmt(Stmt::Block(block)) = item {
                self.var_declarations
                    .insert_into_statements(&mut block.stmts);
                break;
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
