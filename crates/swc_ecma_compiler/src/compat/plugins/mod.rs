mod options;
mod styled_components;

pub use options::PluginsOptions;
use styled_components::StyledComponents;
pub use styled_components::StyledComponentsOptions;
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::context::TransformCtx;

pub struct Plugins<'ctx> {
    styled_components: Option<StyledComponents<'ctx>>,
}

impl<'ctx> Plugins<'ctx> {
    pub fn new(options: PluginsOptions, ctx: &'ctx TransformCtx) -> Self {
        Self {
            styled_components: options
                .styled_components
                .map(|options| StyledComponents::new(options, ctx)),
        }
    }
}

impl VisitMutHook for Plugins<'_> {
    fn enter_module(&mut self, module: &mut Module) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.enter_module(module);
        }
    }

    fn enter_var_declarator(&mut self, node: &mut VarDeclarator) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.enter_var_declarator(node);
        }
    }

    fn enter_expr(&mut self, node: &mut Expr) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.enter_expr(node);
        }
    }

    fn enter_call_expr(&mut self, node: &mut CallExpr) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.enter_call_expr(node);
        }
    }
}
