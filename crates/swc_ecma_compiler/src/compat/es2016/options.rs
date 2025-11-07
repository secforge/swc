use serde::Deserialize;

/// Options for ES2016 transformations.
///
/// This struct configures which ES2016 features should be transformed
/// and how they should be transformed.
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ES2016Options {
    /// Enable exponentiation operator transformation.
    ///
    /// When enabled, transforms the exponentiation operator (`**`) to
    /// `Math.pow`.
    #[serde(skip)]
    pub exponentiation_operator: bool,
}
