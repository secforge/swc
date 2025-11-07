//! Emitting decorator metadata
//!
//! This plugin is used to emit decorator metadata for legacy decorators by
//! the `__metadata` helper.
//!
//! ## Example
//!
//! Input:
//! ```ts
//! class Demo {
//!   @LogMethod
//!   public foo(bar: number) {}
//!
//!   @Prop
//!   prop: string = "hello";
//! }
//! ```
//!
//! Output:
//! ```js
//! class Demo {
//!   foo(bar) {}
//!   prop = "hello";
//! }
//! babelHelpers.decorate([
//!   LogMethod,
//!   babelHelpers.decorateParam(0, babelHelpers.decorateMetadata("design:type", Function)),
//!   babelHelpers.decorateParam(0, babelHelpers.decorateMetadata("design:paramtypes", [Number])),
//!   babelHelpers.decorateParam(0, babelHelpers.decorateMetadata("design:returntype", void 0))
//! ], Demo.prototype, "foo", null);
//! babelHelpers.decorate([Prop, babelHelpers.decorateMetadata("design:type", String)], Demo.prototype, "prop", void 0);
//! ```
//!
//! ## Implementation
//!
//! This is a port of the oxc implementation based on
//! <https://github.com/microsoft/TypeScript/blob/d85767abfd83880cea17cea70f9913e9c4496dcc/src/compiler/transformers/ts.ts#L1119-L1136>
//!
//! ## Limitations
//!
//! ### Compared to TypeScript
//!
//! We lack the type inference ability that TypeScript has, so we cannot
//! determine the exact type of type references.
//!
//! ### Compared to SWC
//!
//! SWC also has the above limitation. SWC provides additional support for
//! inferring enum members, which we currently do not have.
//!
//! ## References
//! * TypeScript's [emitDecoratorMetadata](https://www.typescriptlang.org/tsconfig#emitDecoratorMetadata)

use std::collections::HashMap;

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

/// Type of an enum inferred from its members
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnumType {
    /// All members are string literals or template literals with string-only
    /// expressions
    String,
    /// All members are numeric, bigint, unary numeric, or auto-incremented
    Number,
    /// Mixed types or computed values
    Object,
}

/// Metadata for decorated methods
pub(super) struct MethodMetadata {
    /// The `design:type` metadata expression
    pub r#type: Box<Expr>,
    /// The `design:paramtypes` metadata expression
    pub param_types: Box<Expr>,
    /// The `design:returntype` metadata expression (optional, omitted for
    /// getters/setters)
    pub return_type: Option<Box<Expr>>,
}

/// Legacy decorator metadata transformer
///
/// Emits decorator metadata for TypeScript decorators
pub struct LegacyDecoratorMetadata<'ctx> {
    ctx: &'ctx TransformCtx,
    /// Stack of method metadata - pushed in enter, popped in exit
    method_metadata_stack: Vec<Option<MethodMetadata>>,
    /// Stack of constructor metadata - pushed in enter_class, popped when
    /// needed
    constructor_metadata_stack: Vec<Option<Box<Expr>>>,
    /// Map of enum symbol names to their inferred types
    enum_types: HashMap<Atom, EnumType>,
}

impl<'ctx> LegacyDecoratorMetadata<'ctx> {
    /// Create a new metadata transformer
    pub fn new(ctx: &'ctx TransformCtx) -> Self {
        Self {
            ctx,
            method_metadata_stack: Vec::new(),
            constructor_metadata_stack: Vec::new(),
            enum_types: HashMap::new(),
        }
    }

    /// Pop method metadata from the stack
    pub fn pop_method_metadata(&mut self) -> Option<MethodMetadata> {
        self.method_metadata_stack.pop().and_then(|x| x)
    }

    /// Pop constructor metadata from the stack
    pub fn pop_constructor_metadata(&mut self) -> Option<Box<Expr>> {
        self.constructor_metadata_stack.pop().and_then(|x| x)
    }

