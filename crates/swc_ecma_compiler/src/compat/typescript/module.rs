#![allow(dead_code)]
use swc_common::{util::take::Take, SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_visit::{noop_visit_mut_type, VisitMut, VisitMutWith};

use super::diagnostics;
use crate::compat::context::TransformCtx;

/// TypeScript module transformation
///
/// Handles:
/// - Export assignments (`export = expression`)
/// - Import equals declarations (`import x = require('mod')`)
pub struct TypeScriptModule<'a> {
    /// <https://babeljs.io/docs/babel-plugin-transform-typescript#onlyremovetypeimports>
    only_remove_type_imports: bool,
    ctx: &'a TransformCtx,
    is_esm: bool,
}

impl<'a> TypeScriptModule<'a> {
    pub fn new(only_remove_type_imports: bool, ctx: &'a TransformCtx, is_esm: bool) -> Self {
        Self {
            only_remove_type_imports,
            ctx,
            is_esm,
        }
    }
}

impl VisitMut for TypeScriptModule<'_> {
    noop_visit_mut_type!();

    fn visit_mut_module(&mut self, module: &mut Module) {
        for item in &mut module.body {
            item.visit_mut_with(self);
        }

        // In Babel, it will insert `use strict` in `@babel/transform-modules-commonjs`
        // plugin. Once we have a commonjs plugin, we can consider moving this
        // logic there.
        if !self.is_esm {
            let has_use_strict = module
                .body
                .iter()
                .any(|item| matches!(item, ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) if matches!(&**expr, Expr::Lit(Lit::Str(s)) if &*s.value == "use strict")));

            if !has_use_strict {
                module.body.insert(
                    0,
                    ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                        span: DUMMY_SP,
                        expr: Box::new(Expr::Lit(Lit::Str(Str {
                            span: DUMMY_SP,
                            value: "use strict".into(),
                            raw: None,
                        }))),
                    })),
                );
            }
        }
    }

    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        for item in items.iter_mut() {
            item.visit_mut_with(self);
        }

        // Transform export assignments and import equals
        items.retain_mut(|item| {
            match item {
                ModuleItem::ModuleDecl(ModuleDecl::TsExportAssignment(export_assignment)) => {
                    *item = self.transform_ts_export_assignment(export_assignment);
                    true
                }
                ModuleItem::ModuleDecl(ModuleDecl::TsImportEquals(import_equals)) => {
                    // Transform or remove import equals
                    if let Some(new_item) = self.transform_ts_import_equals(import_equals) {
                        *item = new_item;
                        true
                    } else {
                        false
                    }
                }
                _ => true,
            }
        });
    }
}

impl TypeScriptModule<'_> {
    /// Transform `export = expression` to `module.exports = expression`.
    fn transform_ts_export_assignment(
        &self,
        export_assignment: &mut TsExportAssignment,
    ) -> ModuleItem {
        if self.is_esm {
            self.ctx
                .error(diagnostics::export_assignment_cannot_be_used_in_esm());
        }

        // module.exports = expression
        let assignment_expr = Expr::Assign(AssignExpr {
            span: DUMMY_SP,
            op: op!("="),
            left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(Ident::new(
                    "module".into(),
                    DUMMY_SP,
                    SyntaxContext::empty(),
                ))),
                prop: MemberProp::Ident(IdentName::new("exports".into(), DUMMY_SP)),
            })),
            right: export_assignment.expr.take(),
        });

        ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(assignment_expr),
        }))
    }

    /// Transform TSImportEqualsDeclaration to a VariableDeclaration.
    ///
    /// ```TypeScript
    /// import module = require('module');
    /// import AliasModule = LongNameModule;
    /// ```
    ///
    /// ```JavaScript
    /// const module = require('module');
    /// const AliasModule = LongNameModule;
    /// ```
    fn transform_ts_import_equals(&self, decl: &mut TsImportEqualsDecl) -> Option<ModuleItem> {
        if decl.is_type_only {
            // Type-only imports are removed
            return None;
        }

        let kind = VarDeclKind::Const;
        let id = Pat::Ident(decl.id.clone().into());

        let init = match &mut decl.module_ref {
            TsModuleRef::TsEntityName(entity_name) => Self::transform_ts_entity_name(entity_name),
            TsModuleRef::TsExternalModuleRef(external_ref) => {
                if self.is_esm {
                    self.ctx
                        .error(diagnostics::import_equals_cannot_be_used_in_esm());
                }

                // require('module')
                let callee = Expr::Ident(Ident::new(
                    "require".into(),
                    DUMMY_SP,
                    SyntaxContext::empty(),
                ));
                Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    callee: Callee::Expr(Box::new(callee)),
                    args: vec![ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Lit(Lit::Str(external_ref.expr.clone()))),
                    }],
                    type_args: None,
                })
            }
        };

        let var_decl = VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: id,
                init: Some(Box::new(init)),
                definite: false,
            }],
        };

        Some(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(var_decl)))))
    }

    fn transform_ts_entity_name(entity_name: &TsEntityName) -> Expr {
        match entity_name {
            TsEntityName::TsQualifiedName(qualified_name) => {
                let obj = Box::new(Self::transform_ts_entity_name(&qualified_name.left));
                let prop = MemberProp::Ident(qualified_name.right.clone());
                Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj,
                    prop,
                })
            }
            TsEntityName::Ident(ident) => Expr::Ident(ident.clone()),
        }
    }
}
