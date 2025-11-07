// Temporary stub for DecoratorOptions until SWC AST compatibility issues are
// resolved

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct DecoratorOptions {
    #[serde(default)]
    pub legacy: bool,
    #[serde(default)]
    pub emit_decorator_metadata: bool,
}