    /// Infer the type of an enum based on its members
    fn infer_enum_type(members: &[TsEnumMember]) -> EnumType {
        let mut enum_type = EnumType::Object;

        for member in members {
            if let Some(init) = &member.init {
                match &**init {
                    Expr::Lit(Lit::Str(_)) | Expr::Tpl(_) if enum_type != EnumType::Number => {
                        enum_type = EnumType::String;
                    }
                    // TS considers `+x`, `-x`, `~x` to be `Number` type
                    Expr::Lit(Lit::Num(_)) | Expr::Unary(_) if enum_type != EnumType::String => {
                        enum_type = EnumType::Number;
                    }
                    // For other expressions, we can't determine the type statically
                    _ => return EnumType::Object,
                }
            } else {
                // No initializer means numeric (auto-incrementing)
                if enum_type == EnumType::String {
                    return EnumType::Object;
                }
                enum_type = EnumType::Number;
            }
        }

        enum_type
    }

    /// Serialize a TypeScript type annotation for decorator metadata
    fn serialize_type_annotation(&mut self, type_ann: Option<&Box<TsTypeAnn>>) -> Box<Expr> {
        if let Some(type_ann) = type_ann {
            self.serialize_type_node(&type_ann.type_ann)
        } else {
            Self::global_object()
        }
    }

    /// Serialize a TypeScript type node for use with decorator metadata
    ///
    /// Types are serialized as follows:
    /// - Void types -> "undefined" (e.g. "void 0")
    /// - Function and Constructor types -> global "Function"
    /// - Array and Tuple types -> global "Array"
    /// - Boolean types and type predicates -> global "Boolean"
    /// - String literal types and strings -> global "String"
    /// - Enum and number types -> global "Number"
    /// - Symbol types -> global "Symbol"
    /// - Type references to classes -> constructor for the class
    /// - Everything else -> global "Object"
    fn serialize_type_node(&mut self, node: &TsType) -> Box<Expr> {
        match node {
            TsType::TsKeywordType(kw) => match kw.kind {
                TsKeywordTypeKind::TsVoidKeyword
                | TsKeywordTypeKind::TsUndefinedKeyword
                | TsKeywordTypeKind::TsNullKeyword
                | TsKeywordTypeKind::TsNeverKeyword => Self::void_0(),
                TsKeywordTypeKind::TsBooleanKeyword => Self::global_boolean(),
                TsKeywordTypeKind::TsStringKeyword => Self::global_string(),
                TsKeywordTypeKind::TsNumberKeyword => Self::global_number(),
                TsKeywordTypeKind::TsBigIntKeyword => Self::global_bigint(),
                TsKeywordTypeKind::TsSymbolKeyword => Self::global_symbol(),
                TsKeywordTypeKind::TsObjectKeyword
                | TsKeywordTypeKind::TsAnyKeyword
                | TsKeywordTypeKind::TsUnknownKeyword => Self::global_object(),
                _ => Self::global_object(),
            },
            TsType::TsFnOrConstructorType(_) => Self::global_function(),
            TsType::TsArrayType(_) | TsType::TsTupleType(_) => Self::global_array(),
            TsType::TsTypePredicate(pred) => {
                if pred.asserts {
                    Self::void_0()
                } else {
                    Self::global_boolean()
                }
            }
            TsType::TsLitType(lit) => Self::serialize_literal_of_literal_type_node(&lit.lit),
            TsType::TsTypeRef(type_ref) => self.serialize_type_reference_node(&type_ref.type_name),
            TsType::TsUnionOrIntersectionType(union_or_intersection) => match union_or_intersection
            {
                TsUnionOrIntersectionType::TsUnionType(union) => {
                    self.serialize_union_or_intersection_constituents(&union.types, false)
                }
                TsUnionOrIntersectionType::TsIntersectionType(intersection) => {
                    self.serialize_union_or_intersection_constituents(&intersection.types, true)
                }
            },
            TsType::TsConditionalType(cond) => self.serialize_union_or_intersection_constituents(
                &[cond.true_type.clone(), cond.false_type.clone()],
                false,
            ),
            TsType::TsTypeOperator(op) if matches!(op.op, TsTypeOperatorOp::ReadOnly) => {
                self.serialize_type_node(&op.type_ann)
            }
            TsType::TsParenthesizedType(paren) => self.serialize_type_node(&paren.type_ann),
            // Fallback to Object for complex types
            _ => Self::global_object(),
        }
    }

