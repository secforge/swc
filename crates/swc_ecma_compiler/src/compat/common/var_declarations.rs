//! Utility transform to add `var` or `let` declarations to top of statement
//! blocks.
//!
//! `VarDeclarationsStore` contains a stack of `Declarators`, each comprising
//! 2 x `Vec<VarDeclarator>` (1 for `var`s, 1 for `let`s).
//! `VarDeclarationsStore` is stored on `TransformCtx`.
//!
//! Unlike the OXC version which uses a Traverse-based approach, this SWC
//! version requires manual integration into visitor methods. Each transform
//! that needs to insert variable declarations must:
//!
//! 1. Call `record_entering_statements()` when entering a statement block
//! 2. Call `insert_into_statements()` when exiting a statement block
//! 3. Call `insert_into_program()` when exiting the program
//!
//! Other transforms can add declarators to the store by calling methods of
//! `VarDeclarationsStore`:
//!
//! ```rs
//! self.var_declarations.insert_var(&binding_ident, None);
//! self.var_declarations.insert_let(&binding_ident2, Some(init_expr));
//! ```

#![allow(dead_code)]

use std::cell::RefCell;

use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

use crate::compat::context::TransformCtx;

/// Store for `VarDeclarator`s to be added to enclosing statement block.
pub struct VarDeclarationsStore {
    stack: RefCell<Vec<Option<Declarators>>>,
}

/// Declarators to be inserted in a statement block.
struct Declarators {
    var_declarators: Vec<VarDeclarator>,
    let_declarators: Vec<VarDeclarator>,
}

impl Declarators {
    fn new() -> Self {
        Self {
            var_declarators: Vec::new(),
            let_declarators: Vec::new(),
        }
    }
}

// Public methods
impl VarDeclarationsStore {
    /// Create new `VarDeclarationsStore`.
    pub fn new() -> Self {
        Self {
            stack: RefCell::new(Vec::new()),
        }
    }

    /// Add a `var` declaration to be inserted at top of current enclosing
    /// statement block, given a binding identifier.
    #[inline]
    pub fn insert_var(&self, binding: &BindingIdent, init: Option<Box<Expr>>) {
        let pattern = Pat::Ident(binding.clone());
        self.insert_var_binding_pattern(pattern, init);
    }

    /// Add a `var` declaration with the given init expression to be inserted at
    /// top of current enclosing statement block, given a binding
    /// identifier.
    #[inline]
    pub fn insert_var_with_init(&self, binding: &BindingIdent, init: Box<Expr>) {
        self.insert_var(binding, Some(init));
    }

    /// Create a new UID based on `name`, add a `var` declaration to be inserted
    /// at the top of the current enclosing statement block, and return the
    /// `BindingIdent`.
    ///
    /// Note: This is a simplified version. In a real implementation, you would
    /// need to:
    /// 1. Generate a unique identifier based on `name`
    /// 2. Ensure it doesn't conflict with existing identifiers in scope
    /// 3. Track the binding in a symbol table
    #[inline]
    pub fn create_uid_var(&self, name: &str, ctx: &TransformCtx) -> BindingIdent {
        let binding = create_uid_binding(name, ctx);
        self.insert_var(&binding, None);
        binding
    }

    /// Create a new UID based on `name`, add a `var` declaration with the given
    /// init expression to be inserted at the top of the current enclosing
    /// statement block, and return the `BindingIdent`.
    #[inline]
    pub fn create_uid_var_with_init(
        &self,
        name: &str,
        expression: Box<Expr>,
        ctx: &TransformCtx,
    ) -> BindingIdent {
        let binding = create_uid_binding(name, ctx);
        self.insert_var_with_init(&binding, expression);
        binding
    }

    /// Add a `let` declaration to be inserted at top of current enclosing
    /// statement block, given a binding identifier.
    pub fn insert_let(&self, binding: &BindingIdent, init: Option<Box<Expr>>) {
        let pattern = Pat::Ident(binding.clone());
        self.insert_let_binding_pattern(pattern, init);
    }

    /// Add a `var` declaration to be inserted at top of current enclosing
    /// statement block, given a `Pat` (binding pattern).
    pub fn insert_var_binding_pattern(&self, pattern: Pat, init: Option<Box<Expr>>) {
        let declarator = VarDeclarator {
            span: DUMMY_SP,
            name: pattern,
            init,
            definite: false,
        };
        self.insert_var_declarator(declarator);
    }

    /// Add a `let` declaration to be inserted at top of current enclosing
    /// statement block, given a `Pat` (binding pattern).
    pub fn insert_let_binding_pattern(&self, pattern: Pat, init: Option<Box<Expr>>) {
        let declarator = VarDeclarator {
            span: DUMMY_SP,
            name: pattern,
            init,
            definite: false,
        };
        self.insert_let_declarator(declarator);
    }

    /// Add a `var` declaration to be inserted at top of current enclosing
    /// statement block.
    pub fn insert_var_declarator(&self, declarator: VarDeclarator) {
        let mut stack = self.stack.borrow_mut();
        if let Some(declarators) = stack.last_mut() {
            let declarators = declarators.get_or_insert_with(Declarators::new);
            declarators.var_declarators.push(declarator);
        }
    }

