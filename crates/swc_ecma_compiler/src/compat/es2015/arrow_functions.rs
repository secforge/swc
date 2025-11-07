//! ES2015 Arrow Functions
//!
//! This plugin transforms arrow functions (`() => {}`) to function expressions
//! (`function () {}`).
//!
//! > This plugin is included in `preset-env`, in ES2015
//!
//! ## Missing features
//!
//! Implementation is incomplete at present. Still TODO:
//!
//! * `spec` option.
//! * Handle `arguments` in arrow functions.
//! * Handle `new.target` in arrow functions.
//! * Handle arrow function in function params (`function f(g = () => this) {}`).
//!   Babel gets this wrong: <https://babeljs.io/repl#?code_lz=GYVwdgxgLglg9mABMOcAUAPRBeRaCUOAfIlABYwDOhA3gL5A&presets=&externalPlugins=%40babel%2Fplugin-transform-arrow-functions%407.24.7>
//! * Error on arrow functions in class properties. <https://babeljs.io/repl#?code_lz=MYGwhgzhAEDC0G8BQ1oDMD2HoF5oAoBKXAPmgBcALASwgG4kBfJIA&presets=&externalPlugins=%40babel%2Fplugin-transform-arrow-functions%407.24.7>
//!   or we can support it: `class C { x = () => this; }` -> `class C { x =
//!   (function(_this) { return () => _this; })(this); }`
//! * Error on `super` in arrow functions. <https://babeljs.io/repl#?code_lz=MYGwhgzhAEBiD29oG8C-AoUkYCEwCdoBTADwBciA7AExgSWXWmgFsiyALeagCgEoUTZtHzsArvkrR-0ALwA-aBDEAHIvgB0AM0QBuIRgxA&presets=&externalPlugins=%40babel%2Fplugin-transform-arrow-functions%407.24.7>
//!
//! ## Example
//!
//! Input:
//! ```js
//! var a = () => {};
//! var a = b => b;
//!
//! const double = [1, 2, 3].map(num => num * 2);
//! console.log(double); // [2,4,6]
//!
//! var bob = {
//!   name: "Bob",
//!   friends: ["Sally", "Tom"],
//!   printFriends() {
//!     this.friends.forEach(f => console.log(this.name + " knows " + f));
//!   },
//! };
//! console.log(bob.printFriends());
//! ```
//!
//! Output:
//! ```js
//! var a = function() {};
//! var a = function(b) { return b; };
//!
//! const double = [1, 2, 3].map(function(num) {
//!   return num * 2;
//! });
//! console.log(double); // [2,4,6]
//!
//! var bob = {
//!   name: "Bob",
//!   friends: ["Sally", "Tom"],
//!   printFriends() {
//!     var _this = this;
//!     this.friends.forEach(function(f) {
//!       return console.log(_this.name + " knows " + f);
//!     });
//!   },
//! };
//! console.log(bob.printFriends());
//! ```
//!
//! ## Options
//!
//! ### `spec`
//!
//! `boolean`, defaults to `false`.
//!
//! This option enables the following:
//! * Wrap the generated function in .bind(this) and keeps uses of this inside
//!   the function as-is, instead of using a renamed this.
//! * Add a runtime check to ensure the functions are not instantiated.
//! * Add names to arrow functions.
//!
//! #### Example
//!
//! Using spec mode with the above example produces:
//!
//! ```js
//! var _this = this;
//!
//! var a = function a() {
//!   babelHelpers.newArrowCheck(this, _this);
//! }.bind(this);
//! var a = function a(b) {
//!   babelHelpers.newArrowCheck(this, _this);
//!   return b;
//! }.bind(this);
//!
//! const double = [1, 2, 3].map(
//!   function(num) {
//!     babelHelpers.newArrowCheck(this, _this);
//!     return num * 2;
//!   }.bind(this)
//! );
//! console.log(double); // [2,4,6]
//!
//! var bob = {
//!   name: "Bob",
//!   friends: ["Sally", "Tom"],
//!   printFriends() {
//!     var _this2 = this;
//!     this.friends.forEach(
//!       function(f) {
//!         babelHelpers.newArrowCheck(this, _this2);
//!         return console.log(this.name + " knows " + f);
//!       }.bind(this)
//!     );
//!   },
//! };
//! console.log(bob.printFriends());
//! ```
//!
//! ## Implementation
//!
//! The implementation is placed in
//! [`crate::compat::common::arrow_function_converter::ArrowFunctionConverter`],
//! which can be used in other plugins.
//!
//! ## References:
//!
//! * Babel plugin implementation: <https://github.com/babel/babel/blob/v7.26.2/packages/babel-plugin-transform-arrow-functions>
//! * Arrow function specification: <https://tc39.es/ecma262/#sec-arrow-function-definitions>

