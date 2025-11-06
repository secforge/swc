//! Rewrite import extensions
//!
//! This plugin is used to rewrite/remove extensions from import/export source.
//! It is only handled source that contains `/` or `\` in the source.
//!
//! Based on Babel's [plugin-rewrite-ts-imports](https://github.com/babel/babel/blob/3bcfee232506a4cebe410f02042fb0f0adeeb0b1/packages/babel-preset-typescript/src/plugin-rewrite-ts-imports.ts)

use swc_ecma_ast::*;
use swc_ecma_visit::{noop_visit_mut_type, VisitMut};

use super::options::RewriteExtensionsMode;

pub struct TypeScriptRewriteExtensions {
    mode: RewriteExtensionsMode,
}

impl TypeScriptRewriteExtensions {
    pub fn new(mode: RewriteExtensionsMode) -> Self {
        Self { mode }
    }

    pub fn rewrite_extensions(&self, source: &mut Str) {
        let Some(value) = source.value.as_str() else {
            return;
        };
        if !value.contains(['/', '\\']) {
            return;
        }

        let Some((without_extension, extension)) = value.rsplit_once('.') else {
            return;
        };

        let replace = match extension {
            "mts" => ".mjs",
            "cts" => ".cjs",
            "ts" | "tsx" => ".js",
            _ => return, // do not rewrite or remove other unknown extensions
        };

        source.value = if self.mode.is_remove() {
            without_extension.into()
        } else {
            format!("{}{}", without_extension, replace).into()
        };
        source.raw = None;
    }
}

impl VisitMut for TypeScriptRewriteExtensions {
    noop_visit_mut_type!();

    fn visit_mut_import_decl(&mut self, node: &mut ImportDecl) {
        if node.type_only {
            return;
        }
        self.rewrite_extensions(&mut node.src);
    }

    fn visit_mut_named_export(&mut self, node: &mut NamedExport) {
        if node.type_only {
            return;
        }
        if let Some(src) = node.src.as_mut() {
            self.rewrite_extensions(src);
        }
    }

    fn visit_mut_export_all(&mut self, node: &mut ExportAll) {
        if node.type_only {
            return;
        }
        self.rewrite_extensions(&mut node.src);
    }
}
