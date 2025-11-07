use serde::Deserialize;

use crate::compat::options::Module;

fn default_as_true() -> bool {
    true
}

/// Babel preset-env options
///
/// Note: This is a simplified version that only handles basic target
/// configuration. Advanced features like browserslist queries are not fully
/// supported in the compat layer.
#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BabelEnvOptions {
    /// Target environments
    ///
    /// Note: In the compat layer, this is kept as a stub. The actual target
    /// resolution would need additional dependencies.
    #[serde(skip)]
    pub targets: (),

    #[deprecated = "Not Implemented"]
    #[serde(default = "default_as_true")]
    pub bugfixes: bool,

    #[deprecated = "Not Implemented"]
    pub spec: bool,

    #[deprecated = "Not Implemented"]
    pub loose: bool,

    pub modules: Module,

    #[deprecated = "Not Implemented"]
    pub debug: bool,

    #[deprecated = "Not Implemented"]
    pub include: Option<serde_json::Value>,

    #[deprecated = "Not Implemented"]
    pub exclude: Option<serde_json::Value>,

    #[deprecated = "Not Implemented"]
    pub use_built_ins: Option<serde_json::Value>,

    #[deprecated = "Not Implemented"]
    pub corejs: Option<serde_json::Value>,

    #[deprecated = "Not Implemented"]
    pub force_all_transforms: bool,

    #[deprecated = "Not Implemented"]
    pub config_path: Option<String>,

    #[deprecated = "Not Implemented"]
    pub ignore_browserslist_config: bool,

    #[deprecated = "Not Implemented"]
    pub shipped_proposals: bool,
}

/// Module format options from Babel
#[derive(Default, Debug, Clone, Deserialize)]
pub enum BabelModule {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "amd")]
    Amd,
    #[serde(rename = "umd")]
    Umd,
    #[serde(rename = "systemjs")]
    Systemjs,
    #[serde(rename = "commonjs", alias = "cjs")]
    Commonjs,
    #[serde(untagged)]
    Boolean(bool),
}
