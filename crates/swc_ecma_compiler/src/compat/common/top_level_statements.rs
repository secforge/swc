//! Utility for adding statements to top of program.
//!
//! ## Overview
//!
//! This module provides a mechanism for transforms to add statements to the top
//! of a program. Statements are inserted after any existing `import`
//! declarations.
//!
//! ## Note
//!
//! The OXC version uses `TransformCtx` and the `Traverse` pattern.
//! This SWC version adapts the functionality to work with SWC's architecture.

#![allow(dead_code)]

use std::cell::RefCell;

use swc_ecma_ast::*;

/// Store for statements to be added at top of program.
///
/// Other transforms can add statements to this store, and they will be inserted
/// at the top of the program after all imports.
///
/// ## Example Usage
///
/// ```rust,ignore
/// // Add a statement to be inserted
/// store.insert_statement(stmt);
///
/// // Later, when visiting the program:
/// store.insert_into_program(&mut program);
/// ```
pub struct TopLevelStatementsStore {
    stmts: RefCell<Vec<Stmt>>,
}

impl TopLevelStatementsStore {
    /// Create a new `TopLevelStatementsStore`.
    pub fn new() -> Self {
        Self {
            stmts: RefCell::new(vec![]),
        }
    }

    /// Add a statement to be inserted at top of program.
    ///
    /// The statement will be inserted after any `import` declarations
    /// when `insert_into_program` is called.
    ///
    /// ## Arguments
    /// * `stmt` - The statement to insert
    pub fn insert_statement(&self, stmt: Stmt) {
        self.stmts.borrow_mut().push(stmt);
    }

    /// Add multiple statements to be inserted at top of program.
    ///
    /// ## Arguments
    /// * `stmts` - An iterator of statements to insert
    pub fn insert_statements<I: IntoIterator<Item = Stmt>>(&self, stmts: I) {
        self.stmts.borrow_mut().extend(stmts);
    }

    /// Check if there are any statements to insert.
    ///
    /// ## Returns
    /// `true` if there are statements pending insertion
    pub fn is_empty(&self) -> bool {
        self.stmts.borrow().is_empty()
    }

    /// Get the number of statements pending insertion.
    ///
    /// ## Returns
    /// The number of statements to be inserted
    pub fn len(&self) -> usize {
        self.stmts.borrow().len()
    }

    /// Insert all pending statements at the top of the program.
    ///
    /// Statements are inserted after any existing `import` declarations,
    /// before the first non-import statement.
    ///
    /// ## Arguments
    /// * `program` - The program to modify
    pub fn insert_into_program(&self, program: &mut Program) {
        let mut stmts = self.stmts.borrow_mut();
        if stmts.is_empty() {
            return;
        }

        match program {
            Program::Module(module) => {
                // Find the first non-import statement
                let index = module
                    .body
                    .iter()
                    .position(|item| !is_import_or_export_decl(item))
                    .unwrap_or(module.body.len());

                // Insert all pending statements at that position
                let new_stmts = stmts.drain(..).map(ModuleItem::Stmt).collect::<Vec<_>>();
                module.body.splice(index..index, new_stmts);
            }
            Program::Script(script) => {
                // For scripts, we don't need to check for imports since they don't have them
                // Insert at the beginning
                let stmt_items: Vec<Stmt> = stmts.drain(..).collect();
                script.body.splice(0..0, stmt_items);
            }
        }
    }

    /// Clear all pending statements without inserting them.
    pub fn clear(&self) {
        self.stmts.borrow_mut().clear();
    }
}

impl Default for TopLevelStatementsStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a module item is an import or export declaration.
///
/// We want to insert statements after imports but before other declarations.
fn is_import_or_export_decl(item: &ModuleItem) -> bool {
    matches!(
        item,
        ModuleItem::ModuleDecl(ModuleDecl::Import(_))
            | ModuleItem::ModuleDecl(ModuleDecl::ExportAll(_))
            | ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(_))
    )
}
