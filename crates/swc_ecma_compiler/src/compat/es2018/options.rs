use serde::Deserialize;

use super::object_rest_spread::ObjectRestSpreadOptions;

/// ES2018 transformation options.
///
/// This module provides options for ES2018 features including:
/// - Object rest/spread properties
/// - Async generator functions
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ES2018Options {
    /// Options for object rest/spread transformation
    #[serde(skip)]
    pub object_rest_spread: Option<ObjectRestSpreadOptions>,

    /// Enable async generator function transformation
    #[serde(skip)]
    pub async_generator_functions: bool,
}
