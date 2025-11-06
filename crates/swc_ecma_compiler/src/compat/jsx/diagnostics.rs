pub fn pragma_and_pragma_frag_cannot_be_set() -> String {
    "pragma and pragmaFrag cannot be set when runtime is automatic. Remove `pragma` and \
     `pragmaFrag` options."
        .to_string()
}

pub fn invalid_pragma() -> String {
    "pragma and pragmaFrag must be of the form `foo` or `foo.bar`. Fix `pragma` and `pragmaFrag` \
     options."
        .to_string()
}

pub fn import_source_cannot_be_set() -> String {
    "importSource cannot be set when runtime is classic. Remove `importSource` option.".to_string()
}

pub fn invalid_import_source() -> String {
    "importSource cannot be an empty string or longer than u32::MAX bytes. Fix `importSource` \
     option."
        .to_string()
}

pub fn namespace_does_not_support() -> String {
    "Namespace tags are not supported by default. React's JSX doesn't support namespace tags. You \
     can set `throwIfNamespace: false` to bypass this warning."
        .to_string()
}

pub fn valueless_key() -> String {
    "Please provide an explicit key value. Using \"key\" as a shorthand for \"key={true}\" is not \
     allowed."
        .to_string()
}
