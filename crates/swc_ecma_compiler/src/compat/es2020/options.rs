use serde::Deserialize;

/// Options for ES2020 transformations.
///
/// Controls which ES2020 features should be transformed to ES5-compatible code.
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ES2020Options {
    /// Transform `export * as ns from "mod"` to `import * as ns from "mod";
    /// export { ns }`.
    #[serde(skip)]
    pub export_namespace_from: bool,

    /// Transform nullish coalescing operator (`??`).
    #[serde(skip)]
    pub nullish_coalescing_operator: bool,

    /// Warn about BigInt literals in unsupported environments.
    #[serde(skip)]
    pub big_int: bool,

    /// Transform optional chaining (`?.`).
    #[serde(skip)]
    pub optional_chaining: bool,

    /// Warn about arbitrary module namespace identifier names.
    #[serde(skip)]
    pub arbitrary_module_namespace_names: bool,
}
