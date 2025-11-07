//! Utility to add `import` / `require` statements to top of program.
//!
//! `ModuleImportsStore` contains an `IndexMap<Atom, Vec<ImportKind>>`.
//! It is stored on the visitor struct.
//!
//! Other transforms can add `import`s / `require`s to the store by calling
//! methods of `ModuleImportsStore`.
//!
//! ### Usage
//!
//! ```rs
//! // import { jsx as _jsx } from 'react';
//! self.module_imports.add_named_import(
//!     Atom::from("react"),
//!     Atom::from("jsx"),
//!     private_ident!("_jsx"),
//!     false
//! );
//!
//! // ESM: import React from 'react';
//! // CJS: var _React = require('react');
//! self.module_imports.add_default_import(
//!     Atom::from("react"),
//!     private_ident!("React"),
//!     false
//! );
//! ```
//!
//! > NOTE: Using `import` or `require` is determined by the source type (module
//! > vs script).
//!
//! Based on `@babel/helper-module-imports`
//! <https://github.com/nicolo-ribaudo/babel/tree/v7.25.8/packages/babel-helper-module-imports>

#![allow(dead_code)]

use std::cell::RefCell;

use indexmap::{map::Entry as IndexMapEntry, IndexMap};
use swc_atoms::Atom;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;

/// A named import binding.
struct NamedImport {
    imported: Atom,
    local: Ident,
}

/// Represents a single import.
enum Import {
    Named(NamedImport),
    Default(Ident),
}

/// Store for `import` / `require` statements to be added at top of program.
///
/// This manages a collection of imports that will be inserted into the program
/// when `finalize()` is called.
pub struct ModuleImportsStore {
    imports: RefCell<IndexMap<Atom, Vec<Import>>>,
}

// Public methods
impl ModuleImportsStore {
    /// Create new `ModuleImportsStore`.
    pub fn new() -> Self {
        Self {
            imports: RefCell::new(IndexMap::default()),
        }
    }

    /// Add default `import` or `require` to top of program.
    ///
    /// Which it will be depends on the source type (determined at
    /// finalization).
    ///
    /// * `import local from 'source';` or
    /// * `var local = require('source');`
    ///
    /// If `front` is `true`, `import`/`require` is added to front of the
    /// `import`s/`require`s.
    pub fn add_default_import(&self, source: Atom, local: Ident, front: bool) {
        self.add_import(source, Import::Default(local), front);
    }

    /// Add named `import` to top of program.
    ///
    /// `import { imported as local } from 'source';`
    ///
    /// If `front` is `true`, `import` is added to front of the `import`s.
    ///
    /// Adding named `require`s is not supported, and will cause a panic later
    /// on.
    pub fn add_named_import(&self, source: Atom, imported: Atom, local: Ident, front: bool) {
        self.add_import(
            source,
            Import::Named(NamedImport { imported, local }),
            front,
        );
    }

    /// Returns `true` if no imports have been scheduled for insertion.
    pub fn is_empty(&self) -> bool {
        self.imports.borrow().is_empty()
    }

    /// Finalize and return the import/require statements to be inserted.
    ///
    /// Returns a vector of `ModuleItem`s that should be prepended to the
    /// program.
    ///
    /// * `is_module` - `true` for ESM (generates imports), `false` for scripts
    ///   (generates requires)
    pub fn finalize(&self, is_module: bool) -> Vec<ModuleItem> {
        let mut imports = self.imports.borrow_mut();
        if imports.is_empty() {
            return Vec::new();
        }

        if is_module {
            imports
                .drain(..)
                .map(|(source, names)| Self::create_import_decl(source, names))
                .collect()
        } else {
            imports
                .drain(..)
                .map(|(source, names)| Self::create_require_stmt(source, names))
                .collect()
        }
    }
}

// Internal methods
impl ModuleImportsStore {
    /// Add `import` or `require` to the store.
    ///
    /// If `front` is `true`, `import`/`require` is added to front of the
    /// `import`s/`require`s.
    fn add_import(&self, source: Atom, import: Import, front: bool) {
        match self.imports.borrow_mut().entry(source) {
            IndexMapEntry::Occupied(mut entry) => {
                entry.get_mut().push(import);
                if front && entry.index() != 0 {
                    entry.move_index(0);
                }
            }
            IndexMapEntry::Vacant(entry) => {
                let imports = vec![import];
                if front {
                    entry.shift_insert(0, imports);
                } else {
                    entry.insert(imports);
                }
            }
        }
    }

    /// Create an ESM import declaration.
    ///
    /// `import { named_import } from 'source';` or `import default_import from
    /// 'source';`
    fn create_import_decl(source: Atom, names: Vec<Import>) -> ModuleItem {
        let specifiers = names
            .into_iter()
            .map(|import| match import {
                Import::Named(import) => ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: import.local,
                    imported: Some(ModuleExportName::Ident(Ident {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        sym: import.imported,
                        optional: false,
                    })),
                    is_type_only: false,
                }),
                Import::Default(local) => ImportSpecifier::Default(ImportDefaultSpecifier {
                    span: DUMMY_SP,
                    local,
                }),
            })
            .collect();

        ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers,
            src: Box::new(Str {
                span: DUMMY_SP,
                value: source.into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: Default::default(),
        }))
    }

    /// Create a CJS require statement.
    ///
    /// `var local = require('source');`
    ///
    /// Only default imports are supported for require statements.
    fn create_require_stmt(source: Atom, names: Vec<Import>) -> ModuleItem {
        let Some(Import::Default(local)) = names.into_iter().next() else {
            panic!("Named imports are not supported for require statements");
        };

        let require_call = Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(Ident {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                sym: Atom::from("require"),
                optional: false,
            }))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: source.into(),
                    raw: None,
                }))),
            }],
            ..Default::default()
        });

        ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            kind: VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(local.into()),
                init: Some(Box::new(require_call)),
                definite: false,
            }],
            ..Default::default()
        }))))
    }
}

impl Default for ModuleImportsStore {
    fn default() -> Self {
        Self::new()
    }
}
