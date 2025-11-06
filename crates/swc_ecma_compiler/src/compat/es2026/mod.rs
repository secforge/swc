//! ES2026 transformations
//!
//! This module contains transformations for ES2026 features, ported from oxc.
//!
//! ## Features
//!
//! - Explicit Resource Management (`using` and `await using` declarations)
//!
//! ## Implementation Status
//!
//! This is a port of oxc's ES2026 transformation to work with SWC's AST types.
//! The main differences from the oxc implementation:
//!
//! - Uses `swc_ecma_ast` types instead of `oxc_ast`
//! - Uses `swc_ecma_visit::VisitMut` instead of `oxc_traverse::Traverse`
//! - Uses owned types instead of arena allocation with lifetimes
//! - Helper loading is adapted to SWC's infrastructure
//!
//! ## Porting Notes
//!
//! The original oxc implementation in `/oxc/es2026/` consists of:
//! - `mod.rs` (~2.5KB): Main ES2026 struct with Traverse impl
//! - `options.rs` (~234 bytes): ES2026Options configuration
//! - `explicit_resource_management.rs` (~35KB): Explicit resource management
//!   transformation
//!
//! ### Key Challenges in Porting
//!
//! 1. **AST Type Differences**: SWC's AST has different structure and naming
//!    - `oxc_ast::ast::Program` → `swc_ecma_ast::Program`/`Module`/`Script`
//!    - `oxc_ast::ast::Statement` → `swc_ecma_ast::Stmt`
//!    - `oxc_ast::ast::VariableDeclaration` → `swc_ecma_ast::VarDecl`
//!
//! 2. **Using/Await-Using Detection**: oxc has explicit
//!    `VariableDeclarationKind::Using` and
//!    `VariableDeclarationKind::AwaitUsing` enum variants. SWC's AST doesn't
//!    have these built-in, so we need alternative detection mechanisms (TODO).
//!
//! 3. **Visitor Pattern**: SWC uses `VisitMut` trait which mutates in-place, vs
//!    oxc's `Traverse` with arena allocation.
//!
//! 4. **Helper Infrastructure**: Helper loading needs to be adapted to SWC's
//!    helper system.
//!
//! 5. **Symbol/Scope Management**: oxc uses `oxc_semantic` for symbol
//!    resolution and scope management with `TraverseCtx`. SWC has different
//!    symbol resolution.
//!
//! ## Known Limitations
//!
//! The current implementation is a partial port with the following limitations:
//!
//! - Using/await-using detection is not fully implemented (requires AST
//!   extension or metadata)
//! - Scope and symbol management is simplified compared to oxc's implementation
//! - Some complex transformations (e.g., top-level using with exports) are
//!   stubbed out
//!
//! ## TODO
//!
//! - [ ] Implement proper using/await-using detection mechanism
//! - [ ] Complete the program-level transformation logic
//! - [ ] Add proper scope and symbol tracking
//! - [ ] Implement UID generation with proper uniqueness guarantees
//! - [ ] Add comprehensive tests
//! - [ ] Handle all edge cases from the oxc implementation

mod explicit_resource_management;
mod options;

use explicit_resource_management::ExplicitResourceManagement;
pub use options::ES2026Options;
use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::compat::TransformCtx;

/// ES2026 transformation
///
/// This struct manages ES2026 feature transformations, delegating to the
/// appropriate sub-transformations based on options.
pub struct ES2026 {
    explicit_resource_management: Option<ExplicitResourceManagement>,
}

impl ES2026 {
    /// Create a new ES2026 transformer with the given options
    ///
    /// # Arguments
    /// * `options` - Configuration for ES2026 transformations
    /// * `ctx` - Transform context (unused in current implementation)
    pub fn new(options: ES2026Options, _ctx: &TransformCtx) -> Self {
        let explicit_resource_management = if options.explicit_resource_management {
            Some(ExplicitResourceManagement::new())
        } else {
            None
        };
        Self {
            explicit_resource_management,
        }
    }
}

impl VisitMut for ES2026 {
    fn visit_mut_module(&mut self, n: &mut Module) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_module(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_script(&mut self, n: &mut Script) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_script(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_for_of_stmt(&mut self, n: &mut ForOfStmt) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_for_of_stmt(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_block_stmt(&mut self, n: &mut BlockStmt) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_block_stmt(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_try_stmt(&mut self, n: &mut TryStmt) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_try_stmt(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_function(&mut self, n: &mut Function) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_function(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_arrow_expr(&mut self, n: &mut ArrowExpr) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_arrow_expr(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_static_block(&mut self, n: &mut StaticBlock) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_static_block(n);
        }
        n.visit_mut_children_with(self);
    }

    fn visit_mut_switch_stmt(&mut self, n: &mut SwitchStmt) {
        if let Some(erm) = &mut self.explicit_resource_management {
            erm.visit_mut_switch_stmt(n);
        }
        n.visit_mut_children_with(self);
    }
}