    /// Serialize the literal value of a literal type node
    fn serialize_literal_of_literal_type_node(literal: &TsLit) -> Box<Expr> {
        match literal {
            TsLit::Bool(_) => Self::global_boolean(),
            TsLit::Number(_) => Self::global_number(),
            TsLit::BigInt(_) => Self::global_bigint(),
            TsLit::Str(_) | TsLit::Tpl(_) => Self::global_string(),
        }
    }

    /// Serialize a type reference node
    fn serialize_type_reference_node(&mut self, type_name: &TsEntityName) -> Box<Expr> {
        // Check if this is an enum type reference
        if let TsEntityName::Ident(ident) = type_name {
            if let Some(enum_type) = self.enum_types.get(&ident.sym) {
                return match enum_type {
                    EnumType::String => Self::global_string(),
                    EnumType::Number => Self::global_number(),
                    EnumType::Object => Self::global_object(),
                };
            }
        }

        // For non-enum type references, we need to check at runtime if they're
        // functions `typeof T === "function" ? T : Object`
        let serialized_type = self.serialize_entity_name_as_expression(type_name);

        // Create: typeof serialized_type === "function" ? serialized_type : Object
        let type_of = Box::new(Expr::Unary(UnaryExpr {
            span: DUMMY_SP,
            op: op!("typeof"),
            arg: serialized_type.clone(),
        }));

        let test = Box::new(Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: op!("==="),
            left: type_of,
            right: Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: "function".into(),
                raw: None,
            }))),
        }));

        Box::new(Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test,
            cons: serialized_type,
            alt: Self::global_object(),
        }))
    }

    /// Serialize entity name as an expression with runtime checks
    fn serialize_entity_name_as_expression(&self, name: &TsEntityName) -> Box<Expr> {
        match name {
            TsEntityName::Ident(ident) => {
                // `typeof A !== "undefined" && A`
                let ident1 = Box::new(Expr::Ident(ident.clone()));
                let ident2 = Box::new(Expr::Ident(ident.clone()));
                Self::create_checked_value(ident1, ident2)
            }
            TsEntityName::TsQualifiedName(qualified) => {
                // `typeof A !== "undefined" && A.B`
                let left_expr = self.serialize_entity_name_as_expression(&qualified.left);
                let member = Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: left_expr.clone(),
                    prop: MemberProp::Ident(IdentName {
                        span: DUMMY_SP,
                        sym: qualified.right.sym.clone(),
                    }),
                }));
                Self::create_checked_value(left_expr, member)
            }
        }
    }

    /// Serialize union or intersection type constituents
    fn serialize_union_or_intersection_constituents(
        &mut self,
        types: &[Box<TsType>],
        is_intersection: bool,
    ) -> Box<Expr> {
        let mut serialized_type: Option<Box<Expr>> = None;

        for t in types {
            match &**t {
                TsType::TsKeywordType(kw) => match kw.kind {
                    TsKeywordTypeKind::TsNeverKeyword => {
                        if is_intersection {
                            // Reduce to `never` in an intersection
                            return Self::void_0();
                        }
                        // Elide `never` in a union
                        continue;
                    }
                    TsKeywordTypeKind::TsUnknownKeyword => {
                        if !is_intersection {
                            // Reduce to `unknown` in a union
                            return Self::global_object();
                        }
                        // Elide `unknown` in an intersection
                        continue;
                    }
                    TsKeywordTypeKind::TsAnyKeyword => {
                        return Self::global_object();
                    }
                    _ => {}
                },
                TsType::TsTypeRef(_) => {
                    // Type references always result in Object
                    return Self::global_object();
                }
                _ => {}
            }

            let serialized_constituent = self.serialize_type_node(t);

            // Check if this is the global Object identifier
            if Self::is_global_object(&serialized_constituent) {
                return serialized_constituent;
            }

            // Check if types are compatible
            if let Some(ref existing) = serialized_type {
                if !Self::equate_serialized_type_nodes(existing, &serialized_constituent) {
                    return Self::global_object();
                }
            } else {
                serialized_type = Some(serialized_constituent);
            }
        }

        serialized_type.unwrap_or_else(Self::void_0)
    }

    /// Compare two serialized type nodes for equality
    fn equate_serialized_type_nodes(a: &Expr, b: &Expr) -> bool {
        // Simple structural comparison
        match (a, b) {
            (Expr::Ident(id1), Expr::Ident(id2)) => id1.sym == id2.sym,
            _ => false,
        }
    }

    /// Check if an expression is the global Object identifier
    fn is_global_object(expr: &Expr) -> bool {
        matches!(expr, Expr::Ident(id) if id.sym == "Object")
    }

    /// Serialize parameter types of a function
    fn serialize_parameters_types(&mut self, params: &[Param]) -> Box<Expr> {
        let elements = params
            .iter()
            .map(|param| {
                let type_ann = match &param.pat {
                    Pat::Assign(assign) => match &*assign.left {
                        Pat::Ident(ident) => ident.type_ann.as_ref(),
                        _ => None,
                    },
                    Pat::Ident(ident) => ident.type_ann.as_ref(),
                    _ => None,
                };
                Some(ExprOrSpread {
                    spread: None,
                    expr: self.serialize_type_annotation(type_ann),
                })
            })
            .collect();

        Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: elements,
        }))
    }

    /// Serialize return type of a function
    fn serialize_return_type(&mut self, func: &Function) -> Box<Expr> {
        if func.is_async {
            Self::global_promise()
        } else if let Some(return_type) = &func.return_type {
            self.serialize_type_node(&return_type.type_ann)
        } else {
            Self::void_0()
        }
    }

    /// Create a runtime checked value expression
    /// `typeof left !== "undefined" && right`
    fn create_checked_value(left: Box<Expr>, right: Box<Expr>) -> Box<Expr> {
        let type_of = Box::new(Expr::Unary(UnaryExpr {
            span: DUMMY_SP,
            op: op!("typeof"),
            arg: left,
        }));

        let undefined = Box::new(Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: "undefined".into(),
            raw: None,
        })));

        let left_check = Box::new(Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: op!("!=="),
            left: type_of,
            right: undefined,
        }));

        Box::new(Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: op!("&&"),
            left: left_check,
            right,
        }))
    }

    /// Create a metadata decorator call
    fn create_metadata(&self, key: &str, value: Box<Expr>) -> Box<Expr> {
        // TODO: Use helper_call_expr when available
        // For now, create a simple call expression
        Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                "_metadata".into(),
                DUMMY_SP,
                Default::default(),
            )))),
            args: vec![
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                        span: DUMMY_SP,
                        value: key.into(),
                        raw: None,
                    }))),
                },
                ExprOrSpread {
                    spread: None,
                    expr: value,
                },
            ],
            ..Default::default()
        }))
    }

    /// Create a metadata decorator
    fn create_metadata_decorator(&self, key: &str, value: Box<Expr>) -> Decorator {
        Decorator {
            span: DUMMY_SP,
            expr: self.create_metadata(key, value),
        }
    }

    // Helper methods to create global identifier expressions
    fn void_0() -> Box<Expr> {
        Box::new(Expr::Unary(UnaryExpr {
            span: DUMMY_SP,
            op: op!("void"),
            arg: Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: 0.0,
                raw: None,
            }))),
        }))
    }

    fn global_object() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "Object".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }

    fn global_function() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "Function".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }

    fn global_array() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "Array".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }

    fn global_boolean() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "Boolean".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }

    fn global_string() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "String".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }

    fn global_number() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "Number".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }

    fn global_bigint() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "BigInt".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }

    fn global_symbol() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "Symbol".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }

    fn global_promise() -> Box<Expr> {
        Box::new(Expr::Ident(Ident::new(
            "Promise".into(),
            DUMMY_SP,
            Default::default(),
        )))
    }
}

