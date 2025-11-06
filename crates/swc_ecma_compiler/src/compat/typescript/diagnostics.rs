/// Diagnostic message for import equals used in ESM context
pub fn import_equals_cannot_be_used_in_esm() -> String {
    "Import assignment cannot be used when targeting ECMAScript modules. Consider using 'import * \
     as ns from \"mod\"', 'import {a} from \"mod\"', 'import d from \"mod\"', or another module \
     format instead. (TS1202)"
        .to_string()
}

/// Diagnostic message for export assignment used in ESM context
pub fn export_assignment_cannot_be_used_in_esm() -> String {
    "Export assignment cannot be used when targeting ECMAScript modules. Consider using 'export \
     default' or another module format instead. (TS1203)"
        .to_string()
}

/// Diagnostic message for nested ambient modules
pub fn ambient_module_nested() -> String {
    "Ambient modules cannot be nested in other modules or namespaces.".to_string()
}

/// Diagnostic message for namespaces exporting non-const variables
pub fn namespace_exporting_non_const() -> String {
    "Namespaces exporting non-const are not supported by Babel. \
     Change to const or see: https://babeljs.io/docs/en/babel-plugin-transform-typescript"
        .to_string()
}

/// Diagnostic message for non-declarative namespaces
pub fn namespace_not_supported() -> String {
    "Namespace not marked type-only declare. Non-declarative namespaces are only supported \
     experimentally in Babel. To enable and review caveats see: \
     https://babeljs.io/docs/en/babel-plugin-transform-typescript"
        .to_string()
}
