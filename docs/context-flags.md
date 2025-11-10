# Context Flag: `InType`

## Definitions
- `Context::InType` is defined in `crates/swc_ecma_lexer/src/common/context.rs` and re-exported by the parser crate (`crates/swc_ecma_parser/src/lib.rs`).
- `Parser::in_type` (`crates/swc_ecma_lexer/src/common/parser/mod.rs`) is the canonical helper for temporarily inserting `InType`, executing a closure, and restoring the prior context.

## Effect on Lexing
- Both lexers (`crates/swc_ecma_lexer/src/lexer/mod.rs` and `crates/swc_ecma_parser/src/lexer/mod.rs`) inspect `InType` *and* the lexer’s `generic_depth` each time they read `<` or `>`:
  - If `syntax.typescript()` **and** (`Context::InType` is set **or** `generic_depth > 0`), `<` and `>` are emitted as dedicated type tokens (`tok!('<')`, `Token::Lt`, etc.) rather than relational operators.
  - Otherwise they behave like normal comparison or shift operators and still recognize combined forms such as `>=`, `>>`, and `>>>`.
- JSX lexing is disabled whenever `InType` is active (`crates/swc_ecma_lexer/src/lexer/state.rs:786-815`), which prevents `<T>` from being mistaken for JSX elements inside types.
- Identifier parsing allows JSX-style identifiers (`foo:bar`) only when `InType` is on (`crates/swc_ecma_lexer/src/common/parser/ident.rs:20-44`).

## Typical `InType` Usage
- Any TypeScript-specific parse entry sets `InType` while it runs:
  - Type arguments/parameters and annotations (`crates/swc_ecma_lexer/src/common/parser/typescript.rs:469-620`).
  - `ts_import_type`, `ts_type_query`, conditional/mapped/indexed/array types (`crates/swc_ecma_lexer/src/common/parser/typescript.rs:1800-2315`).
  - Function/class helpers that may see `<T>` immediately after the keyword (`crates/swc_ecma_lexer/src/common/parser/class_and_fn.rs:220-260`).
  - Statement helpers like catch-parameter annotations (`crates/swc_ecma_lexer/src/common/parser/stmt.rs:760-805`) and `async as Type` expressions (`crates/swc_ecma_lexer/src/common/parser/expr.rs:2285-2335`).
- `Parser::do_in_generic` temporarily increments the lexer’s `generic_depth` while a `<...>` construct is parsed; internally it also calls `in_type` so the lexer suppresses JSX tokenization. Most modern `<...>` call-sites rely on this helper instead of manipulating contexts manually.
- The DTS fast-path visitor (`crates/swc_typescript/src/fast_dts/visitors/type_usage.rs:206-224`) also relies on `InType` to avoid traversing JSX while inside a type query.

## Migration Notes
- Prior to the `generic_depth` work, parsers relied on `Context::ShouldNotLexLtOrGtAsType` to *disable* type-argument lexing while scanning expressions or JSX. That flag has been removed; instead, new `<...>` contexts must opt *in* by wrapping the parse in `Parser::do_in_generic`.
- Legacy helpers such as `cut_lshift`, `merge_lt_gt`, and `expect_without_advance` have been removed. Any new syntax that introduces `<...>` sections must ensure it uses `do_in_generic` (or another depth-aware helper) so the lexer emits the right tokens up front.
