mod options;
mod styled_components;

pub use options::PluginsOptions;
use styled_components::StyledComponents;
pub use styled_components::StyledComponentsOptions;
use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

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

impl VisitMut for Plugins<'_> {
    fn visit_mut_module(&mut self, module: &mut Module) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.visit_mut_module(module);
        }
        module.visit_mut_children_with(self);
    }

    fn visit_mut_script(&mut self, script: &mut Script) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.visit_mut_script(script);
        }
        script.visit_mut_children_with(self);
    }

    fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.visit_mut_var_declarator(node);
        }
        node.visit_mut_children_with(self);
    }

    fn visit_mut_expr(&mut self, node: &mut Expr) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.visit_mut_expr(node);
        }
        node.visit_mut_children_with(self);
    }

    fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
        if let Some(styled_components) = &mut self.styled_components {
            styled_components.visit_mut_call_expr(node);
        }
        node.visit_mut_children_with(self);
    }
}