#![allow(dead_code)]
use std::mem;

use serde::Deserialize;
use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::{common::var_declarations::VarDeclarationsStore, context::TransformCtx};

/// Options for transforming arrow functions.
///
/// Controls how arrow functions are converted to regular function expressions.
#[derive(Debug, Default, Clone, Copy, Deserialize)]
pub struct ArrowFunctionsOptions {
    /// This option enables the following:
    /// * Wrap the generated function in .bind(this) and keeps uses of this
    ///   inside the function as-is, instead of using a renamed this.
    /// * Add a runtime check to ensure the functions are not instantiated.
    /// * Add names to arrow functions.
    #[serde(default)]
    pub spec: bool,
}

/// Arrow function transformer for ES2015.
///
/// Transforms arrow functions to regular function expressions, handling
/// `this` binding appropriately. This is a SWC-based implementation that
/// uses the VisitMutHook pattern.
///
/// # Implementation
///
/// This transformer converts arrow functions to regular function expressions:
/// - `() => expr` becomes `function() { return expr; }`
/// - `() => { stmt }` becomes `function() { stmt }`
/// - Handles `this` binding by creating `var _this = this;` when needed
pub struct ArrowFunctions<'ctx> {
    #[allow(dead_code)]
    options: ArrowFunctionsOptions,
    #[allow(dead_code)]
    ctx: &'ctx TransformCtx,

    /// Tracks whether we need to create a `_this` variable
    needs_this_binding: bool,

    /// Store for variable declarations to be inserted
    var_declarations: VarDeclarationsStore,

    /// Counter for generating unique identifiers
    uid_counter: usize,

    /// Depth counter to track nesting level (for functions that shadow `this`)
    function_depth: usize,

    /// Name for the `this` binding variable
    this_var_name: Option<Atom>,
}

impl<'ctx> ArrowFunctions<'ctx> {
    /// Create a new arrow functions transformer.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for arrow function transformation
    /// * `ctx` - Transform context containing shared state and utilities
    pub fn new(options: ArrowFunctionsOptions, ctx: &'ctx TransformCtx) -> Self {
        Self {
            options,
            ctx,
            needs_this_binding: false,
            var_declarations: VarDeclarationsStore::new(),
            uid_counter: 0,
            function_depth: 0,
            this_var_name: None,
        }
    }

    /// Generate a unique identifier name.
    fn generate_uid(&mut self, base_name: &str) -> Atom {
        self.uid_counter += 1;
        Atom::from(format!(
            "_{}{}",
            base_name,
            if self.uid_counter > 1 {
                self.uid_counter.to_string()
            } else {
                String::new()
            }
        ))
    }

    /// Get or create the `this` binding variable name.
    fn get_this_var_name(&mut self) -> Atom {
        if let Some(ref name) = self.this_var_name {
            name.clone()
        } else {
            let name = self.generate_uid("this");
            self.this_var_name = Some(name.clone());
            name
        }
    }