    /// Add a `let` declaration to be inserted at top of current enclosing
    /// statement block.
    pub fn insert_let_declarator(&self, declarator: VarDeclarator) {
        let mut stack = self.stack.borrow_mut();
        if let Some(declarators) = stack.last_mut() {
            let declarators = declarators.get_or_insert_with(Declarators::new);
            declarators.let_declarators.push(declarator);
        }
    }
}

// Internal methods
impl VarDeclarationsStore {
    /// Record that we are entering a statements block.
    /// Must be called when entering any block that contains statements.
    pub fn record_entering_statements(&self) {
        let mut stack = self.stack.borrow_mut();
        stack.push(None);
    }

    /// Insert accumulated var/let declarations into the statements block.
    /// Must be called when exiting any block that contains statements.
    ///
    /// Returns true if any declarations were inserted.
    pub fn insert_into_statements(&self, stmts: &mut Vec<Stmt>) -> bool {
        if let Some((var_statement, let_statement)) = self.get_var_statement() {
            let mut new_stmts = Vec::with_capacity(stmts.len() + 2);
            match (var_statement, let_statement) {
                (Some(var_statement), Some(let_statement)) => {
                    // Insert `var` and `let` statements
                    new_stmts.push(var_statement);
                    new_stmts.push(let_statement);
                }
                (Some(statement), None) | (None, Some(statement)) => {
                    // Insert `var` or `let` statement
                    new_stmts.push(statement);
                }
                (None, None) => return false,
            }
            new_stmts.append(stmts);
            *stmts = new_stmts;
            true
        } else {
            false
        }
    }

    /// Check if there are any pending declarations at the top level.
    /// Should be called when exiting the program to ensure all declarations are
    /// processed.
    pub fn has_pending_declarations(&self) -> bool {
        let stack = self.stack.borrow();
        !stack.is_empty()
            && stack.iter().any(|opt| {
                opt.as_ref()
                    .is_some_and(|d| !d.var_declarators.is_empty() || !d.let_declarators.is_empty())
            })
    }

    /// Get pending var/let statements for top-level insertion.
    /// Returns None if no declarations are pending.
    pub fn get_top_level_statements(&self) -> Option<Vec<Stmt>> {
        let (var_statement, let_statement) = self.get_var_statement()?;

        let mut stmts = Vec::new();
        if let Some(var_stmt) = var_statement {
            stmts.push(var_stmt);
        }
        if let Some(let_stmt) = let_statement {
            stmts.push(let_stmt);
        }

        if stmts.is_empty() {
            None
        } else {
            Some(stmts)
        }
    }

    #[inline]
    fn get_var_statement(&self) -> Option<(Option<Stmt>, Option<Stmt>)> {
        let mut stack = self.stack.borrow_mut();
        let declarators = stack.pop()??;

        let Declarators {
            var_declarators,
            let_declarators,
        } = declarators;

        let var_statement = (!var_declarators.is_empty())
            .then(|| Self::create_declaration(VarDeclKind::Var, var_declarators));
        let let_statement = (!let_declarators.is_empty())
            .then(|| Self::create_declaration(VarDeclKind::Let, let_declarators));

        Some((var_statement, let_statement))
    }

    fn create_declaration(kind: VarDeclKind, declarators: Vec<VarDeclarator>) -> Stmt {
        Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            kind,
            declare: false,
            decls: declarators,
            ..Default::default()
        })))
    }
}

impl Default for VarDeclarationsStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a unique identifier binding.
///
/// This is a simplified implementation. A production version would need to:
/// - Track existing identifiers to avoid conflicts
/// - Generate truly unique names
/// - Integrate with a symbol table
fn create_uid_binding(name: &str, _ctx: &TransformCtx) -> BindingIdent {
    use swc_atoms::Atom;

    // Simple UID generation - in production this should check for conflicts
    let uid_name = if name.starts_with('_') {
        Atom::from(name.to_string())
    } else {
        Atom::from(format!("_{name}"))
    };

    BindingIdent {
        id: Ident::new(uid_name, DUMMY_SP, Default::default()),
        type_ann: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_declarations_store() {
        let store = VarDeclarationsStore::new();

        // Enter a statement block
        store.record_entering_statements();

        // Add a var declaration
        let binding = BindingIdent {
            id: Ident::new("test".into(), DUMMY_SP, Default::default()),
            type_ann: None,
        };
        store.insert_var(&binding, None);

        // Get the declaration
        let (var_stmt, let_stmt) = store.get_var_statement().unwrap();
        assert!(var_stmt.is_some());
        assert!(let_stmt.is_none());
    }

    #[test]
    fn test_multiple_declarations() {
        let store = VarDeclarationsStore::new();

        store.record_entering_statements();

        let var_binding = BindingIdent {
            id: Ident::new("var_test".into(), DUMMY_SP, Default::default()),
            type_ann: None,
        };
        let let_binding = BindingIdent {
            id: Ident::new("let_test".into(), DUMMY_SP, Default::default()),
            type_ann: None,
        };

        store.insert_var(&var_binding, None);
        store.insert_let(&let_binding, None);

        let (var_stmt, let_stmt) = store.get_var_statement().unwrap();
        assert!(var_stmt.is_some());
        assert!(let_stmt.is_some());
    }
}
