//! Legacy decorator transformation
//!
//! This module transforms legacy decorators by calling `_decorate` and
//! `_decorateParam` helpers to apply decorators.
//!
//! ## Examples
//!
//! Input:
//! ```ts
//! @dec
//! class Class {
//!   @dec
//!   prop = 0;
//!
//!   @dec
//!   method(@dec param) {}
//! }
//! ```
//!
//! Output:
//! ```js
//! let Class = class Class {
//!   prop = 0;
//!   method(param) {}
//! };
//!
//! _decorate([dec], Class.prototype, "method", null);
//!
//! _decorate([
//!   _decorateParam(0, dec)
//! ], Class.prototype, "method", null);
//!
//! Class = _decorate([dec], Class);
//! ```
//!
//! ## Implementation
//!
//! This is a port of the oxc legacy decorator implementation to SWC.
//! The original implementation is based on [TypeScript Experimental Decorators](https://github.com/microsoft/TypeScript/blob/d85767abfd83880cea17cea70f9913e9c4496dcc/src/compiler/transformers/legacyDecorators.ts).
//!
//! ## References
//! * TypeScript Experimental Decorators documentation: <https://www.typescriptlang.org/docs/handbook/decorators.html>

mod metadata;

use std::mem;

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use self::metadata::LegacyDecoratorMetadata;
use crate::compat::context::TransformCtx;

/// Binding information for a decorated class
struct ClassBindingInfo {
    /// The class binding name
    name: Atom,
    /// Whether the class has an alias (for self-references inside the class
    /// body)
    has_alias: bool,
}

/// Decoration state for the current class being processed
#[derive(Default)]
struct ClassDecorations {
    /// Whether the current class should be transformed
    should_transform: bool,
    /// Decoration statements accumulated for the current class
    decoration_stmts: Vec<Stmt>,
    /// Class binding info for the current class
    class_binding: Option<ClassBindingInfo>,
    /// Whether any decorator contains a private-in expression
    has_private_in_expression: bool,
}

/// Legacy decorator transformer
///
/// Handles transformation of legacy (experimental) decorators
pub struct LegacyDecorator<'ctx> {
    emit_decorator_metadata: bool,
    metadata: LegacyDecoratorMetadata<'ctx>,
    /// Stack of class decoration state (for nested classes)
    class_decorations_stack: Vec<ClassDecorations>,
    /// Pending statements to inject after the current statement
    pending_statements: Vec<Stmt>,
    ctx: &'ctx TransformCtx,
}

impl<'ctx> LegacyDecorator<'ctx> {
    /// Create a new legacy decorator transformer
    pub fn new(emit_decorator_metadata: bool, ctx: &'ctx TransformCtx) -> Self {
        Self {
            emit_decorator_metadata,
            metadata: LegacyDecoratorMetadata::new(ctx),
            class_decorations_stack: vec![ClassDecorations::default()],
            pending_statements: Vec::new(),
            ctx,
        }
    }

    /// Check if a class or its constructor has decorators
    fn class_has_decorators(class: &Class) -> bool {
        if !class.decorators.is_empty() {
            return true;
        }

        class.body.iter().any(|member| match member {
            ClassMember::Constructor(ctor) => ctor.params.iter().any(
                |param| matches!(param, ParamOrTsParamProp::Param(p) if !p.decorators.is_empty()),
            ),
            _ => false,
        })
    }

    /// Check if class has decorated members (properties/methods)
    fn has_decorated_members(class: &Class) -> bool {
        class.body.iter().any(|member| match member {
            ClassMember::Method(method) => {
                !method.function.decorators.is_empty()
                    || method
                        .function
                        .params
                        .iter()
                        .any(|p| !p.decorators.is_empty())
            }
            ClassMember::ClassProp(prop) => !prop.decorators.is_empty(),
            ClassMember::PrivateProp(prop) => !prop.decorators.is_empty(),
            ClassMember::AutoAccessor(accessor) => !accessor.decorators.is_empty(),
            _ => false,
        })
    }

