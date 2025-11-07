//! TypeScript transformation for SWC
//!
//! [Preset TypeScript](https://babeljs.io/docs/babel-preset-typescript)
//!
//! This preset includes the following plugins:
//!
//! * [transform-typescript](https://babeljs.io/docs/babel-plugin-transform-typescript)
//!
//! This plugin adds support for the types syntax used by the TypeScript
//! programming language. However, this plugin does not add the ability to
//! type-check the JavaScript passed to it. For that, you will need to install
//! and set up TypeScript.
//!
//! Note that although the TypeScript compiler tsc actively supports certain
//! JavaScript proposals such as optional chaining (?.), nullish coalescing (??)
//! and class properties (this.#x), this preset does not include these features
//! because they are not the types syntax available in TypeScript only.
//! We recommend using preset-env with preset-typescript if you want to
//! transpile these features.
//!
//! ## Example
//!
//! In:  `const x: number = 0;`
//! Out: `const x = 0;`

use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;
use swc_ecma_visit::VisitMut;

use crate::compat::context::TransformCtx;

mod annotations;
mod class;
mod diagnostics;
mod r#enum;
mod module;
mod namespace;
mod options;
mod rewrite_extensions;

use annotations::TypeScriptAnnotations;
use class::TypeScriptClass;
use module::TypeScriptModule;
use namespace::TypeScriptNamespace;
pub use options::TypeScriptOptions;
use r#enum::TypeScriptEnum;
use rewrite_extensions::TypeScriptRewriteExtensions;

/// TypeScript transformation
///
/// This struct coordinates all TypeScript-related transformations:
/// - Type annotation removal
/// - Enum transformation
/// - Namespace transformation
/// - Module syntax transformation
/// - Import/export extension rewriting
pub struct TypeScript<'a> {
    ctx: &'a TransformCtx,
    annotations: TypeScriptAnnotations<'a>,
    r#enum: TypeScriptEnum,
    namespace: TypeScriptNamespace<'a>,
    module: TypeScriptModule<'a>,
    class: TypeScriptClass<'a>,
    rewrite_extensions: Option<TypeScriptRewriteExtensions>,
}

impl<'a> TypeScript<'a> {
    /// Create a new TypeScript transformation instance
    ///
    /// # Arguments
    /// * `options` - TypeScript transformation options
    /// * `ctx` - Transform context
    /// * `is_esm` - Whether the module is ES module (true) or CommonJS (false)
    pub fn new(options: &TypeScriptOptions, ctx: &'a TransformCtx, is_esm: bool) -> Self {
        let remove_class_fields_without_initializer =
            !options.allow_declare_fields || options.remove_class_fields_without_initializer;

        Self {
            ctx,
            annotations: TypeScriptAnnotations::new(options, ctx),
            r#enum: TypeScriptEnum::new(),
            namespace: TypeScriptNamespace::new(options, ctx),
            module: TypeScriptModule::new(options.only_remove_type_imports, ctx, is_esm),
            class: TypeScriptClass::new(ctx, remove_class_fields_without_initializer),
            rewrite_extensions: options
                .rewrite_import_extensions
                .map(TypeScriptRewriteExtensions::new),
        }
    }
}

impl VisitMutHook for TypeScript<'_> {
    fn enter_module(&mut self, module: &mut Module) {
        // First, handle module-level transformations
        // Note: sub-modules still use VisitMut, so we call their visit_mut_* methods
        self.module.visit_mut_module(module);
        self.namespace.visit_mut_module(module);
        self.r#enum.visit_mut_module(module);

        // Then handle annotations (type removal)
        self.annotations.visit_mut_module(module);

        // Finally, rewrite import extensions if configured
        if let Some(rewrite) = &mut self.rewrite_extensions {
            rewrite.enter_module(module);
        }
    }

    fn enter_script(&mut self, script: &mut Script) {
        // Handle script mode (non-module)
        // Note: annotations still uses VisitMut
        self.annotations.visit_mut_script(script);
    }

    fn enter_class(&mut self, class: &mut Class) {
        // Apply class transformations
        // Note: sub-modules still use VisitMut
        self.class.visit_mut_class(class);
        self.annotations.visit_mut_class(class);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::{compat::context::TransformCtx, Config};

    fn create_test_ctx() -> TransformCtx {
        let config = Config::default();
        TransformCtx::new(Path::new("test.ts"), &config)
    }

    #[test]
    fn test_typescript_creation() {
        let ctx = create_test_ctx();
        let options = TypeScriptOptions::default();
        let _ts = TypeScript::new(&options, &ctx, true);
    }
}