impl VisitMutHook for LegacyDecoratorMetadata<'_> {
    fn enter_stmt(&mut self, stmt: &mut Stmt) {
        // Collect enum types
        if let Stmt::Decl(Decl::TsEnum(decl)) = stmt {
            let enum_type = Self::infer_enum_type(&decl.members);
            self.enum_types.insert(decl.id.sym.clone(), enum_type);
        }
    }

    fn enter_class(&mut self, class: &mut Class) {
        // Check if we need to generate constructor metadata
        let constructor = class.body.iter().find_map(|member| {
            if let ClassMember::Constructor(ctor) = member {
                Some(ctor)
            } else {
                None
            }
        });

        let metadata = if let Some(constructor) = constructor {
            if !class.decorators.is_empty()
                || constructor
                    .params
                    .iter()
                    .any(|p| matches!(p, ParamOrTsParamProp::Param(param) if !param.decorators.is_empty()))
            {
                let params: Vec<_> = constructor
                    .params
                    .iter()
                    .filter_map(|p| match p {
                        ParamOrTsParamProp::Param(param) => Some(param.clone()),
                        _ => None,
                    })
                    .collect();
                let serialized = self.serialize_parameters_types(&params);
                Some(self.create_metadata("design:paramtypes", serialized))
            } else {
                None
            }
        } else {
            None
        };

        self.constructor_metadata_stack.push(metadata);
    }

    fn enter_class_method(&mut self, method: &mut ClassMethod) {
        // Note: In SWC, constructors are ClassMember::Constructor, not ClassMethod
        // So this method only handles regular methods

        let is_decorated = !method.function.decorators.is_empty()
            || method
                .function
                .params
                .iter()
                .any(|param| !param.decorators.is_empty());

        let metadata = if is_decorated {
            let (design_type, return_type) = match method.kind {
                MethodKind::Getter => {
                    // For getters, design type is the return type
                    (self.serialize_return_type(&method.function), None)
                }
                MethodKind::Setter => {
                    // For setters, design type is the first parameter type
                    let type_expr = method
                        .function
                        .params
                        .first()
                        .and_then(|p| match &p.pat {
                            Pat::Ident(ident) => ident.type_ann.as_ref(),
                            _ => None,
                        })
                        .map(|ann| self.serialize_type_node(&ann.type_ann))
                        .unwrap_or_else(Self::global_object);
                    (type_expr, None)
                }
                _ => {
                    // For methods, design type is always Function
                    (
                        Self::global_function(),
                        Some(self.serialize_return_type(&method.function)),
                    )
                }
            };

            let param_types = self.serialize_parameters_types(&method.function.params);

            Some(MethodMetadata {
                r#type: self.create_metadata("design:type", design_type),
                param_types: self.create_metadata("design:paramtypes", param_types),
                return_type: return_type.map(|t| self.create_metadata("design:returntype", t)),
            })
        } else {
            None
        };

        self.method_metadata_stack.push(metadata);
    }

    fn enter_class_prop(&mut self, prop: &mut ClassProp) {
        if !prop.decorators.is_empty() {
            let serialized_type = self.serialize_type_annotation(prop.type_ann.as_ref());
            let decorator = self.create_metadata_decorator("design:type", serialized_type);
            prop.decorators.push(decorator);
        }
    }

    fn enter_private_prop(&mut self, prop: &mut PrivateProp) {
        if !prop.decorators.is_empty() {
            let serialized_type = self.serialize_type_annotation(prop.type_ann.as_ref());
            let decorator = self.create_metadata_decorator("design:type", serialized_type);
            prop.decorators.push(decorator);
        }
    }
}
