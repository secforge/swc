use serde::Deserialize;

use super::ArrowFunctionsOptions;

/// Options for ES2015 transformations.
///
/// This struct configures which ES2015 features should be transformed
/// and how they should be transformed.
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ES2015Options {
    /// Options for transforming arrow functions to regular functions.
    ///
    /// When enabled, transforms arrow functions (`() => {}`) to function
    /// expressions (`function () {}`), including proper handling of `this`
    /// binding.
    #[serde(skip)]
    pub arrow_function: Option<ArrowFunctionsOptions>,
}