    /// Get class member prefix expression (`Class` for static,
    /// `Class.prototype` for instance)
    fn get_class_member_prefix(class_name: &Atom, is_static: bool) -> Box<Expr> {
        let class_ident = Box::new(Expr::Ident(Ident::new(
            class_name.clone(),
            DUMMY_SP,
            Default::default(),
        )));

        if is_static {
            class_ident
        } else {
            // Class.prototype
            Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: class_ident,
                prop: MemberProp::Ident(IdentName::new("prototype".into(), DUMMY_SP)),
            }))
        }
    }

    /// Convert decorators to an array expression
    fn decorators_to_array(decorators: Vec<Decorator>) -> Box<Expr> {
        let elements = decorators
            .into_iter()
            .map(|dec| {
                Some(ExprOrSpread {
                    spread: None,
                    expr: dec.expr,
                })
            })
            .collect();

        Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: elements,
        }))
    }

    /// Get property key name as an expression
    fn get_property_key_name(key: &PropName) -> Box<Expr> {
        match key {
            PropName::Ident(ident) => Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: ident.sym.clone().into(),
                raw: None,
            }))),
            PropName::Str(s) => Box::new(Expr::Lit(Lit::Str(s.clone()))),
            PropName::Num(n) => Box::new(Expr::Lit(Lit::Num(n.clone()))),
            PropName::Computed(computed) => computed.expr.clone(),
            PropName::BigInt(big) => Box::new(Expr::Lit(Lit::BigInt(big.clone()))),
        }
    }

    /// Create a `_decorate` helper call statement
    fn create_decorate_call(
        &self,
        decorations: Box<Expr>,
        target: Box<Expr>,
        key: Box<Expr>,
        descriptor: Box<Expr>,
    ) -> Stmt {
        // TODO: Use helper_call_expr when available
        let call = Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                "_decorate".into(),
                DUMMY_SP,
                Default::default(),
            )))),
            args: vec![
                ExprOrSpread {
                    spread: None,
                    expr: decorations,
                },
                ExprOrSpread {
                    spread: None,
                    expr: target,
                },
                ExprOrSpread {
                    spread: None,
                    expr: key,
                },
                ExprOrSpread {
                    spread: None,
                    expr: descriptor,
                },
            ],
            ..Default::default()
        }));

        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: call,
        })
    }

    /// Create a class decorator assignment statement
    fn create_class_decorator_assignment(&self, class_name: &Atom, decorations: Box<Expr>) -> Stmt {
        // Class = _decorate([decorators], Class)
        let decorate_call = Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                "_decorate".into(),
                DUMMY_SP,
                Default::default(),
            )))),
            args: vec![
                ExprOrSpread {
                    spread: None,
                    expr: decorations,
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Ident(Ident::new(
                        class_name.clone(),
                        DUMMY_SP,
                        Default::default(),
                    ))),
                },
            ],
            ..Default::default()
        }));

        let assignment = Box::new(Expr::Assign(AssignExpr {
            span: DUMMY_SP,
            op: op!("="),
            left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent {
                id: Ident::new(class_name.clone(), DUMMY_SP, Default::default()),
                type_ann: None,
            })),
            right: decorate_call,
        }));

        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: assignment,
        })
    }

    /// Transform parameter decorators to `_decorateParam` calls
    fn transform_param_decorators(&self, params: &mut [Param]) -> Vec<ExprOrSpread> {
        let mut param_decorations = Vec::new();

        for (index, param) in params.iter_mut().enumerate() {
            if param.decorators.is_empty() {
                continue;
            }

            for decorator in param.decorators.drain(..) {
                // _decorateParam(index, decorator)
                let param_decorator_call = Box::new(Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                        "_decorateParam".into(),
                        DUMMY_SP,
                        Default::default(),
                    )))),
                    args: vec![
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: index as f64,
                                raw: None,
                            }))),
                        },
                        ExprOrSpread {
                            spread: None,
                            expr: decorator.expr,
                        },
                    ],
                    ..Default::default()
                }));

                param_decorations.push(ExprOrSpread {
                    spread: None,
                    expr: param_decorator_call,
                });
            }
        }

        param_decorations
    }

    /// Get all decorators of a method (including parameter decorators)
    fn get_all_method_decorators(&mut self, method: &mut ClassMethod) -> Option<Box<Expr>> {
        let method_decorators = mem::take(&mut method.function.decorators);
        let param_decorators = self.transform_param_decorators(&mut method.function.params);

        if method_decorators.is_empty() && param_decorators.is_empty() {
            if self.emit_decorator_metadata {
                // Pop metadata even if no decorators
                // Note: ClassMethod is never a constructor in SWC (constructors are ClassMember::Constructor)
                self.metadata.pop_method_metadata();
            }
            return None;
        }

        let mut all_decorators = Vec::new();

        // Method decorators come first
        all_decorators.extend(method_decorators.into_iter().map(|dec| ExprOrSpread {
            spread: None,
            expr: dec.expr,
        }));

        // Then parameter decorators
        all_decorators.extend(param_decorators);

        // Then metadata (if enabled)
        if self.emit_decorator_metadata {
            // Note: ClassMethod is never a constructor in SWC (constructors are ClassMember::Constructor)
            if let Some(metadata) = self.metadata.pop_method_metadata() {
                all_decorators.push(ExprOrSpread {
                    spread: None,
                    expr: metadata.r#type,
                });
                all_decorators.push(ExprOrSpread {
                    spread: None,
                    expr: metadata.param_types,
                });
                if let Some(return_type) = metadata.return_type {
                    all_decorators.push(ExprOrSpread {
                        spread: None,
                        expr: return_type,
                    });
                }
            }
        }

        Some(Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: all_decorators.into_iter().map(Some).collect(),
        })))
    }

    /// Handle decorated class member (method, property, accessor)
    fn handle_decorated_class_member(
        &mut self,
        class_name: &Atom,
        is_static: bool,
        key: &PropName,
        decorators: Vec<Decorator>,
        descriptor: Box<Expr>,
    ) {
        let current_class = self.class_decorations_stack.last_mut().unwrap();

        if !current_class.should_transform {
            return;
        }

        let decorations = Self::decorators_to_array(decorators);
        let target = Self::get_class_member_prefix(class_name, is_static);
        let key_expr = Self::get_property_key_name(key);

        let stmt = self.create_decorate_call(decorations, target, key_expr, descriptor);
        current_class.decoration_stmts.push(stmt);
    }

    /// Transform a class declaration with decorators
    fn transform_decorated_class(&mut self, class: &mut Class) -> Option<Stmt> {
        let current_decorations = self.class_decorations_stack.pop().unwrap();

        if !current_decorations.should_transform {
            return None;
        }

        let has_class_decorators = Self::class_has_decorators(class);
        let has_member_decorators = !current_decorations.decoration_stmts.is_empty();

        if !has_class_decorators && !has_member_decorators {
            return None;
        }

        // Get or generate class name
        let class_name = if let Some(ident) = &class.ident {
            ident.sym.clone()
        } else {
            "_class".into()
        };

        // Collect member decoration statements
        let mut stmts = current_decorations.decoration_stmts;

        // Handle class-level decorators
        if has_class_decorators {
            let decorations = Self::decorators_to_array(mem::take(&mut class.decorators));
            let stmt = self.create_class_decorator_assignment(&class_name, decorations);
            stmts.push(stmt);
        }

        // Store statements to inject after the class
        self.pending_statements.extend(stmts);

        // Transform class to variable declaration if it has class decorators
        if has_class_decorators {
            let class_expr = Expr::Class(ClassExpr {
                ident: class.ident.clone(),
                class: Box::new(class.clone()),
            });

            Some(Stmt::Decl(Decl::Var(Box::new(VarDecl {
                span: DUMMY_SP,
                kind: VarDeclKind::Let,
                declare: false,
                decls: vec![VarDeclarator {
                    span: DUMMY_SP,
                    name: Pat::Ident(BindingIdent {
                        id: Ident::new(class_name, DUMMY_SP, Default::default()),
                        type_ann: None,
                    }),
                    init: Some(Box::new(class_expr)),
                    definite: false,
                }],
                ..Default::default()
            }))))
        } else {
            None
        }
    }
}

