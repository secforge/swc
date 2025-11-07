#![allow(dead_code)]
use swc_common::util::take::Take;
use swc_ecma_ast::*;
use swc_ecma_visit::{noop_visit_mut_type, VisitMut, VisitMutWith};

use super::TypeScriptOptions;
use crate::compat::context::TransformCtx;

/// TypeScript type annotation removal
///
/// Removes all TypeScript type annotations and syntax from the AST,
/// leaving only valid JavaScript code.
pub struct TypeScriptAnnotations<'a> {
    ctx: &'a TransformCtx,
    only_remove_type_imports: bool,
    has_jsx_element: bool,
    has_jsx_fragment: bool,
    jsx_element_import_name: String,
    jsx_fragment_import_name: String,
}

impl<'a> TypeScriptAnnotations<'a> {
    pub fn new(options: &TypeScriptOptions, ctx: &'a TransformCtx) -> Self {
        let jsx_element_import_name = if options.jsx_pragma.contains('.') {
            options
                .jsx_pragma
                .split('.')
                .next()
                .map(String::from)
                .unwrap()
        } else {
            options.jsx_pragma.to_string()
        };

        let jsx_fragment_import_name = if options.jsx_pragma_frag.contains('.') {
            options
                .jsx_pragma_frag
                .split('.')
                .next()
                .map(String::from)
                .unwrap()
        } else {
            options.jsx_pragma_frag.to_string()
        };

        Self {
            ctx,
            only_remove_type_imports: options.only_remove_type_imports,
            has_jsx_element: false,
            has_jsx_fragment: false,
            jsx_element_import_name,
            jsx_fragment_import_name,
        }
    }

    /// Check if the given name is a JSX pragma or fragment pragma import
    fn is_jsx_imports(&self, name: &str) -> bool {
        (self.has_jsx_element && name == self.jsx_element_import_name)
            || (self.has_jsx_fragment && name == self.jsx_fragment_import_name)
    }
}

