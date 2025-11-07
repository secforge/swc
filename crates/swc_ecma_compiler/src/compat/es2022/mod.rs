use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

mod class_properties;
mod class_static_block;
mod options;

use class_properties::ClassProperties;
pub use class_properties::ClassPropertiesOptions;
use class_static_block::ClassStaticBlock;
pub use options::ES2022Options;

/// ES2022 transformer.
///
/// Transforms ES2022 features to older JavaScript versions.
pub struct ES2022<'ctx> {
    // Plugins
    class_static_block: Option<ClassStaticBlock>,
    class_properties: Option<ClassProperties<'ctx>>,
}

impl<'ctx> ES2022<'ctx> {
    /// Create a new ES2022 transformer.
    #[allow(dead_code)]
    pub fn new(
        options: ES2022Options,
        remove_class_fields_without_initializer: bool,
        ctx: &'ctx TransformCtx,
    ) -> Self {
        // Class properties transform performs the static block transform differently.
        // So only enable static block transform if class properties transform is
        // disabled.
        let (class_static_block, class_properties) =
            if let Some(properties_options) = options.class_properties {
                let class_properties = ClassProperties::new(
                    properties_options,
                    options.class_static_block,
                    remove_class_fields_without_initializer,
                    ctx,
                );
                (None, Some(class_properties))
            } else {
                let class_static_block = if options.class_static_block {
                    Some(ClassStaticBlock::new())
                } else {
                    None
                };
                (class_static_block, None)
            };
        Self {
            class_static_block,
            class_properties,
        }
    }
}

impl VisitMutHook for ES2022<'_> {
    #[inline]
    fn exit_program(&mut self, program: &mut Program) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.exit_program(program);
        }
    }

    fn enter_expr(&mut self, expr: &mut Expr) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.enter_expr(expr);
        }
    }

    fn exit_expr(&mut self, expr: &mut Expr) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.exit_expr(expr);
        }
    }

    fn enter_class(&mut self, class: &mut Class) {
        match &mut self.class_properties {
            Some(class_properties) => {
                class_properties.enter_class(class);
            }
            _ => {
                if let Some(class_static_block) = &mut self.class_static_block {
                    class_static_block.enter_class(class);
                }
            }
        }
    }

    fn exit_class(&mut self, class: &mut Class) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.exit_class(class);
        }
    }

    fn enter_assign_target(&mut self, target: &mut AssignTarget) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.enter_assign_target(target);
        }
    }

    fn enter_class_prop(&mut self, prop: &mut ClassProp) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.enter_class_prop(prop);
        }
    }

    fn exit_class_prop(&mut self, prop: &mut ClassProp) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.exit_class_prop(prop);
        }
    }

    fn enter_static_block(&mut self, block: &mut StaticBlock) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.enter_static_block(block);
        }
    }

    fn exit_static_block(&mut self, block: &mut StaticBlock) {
        if let Some(class_properties) = &mut self.class_properties {
            class_properties.exit_static_block(block);
        }
    }

    fn enter_await_expr(&mut self, _node: &mut AwaitExpr) {
        // TODO: Implement top-level await warning when needed
        // Note: This would require context to determine if we're at the top
        // level and access to a diagnostics system.
    }
}
