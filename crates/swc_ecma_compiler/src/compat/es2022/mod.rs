/// ES2022 options and exports
///
/// This module provides placeholder exports for ES2022 features.
/// The actual transformation implementations will be added as needed.
use serde::Deserialize;

/// ES2022 transformation options
///
/// This struct provides options for ES2022 features including:
/// - Class static blocks
/// - Class properties
/// - Top-level await
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ES2022Options {
    /// Enable class static block transformation
    #[serde(skip)]
    pub class_static_block: bool,

    /// Options for class properties transformation
    #[serde(skip)]
    pub class_properties: Option<ClassPropertiesOptions>,

    /// Enable top-level await transformation
    #[serde(skip)]
    pub top_level_await: bool,
}

/// Class properties transformation options
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ClassPropertiesOptions {
    // Add configuration options as needed
}
