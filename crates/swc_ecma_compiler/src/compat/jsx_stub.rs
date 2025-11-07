// Temporary stub for JSX types until SWC AST compatibility issues are resolved

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JsxOptions {
    #[serde(default)]
    pub development: bool,
    #[serde(default)]
    pub jsx_plugin: bool,
    #[serde(default)]
    pub display_name_plugin: bool,
    #[serde(default)]
    pub jsx_self_plugin: bool,
    #[serde(default)]
    pub jsx_source_plugin: bool,
    #[serde(default)]
    pub pure: bool,
    #[serde(default)]
    pub refresh: Option<ReactRefreshOptions>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReactRefreshOptions {
    #[serde(default)]
    pub emit_full_signatures: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsxRuntime {
    Classic,
    Automatic,
}

impl Default for JsxRuntime {
    fn default() -> Self {
        Self::Classic
    }
}
