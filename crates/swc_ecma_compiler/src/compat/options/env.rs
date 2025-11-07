use serde::Deserialize;

use super::{babel::BabelEnvOptions, Module};
use crate::compat::{
    es2015::{ArrowFunctionsOptions, ES2015Options},
    es2016::ES2016Options,
    es2017::ES2017Options,
    es2018::{ES2018Options, ObjectRestSpreadOptions},
    es2019::ES2019Options,
    es2020::ES2020Options,
    es2021::ES2021Options,
    es2022::{ClassPropertiesOptions, ES2022Options},
    es2026::ES2026Options,
    regexp::RegExpOptions,
};

/// Environment options for ECMAScript transformations
///
/// This struct configures which ECMAScript features should be transformed
/// based on target environments.
///
/// Note: In the compat layer, target resolution is simplified compared to the
/// oxc version which used browserslist queries. Advanced target configuration
/// would require additional dependencies.
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(try_from = "BabelEnvOptions")]
pub struct EnvOptions {
    /// Specify what module code is generated.
    pub module: Module,

    pub regexp: RegExpOptions,

    pub es2015: ES2015Options,

    pub es2016: ES2016Options,

    pub es2017: ES2017Options,

    pub es2018: ES2018Options,

    pub es2019: ES2019Options,

    pub es2020: ES2020Options,

    pub es2021: ES2021Options,

    pub es2022: ES2022Options,

    pub es2026: ES2026Options,
}

impl EnvOptions {
    /// Explicitly enable all plugins that are ready, mainly for testing
    /// purposes.
    ///
    /// NOTE: for internal use only
    #[doc(hidden)]
    pub fn enable_all(include_unfinished_plugins: bool) -> Self {
        Self {
            module: Module::default(),
            regexp: RegExpOptions {
                sticky_flag: true,
                unicode_flag: true,
                unicode_property_escapes: true,
                dot_all_flag: true,
                named_capture_groups: true,
                look_behind_assertions: true,
                match_indices: true,
                set_notation: true,
            },
            es2015: ES2015Options {
                // Turned off because it is not ready.
                arrow_function: if include_unfinished_plugins {
                    Some(ArrowFunctionsOptions::default())
                } else {
                    None
                },
            },
            es2016: ES2016Options {
                exponentiation_operator: true,
            },
            es2017: ES2017Options {
                async_to_generator: true,
            },
            es2018: ES2018Options {
                object_rest_spread: Some(ObjectRestSpreadOptions::default()),
                async_generator_functions: true,
            },
            es2019: ES2019Options {
                optional_catch_binding: true,
            },
            es2020: ES2020Options {
                export_namespace_from: true,
                nullish_coalescing_operator: true,
                // Turn this on would throw error for all bigints.
                big_int: false,
                optional_chaining: true,
                arbitrary_module_namespace_names: false,
            },
            es2021: ES2021Options {
                logical_assignment_operators: true,
            },
            es2022: ES2022Options {
                class_static_block: true,
                class_properties: Some(ClassPropertiesOptions::default()),
                // Turn this on would throw error for all top-level awaits.
                top_level_await: false,
            },
            es2026: ES2026Options {
                explicit_resource_management: true,
            },
        }
    }

    /// Initialize from a [browserslist] query.
    ///
    /// Note: This is a stub in the compat layer. Full browserslist support
    /// would require additional dependencies.
    ///
    /// # Errors
    ///
    /// * When the query failed to parse.
    ///
    /// [browserslist]: <https://github.com/browserslist/browserslist>
    pub fn from_browserslist_query(_query: &str) -> Result<Self, String> {
        Err("Browserslist queries are not supported in the compat layer".to_string())
    }

    /// Initialize from a comma separated list of targets.
    ///
    /// Note: This is a stub in the compat layer.
    ///
    /// # Errors
    ///
    /// * When the query failed to parse.
    pub fn from_target(_s: &str) -> Result<Self, String> {
        Err("Target strings are not supported in the compat layer".to_string())
    }

    /// Initialize from a list of targets.
    ///
    /// Note: This is a stub in the compat layer.
    ///
    /// # Errors
    ///
    /// * When the query failed to parse.
    pub fn from_target_list<S: AsRef<str>>(_list: &[S]) -> Result<Self, String> {
        Err("Target lists are not supported in the compat layer".to_string())
    }
}

impl From<BabelEnvOptions> for EnvOptions {
    fn from(_o: BabelEnvOptions) -> Self {
        // In the compat layer, we return default options since we don't have
        // target resolution capabilities without oxc_compat
        Self::default()
    }
}
