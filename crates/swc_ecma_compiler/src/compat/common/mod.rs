//! Common utilities shared across transformation passes.

pub mod arrow_function_converter;
pub mod computed_key;
pub mod duplicate;
pub mod helper_loader;
pub mod module_imports;
pub mod statement_injector;
pub mod top_level_statements;
pub mod var_declarations;

pub use arrow_function_converter::ArrowFunctionConverterMode;
pub use computed_key::{create_computed_key_temp_var, key_needs_temp_var};
pub use duplicate::{can_duplicate_without_side_effects, duplicate_expression};
pub use helper_loader::{Helper, HelperLoaderMode, HelperLoaderOptions, HelperLoaderStore};
pub use statement_injector::StatementInjectorStore;
pub use top_level_statements::TopLevelStatementsStore;
pub use var_declarations::VarDeclarationsStore;