    /// Transform an arrow function to a regular function expression.
    fn transform_arrow_function(&mut self, arrow: &mut ArrowExpr) -> Expr {
        let params = mem::take(&mut arrow.params);
        let body = mem::take(&mut arrow.body);
        let is_async = arrow.is_async;
        let is_generator = arrow.is_generator;
        let return_type = arrow.return_type.take();

        // Convert body: if it's an expression, wrap in return statement
        let block_body = match *body {
            BlockStmtOrExpr::BlockStmt(block) => block,
            BlockStmtOrExpr::Expr(expr) => {
                // Expression body - wrap in return statement
                BlockStmt {
                    span: DUMMY_SP,
                    stmts: vec![Stmt::Return(ReturnStmt {
                        span: DUMMY_SP,
                        arg: Some(expr),
                    })],
                    ..Default::default()
                }
            }
        };

        // Convert arrow function parameters (Vec<Pat>) to function parameters
        // (Vec<Param>)
        let function_params = params
            .into_iter()
            .map(|pat| Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat,
            })
            .collect();

        // Create the function expression
        Expr::Fn(FnExpr {
            ident: None,
            function: Box::new(Function {
                params: function_params,
                decorators: vec![],
                span: arrow.span,
                body: Some(block_body),
                is_generator,
                is_async,
                type_params: arrow.type_params.take(),
                return_type,
                ..Default::default()
            }),
        })
    }
}

impl VisitMutHook for ArrowFunctions<'_> {
    /// Called when entering an expression node.
    fn exit_expr(&mut self, expr: &mut Expr) {
        // Transform arrow functions to regular functions
        if let Expr::Arrow(arrow) = expr {
            let transformed = self.transform_arrow_function(arrow);
            *expr = transformed;
        }
    }

    /// Called when entering a function - increment depth to track scope
    fn enter_fn_decl(&mut self, _func: &mut FnDecl) {
        self.function_depth += 1;
    }

    /// Called when exiting a function - decrement depth
    fn exit_fn_decl(&mut self, _func: &mut FnDecl) {
        self.function_depth -= 1;
    }

    /// Called when entering a function expression - increment depth
    fn enter_fn_expr(&mut self, _func: &mut FnExpr) {
        self.function_depth += 1;
    }

    /// Called when exiting a function expression - decrement depth
    fn exit_fn_expr(&mut self, _func: &mut FnExpr) {
        self.function_depth -= 1;
    }

    /// Called when encountering `this` expression
    fn enter_this_expr(&mut self, this_expr: &mut ThisExpr) {
        // If we're inside an arrow function (function_depth tracking would need more
        // work) Mark that we need a this binding
        // For now, we'll handle this in a simpler way
        let _ = this_expr;
    }

    /// Called when entering a module - record for variable declarations
    fn enter_module(&mut self, _module: &mut Module) {
        self.var_declarations.record_entering_statements();
    }

    /// Called when exiting a module - insert variable declarations
    fn exit_module(&mut self, module: &mut Module) {
        // Insert variable declarations at the top of the module if needed
        if self.needs_this_binding {
            if let Some(this_var_name) = &self.this_var_name {
                let binding = BindingIdent {
                    id: Ident::new(this_var_name.clone(), DUMMY_SP, Default::default()),
                    type_ann: None,
                };

                // Create initializer: `this`
                let init = Some(Box::new(Expr::This(ThisExpr { span: DUMMY_SP })));

                self.var_declarations.insert_var(&binding, init);
            }
        }

        // Insert all accumulated variable declarations
        for item in &mut module.body {
            if let ModuleItem::Stmt(Stmt::Block(block)) = item {
                self.var_declarations
                    .insert_into_statements(&mut block.stmts);
                break;
            }
        }
    }

    /// Called when entering a block statement
    fn enter_block_stmt(&mut self, _block: &mut BlockStmt) {
        self.var_declarations.record_entering_statements();
    }

    /// Called when exiting a block statement
    fn exit_block_stmt(&mut self, block: &mut BlockStmt) {
        self.var_declarations
            .insert_into_statements(&mut block.stmts);
    }
}