impl VisitMut for TypeScriptAnnotations<'_> {
    noop_visit_mut_type!();

    fn visit_mut_module(&mut self, module: &mut Module) {
        // Check if this is a .d.ts file
        // TODO: Need to check source_type or file extension
        // For now, process normally
        module.visit_mut_children_with(self);
    }

    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        items.visit_mut_children_with(self);

        // Remove type-only imports/exports
        items.retain_mut(|item| match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                if import_decl.type_only {
                    return false;
                }

                // Remove type-only specifiers
                import_decl.specifiers.retain(|spec| match spec {
                    ImportSpecifier::Named(named) => !named.is_type_only,
                    _ => true,
                });

                // Keep import if it has specifiers or is a side-effect import
                !import_decl.specifiers.is_empty() || import_decl.specifiers.is_empty()
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) => {
                if export.type_only {
                    return false;
                }

                // Remove type-only specifiers
                export.specifiers.retain(|spec| match spec {
                    ExportSpecifier::Named(named) => !named.is_type_only,
                    _ => true,
                });

                !export.specifiers.is_empty() || export.src.is_some()
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportAll(export)) => !export.type_only,
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
                // Remove TS-only declarations
                !matches!(
                    export.decl,
                    Decl::TsInterface(_)
                        | Decl::TsTypeAlias(_)
                        | Decl::TsEnum(_)
                        | Decl::TsModule(_)
                )
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(_)) => true,
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(_)) => true,
            ModuleItem::ModuleDecl(ModuleDecl::TsImportEquals(_))
            | ModuleItem::ModuleDecl(ModuleDecl::TsExportAssignment(_))
            | ModuleItem::ModuleDecl(ModuleDecl::TsNamespaceExport(_)) => {
                // These are handled by other visitors
                false
            }
            ModuleItem::Stmt(_) => true,
        });
    }

    fn visit_mut_class(&mut self, class: &mut Class) {
        // Remove type parameters and implements
        class.type_params = None;
        class.super_type_params = None;
        class.implements.clear();
        class.is_abstract = false;

        class.visit_mut_children_with(self);

        // Remove type-only members
        class.body.retain(|member| match member {
            ClassMember::Constructor(_) => true,
            ClassMember::Method(method) => method.function.body.is_some(),
            ClassMember::PrivateMethod(_) => true,
            ClassMember::ClassProp(prop) => {
                // Keep if not declare-only
                !prop.declare
            }
            ClassMember::PrivateProp(_) => true,
            ClassMember::TsIndexSignature(_) => false,
            ClassMember::StaticBlock(_) => true,
            ClassMember::AutoAccessor(_) => true,
            ClassMember::Empty(_) => false,
        });
    }

    fn visit_mut_class_method(&mut self, method: &mut ClassMethod) {
        method.accessibility = None;
        method.is_optional = false;
        method.is_override = false;
        method.visit_mut_children_with(self);
    }

    fn visit_mut_private_method(&mut self, method: &mut PrivateMethod) {
        method.accessibility = None;
        method.is_optional = false;
        method.is_override = false;
        method.visit_mut_children_with(self);
    }

    fn visit_mut_class_prop(&mut self, prop: &mut ClassProp) {
        prop.accessibility = None;
        prop.type_ann = None;
        prop.is_optional = false;
        prop.is_override = false;
        prop.readonly = false;
        prop.definite = false;
        prop.visit_mut_children_with(self);
    }

    fn visit_mut_private_prop(&mut self, prop: &mut PrivateProp) {
        prop.accessibility = None;
        prop.type_ann = None;
        prop.is_optional = false;
        prop.is_override = false;
        prop.readonly = false;
        prop.definite = false;
        prop.visit_mut_children_with(self);
    }

    fn visit_mut_auto_accessor(&mut self, accessor: &mut AutoAccessor) {
        accessor.accessibility = None;
        accessor.type_ann = None;
        accessor.visit_mut_children_with(self);
    }

    fn visit_mut_constructor(&mut self, constructor: &mut Constructor) {
        constructor.accessibility = None;
        constructor.visit_mut_children_with(self);
    }

    fn visit_mut_function(&mut self, func: &mut Function) {
        func.type_params = None;
        func.return_type = None;
        func.visit_mut_children_with(self);
    }

    fn visit_mut_arrow_expr(&mut self, arrow: &mut ArrowExpr) {
        arrow.type_params = None;
        arrow.return_type = None;
        arrow.visit_mut_children_with(self);
    }

    fn visit_mut_param(&mut self, param: &mut Param) {
        param.pat.visit_mut_with(self);
    }

    fn visit_mut_pat(&mut self, pat: &mut Pat) {
        // Remove type annotations from patterns
        match pat {
            Pat::Ident(ident) => {
                ident.type_ann = None;
                ident.optional = false;
            }
            Pat::Array(array) => {
                array.type_ann = None;
                array.optional = false;
                array.visit_mut_children_with(self);
            }
            Pat::Object(obj) => {
                obj.type_ann = None;
                obj.optional = false;
                obj.visit_mut_children_with(self);
            }
            Pat::Rest(rest) => {
                rest.type_ann = None;
                rest.visit_mut_children_with(self);
            }
            Pat::Assign(assign) => {
                assign.visit_mut_children_with(self);
            }
            Pat::Invalid(_) | Pat::Expr(_) => {}
        }
    }

    fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
        declarator.definite = false;
        declarator.visit_mut_children_with(self);
    }

    fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
        call.type_args = None;
        call.visit_mut_children_with(self);
    }

    fn visit_mut_new_expr(&mut self, new: &mut NewExpr) {
        new.type_args = None;
        new.visit_mut_children_with(self);
    }

    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        // Remove TypeScript-specific expressions
        match expr {
            Expr::TsTypeAssertion(type_assertion) => {
                *expr = *type_assertion.expr.take();
                expr.visit_mut_with(self);
            }
            Expr::TsConstAssertion(const_assertion) => {
                *expr = *const_assertion.expr.take();
                expr.visit_mut_with(self);
            }
            Expr::TsNonNull(non_null) => {
                *expr = *non_null.expr.take();
                expr.visit_mut_with(self);
            }
            Expr::TsAs(as_expr) => {
                *expr = *as_expr.expr.take();
                expr.visit_mut_with(self);
            }
            Expr::TsInstantiation(inst) => {
                *expr = *inst.expr.take();
                expr.visit_mut_with(self);
            }
            Expr::TsSatisfies(satisfies) => {
                *expr = *satisfies.expr.take();
                expr.visit_mut_with(self);
            }
            _ => expr.visit_mut_children_with(self),
        }
    }

    fn visit_mut_jsx_element(&mut self, elem: &mut JSXElement) {
        self.has_jsx_element = true;
        elem.visit_mut_children_with(self);
    }

    fn visit_mut_jsx_fragment(&mut self, frag: &mut JSXFragment) {
        self.has_jsx_fragment = true;
        frag.visit_mut_children_with(self);
    }

    fn visit_mut_jsx_opening_element(&mut self, elem: &mut JSXOpeningElement) {
        elem.type_args = None;
        elem.visit_mut_children_with(self);
    }

    fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
        stmts.visit_mut_children_with(self);

        // Remove declare statements and TS-only statements
        stmts.retain(|stmt| match stmt {
            Stmt::Decl(decl) => match decl {
                Decl::TsInterface(_)
                | Decl::TsTypeAlias(_)
                | Decl::TsEnum(_)
                | Decl::TsModule(_) => false,
                Decl::Class(class) => !class.declare,
                Decl::Fn(func) => !func.declare,
                Decl::Var(var) => !var.declare,
                _ => true,
            },
            _ => true,
        });
    }
}
