//! ES2020: Export Namespace From
//!
//! This plugin transforms `export * as ns from "mod"` syntax to use separate
//! import and export statements.
//!
//! ## Example
//!
//! Input:
//! ```javascript
//! export * as ns from "mod";
//! ```
//!
//! Output:
//! ```javascript
//! import * as _ns from "mod";
//! export { _ns as ns };
//! ```
//!
//! ## References
//!
//! * Babel plugin: <https://babeljs.io/docs/babel-plugin-proposal-export-namespace-from>
//! * TC39 proposal: <https://github.com/tc39/proposal-export-ns-from>

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ExportNamedSpecifier, ExportSpecifier, Ident, ImportDecl, ImportSpecifier,
    ImportStarAsSpecifier, Module, ModuleDecl, ModuleExportName, ModuleItem, NamedExport,
};
use swc_ecma_hooks::VisitMutHook;

/// Transforms `export * as ns from "mod"` to separate import/export statements.
pub struct ExportNamespaceFrom {
    uid_counter: usize,
}

impl ExportNamespaceFrom {
    pub fn new() -> Self {
        Self { uid_counter: 0 }
    }

    /// Generates a unique identifier based on the given name.
    fn generate_uid(&mut self, name: &str) -> String {
        self.uid_counter += 1;
        format!(
            "_{}{}",
            name,
            if self.uid_counter > 1 {
                self.uid_counter.to_string()
            } else {
                String::new()
            }
        )
    }

    /// Transforms export namespace specifiers.
    ///
    /// Example:
    /// ```javascript
    /// export * as ns from "mod";
    /// // becomes
    /// import * as _ns from "mod";
    /// export { _ns as ns };
    /// ```
    fn transform_export_namespace(
        &mut self,
        named_export: &NamedExport,
    ) -> Option<Vec<ModuleDecl>> {
        // Check if this is `export * as ns from "mod"`
        if named_export.specifiers.len() != 1 {
            return None;
        }

        let spec = named_export.specifiers.first()?;
        let namespace_spec = match spec {
            ExportSpecifier::Namespace(ns) => ns,
            _ => return None,
        };

        // Must have a source
        let src = named_export.src.as_ref()?;

        // Get the exported name
        let exported_name = &namespace_spec.name;
        // Use a simple name for UID generation
        let base_name = match exported_name {
            ModuleExportName::Ident(ident) => format!("{}", ident.sym),
            ModuleExportName::Str(_) => "ns".to_string(),
        };

        // Generate unique identifier for the import
        let import_name = self.generate_uid(&base_name);

        // Create `import * as _ns from "mod"`
        let import_decl = ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![ImportSpecifier::Namespace(ImportStarAsSpecifier {
                span: DUMMY_SP,
                local: Ident {
                    span: DUMMY_SP,
                    ctxt: Default::default(),
                    sym: Atom::from(import_name.clone()),
                    optional: false,
                },
            })],
            src: src.clone(),
            type_only: named_export.type_only,
            with: named_export.with.clone(),
            phase: Default::default(),
        });

        // Create `export { _ns as ns }`
        let export_spec = ExportSpecifier::Named(ExportNamedSpecifier {
            span: DUMMY_SP,
            orig: ModuleExportName::Ident(Ident {
                span: DUMMY_SP,
                ctxt: Default::default(),
                sym: Atom::from(import_name),
                optional: false,
            }),
            exported: Some(exported_name.clone()),
            is_type_only: false,
        });

        let export_decl = ModuleDecl::ExportNamed(NamedExport {
            span: DUMMY_SP,
            specifiers: vec![export_spec],
            src: None,
            type_only: false,
            with: None,
        });

        Some(vec![import_decl, export_decl])
    }
}

impl VisitMutHook for ExportNamespaceFrom {
    fn exit_module(&mut self, module: &mut Module) {
        // First collect all export namespace declarations that need transformation
        let mut to_transform = vec![];

        for (idx, item) in module.body.iter().enumerate() {
            if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export)) = item {
                if named_export.src.is_some()
                    && named_export.specifiers.len() == 1
                    && matches!(
                        named_export.specifiers.first(),
                        Some(ExportSpecifier::Namespace(_))
                    )
                {
                    to_transform.push(idx);
                }
            }
        }

        // Transform each export namespace declaration
        for idx in to_transform.into_iter().rev() {
            if let Some(ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export))) =
                module.body.get(idx)
            {
                if let Some(new_decls) = self.transform_export_namespace(named_export) {
                    // Replace the export namespace with the import and export declarations
                    let new_items: Vec<_> =
                        new_decls.into_iter().map(ModuleItem::ModuleDecl).collect();

                    // Remove the original export namespace and insert the new ones
                    module.body.splice(idx..=idx, new_items);
                }
            }
        }
    }
}

impl Default for ExportNamespaceFrom {
    fn default() -> Self {
        Self::new()
    }
}
