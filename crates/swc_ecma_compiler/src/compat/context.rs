use std::{
    cell::RefCell,
    mem,
    path::{Path, PathBuf},
};

use swc_ecma_ast::EsVersion;

use crate::{compat::common::statement_injector::StatementInjectorStore, Config};

/// Transform context for SWC-based transformations.
///
/// This context provides shared state and utilities for all transformation
/// passes. Unlike the oxc version which uses TraverseCtx, this version is
/// designed to work with SWC's VisitMut pattern.
pub struct TransformCtx {
    errors: RefCell<Vec<String>>,

    /// Source filename
    pub filename: String,

    /// Source path in the form of `<CWD>/path/to/file/input.js`
    pub source_path: PathBuf,

    /// Target ECMAScript version
    pub target: EsVersion,

    /// Source text (not used in SWC transforms, but kept for compatibility)
    pub source_text: &'static str,

    /// Compiler assumptions for optimization
    pub assumptions: swc_ecma_transforms_base::assumptions::Assumptions,

    /// Manage inserting statements immediately before or after the target
    /// statement
    pub statement_injector: StatementInjectorStore,

    // State for multiple plugins interacting
    /// `true` if class properties plugin is enabled
    pub is_class_properties_plugin_enabled: bool,
}

impl TransformCtx {
    /// Create a new transform context from a source path and configuration.
    ///
    /// # Arguments
    /// * `source_path` - Path to the source file being transformed
    /// * `config` - Compiler configuration containing assumptions and features
    pub fn new(source_path: &Path, config: &Config) -> Self {
        let filename = source_path
            .file_stem() // omit file extension
            .map_or_else(
                || String::from("unknown"),
                |name| name.to_string_lossy().to_string(),
            );

        Self {
            errors: RefCell::new(vec![]),
            filename,
            source_path: source_path.to_path_buf(),
            target: EsVersion::Es5,
            source_text: "",
            assumptions: config.assumptions,
            statement_injector: StatementInjectorStore::new(),
            is_class_properties_plugin_enabled: false,
        }
    }

    /// Take all accumulated errors from the context.
    ///
    /// This consumes the error list, leaving an empty vector in its place.
    pub fn take_errors(&self) -> Vec<String> {
        mem::take(&mut self.errors.borrow_mut())
    }

    /// Add an error to the context.
    ///
    /// # Arguments
    /// * `error` - Error message to add
    pub fn error(&self, error: String) {
        self.errors.borrow_mut().push(error);
    }
}