impl VisitMutHook for LegacyDecorator<'_> {
    fn enter_stmt(&mut self, stmt: &mut Stmt) {
        if self.emit_decorator_metadata {
            self.metadata.enter_stmt(stmt);
        }
    }

    fn enter_class(&mut self, class: &mut Class) {
        // Push new class decoration state
        let should_transform = !matches!(class.ident, None) && !class.is_abstract;
        self.class_decorations_stack.push(ClassDecorations {
            should_transform,
            ..Default::default()
        });

        if self.emit_decorator_metadata {
            self.metadata.enter_class(class);
        }
    }

    fn exit_class(&mut self, class: &mut Class) {
        // Note: Actual transformation happens in exit_stmt for class declarations
        if self.emit_decorator_metadata {
            // Metadata stacks need to be balanced
        }
    }

    fn enter_class_method(&mut self, method: &mut ClassMethod) {
        if self.emit_decorator_metadata {
            self.metadata.enter_class_method(method);
        }
    }

    fn exit_class_method(&mut self, method: &mut ClassMethod) {
        // Note: In SWC, constructors are ClassMember::Constructor, not ClassMethod
        // So this method only handles regular methods

        let current_class = self.class_decorations_stack.last().unwrap();
        if !current_class.should_transform {
            return;
        }

        let class_name = current_class
            .class_binding
            .as_ref()
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "_class".into());

        if let Some(decorations) = self.get_all_method_decorators(method) {
            // null descriptor means get from Object.getOwnPropertyDescriptor
            let descriptor = Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })));

            let target = Self::get_class_member_prefix(&class_name, method.is_static);
            let key = Self::get_property_key_name(&method.key);

            let stmt = self.create_decorate_call(decorations, target, key, descriptor);

            // Need to get mutable reference again
            let current_class = self.class_decorations_stack.last_mut().unwrap();
            current_class.decoration_stmts.push(stmt);
        }
    }

    fn exit_class_prop(&mut self, prop: &mut ClassProp) {
        if prop.decorators.is_empty() {
            return;
        }

        let current_class = self.class_decorations_stack.last().unwrap();
        if !current_class.should_transform {
            return;
        }

        let class_name = current_class
            .class_binding
            .as_ref()
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "_class".into());

        let decorators = mem::take(&mut prop.decorators);

        // void 0 descriptor means use Object.defineProperty
        let descriptor = Box::new(Expr::Unary(UnaryExpr {
            span: DUMMY_SP,
            op: op!("void"),
            arg: Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: 0.0,
                raw: None,
            }))),
        }));

        self.handle_decorated_class_member(
            &class_name,
            prop.is_static,
            &prop.key,
            decorators,
            descriptor,
        );
    }

    fn enter_class_prop(&mut self, prop: &mut ClassProp) {
        if self.emit_decorator_metadata {
            self.metadata.enter_class_prop(prop);
        }
    }

    fn enter_private_prop(&mut self, prop: &mut PrivateProp) {
        if self.emit_decorator_metadata {
            self.metadata.enter_private_prop(prop);
        }
    }

    fn exit_private_prop(&mut self, prop: &mut PrivateProp) {
        // Private props with decorators - similar to class props
        if prop.decorators.is_empty() {
            return;
        }

        // Legacy decorators don't really support private props, but handle them anyway
        prop.decorators.clear();
    }
}
