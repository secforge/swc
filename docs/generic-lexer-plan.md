# Plan: Generic Depth-Based Lexing

## Goals
- Replace the negative `Context::ShouldNotLexLtOrGtAsType` guard with an opt-in scheme driven by a generic-depth counter inside both lexers.
- Keep `<`/`>` tokenization correct for:
  - Type arguments/parameters in TS syntax (type refs, import types, type aliases, etc.).
  - Expression-level generics (`new Foo<T>()`, `call<T>()`, decorator calls, TS instantiations).
  - JSX/TSX constructs (`<Foo<T>>`, JSX tag type arguments).
  - Regular relational/shift operators when generics are not permitted.
- Preserve public token APIs so downstream crates (deno_ast, dprint-plugin-typescript, etc.) continue to work.
- Provide clear invariants for future contributors on how and when to bump/unbump the depth counter.

## Constraints & Considerations
- `Context::InType` must remain authoritative for “type mode” (blocking JSX lexing, enabling type keywords). Depth-tracking needs to coexist with it until callers migrate.
- JSX relies on `TokenContext` to decide if `<` starts JSX or JS; depth-based lexing must not misclassify JSX tags, yet TSX must still allow `<Tag<T>>`.
- Helpers such as `ts_in_no_context`, `try_parse_ts`, and parser checkpoints currently snapshot lexer/ctx state. Any new depth counter must participate in those snapshots so speculative parses can rewind safely.
- The parser front-end exists twice (`swc_ecma_lexer` and `swc_ecma_parser`). Changes have to be mirrored.
- Validation spans multiple repos; integration with `dprint fmt`, `deno_ast`, etc., is required before shipping.

## Step-by-step Plan
1. **Audit Generic Entry Points**
   - Enumerate every parser location that legitimately consumes `<...>` as type arguments/parameters (TS type references, `new`/call expressions, TS import/type query, JSX type arguments, TS-only literals, etc.).
   - Document whether each location currently calls `ts_in_no_context`, `do_inside_of_context`, or other helpers so we know where the depth counter must be toggled.
2. **Design Lexer State Changes**
   - **Storage location:** add `generic_depth: u8` (plus helpers) to `lexer::state::State` (`crates/swc_ecma_lexer/src/lexer/state.rs`). Because this struct is cloned for checkpoints (both in `Lexer` and in `swc_ecma_parser::Parser`), the depth will automatically roll back with existing `checkpoint_save/load` flows.
   - **APIs exposed through `Tokens`:**
     - Extend `crate::common::input::Tokens` and both implementors (`crates/swc_ecma_lexer/src/lexer/mod.rs` + `crates/swc_ecma_parser/src/parser/input.rs`) with:
       - `fn generic_depth(&self) -> u8`
       - `fn with_generic_depth<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T`, which increments depth before running `f` and restores it afterward (internally it can call a shared `fn bump_generic_depth(&mut self, delta: i8)`).
       - A lighter-weight `fn is_in_generic(&self) -> bool` sugar.
     - Wire a parser helper `do_in_generic(|parser| { … })` that simply delegates to `input_mut().with_generic_depth(|buffer| { … })`, mirroring `do_inside_of_context`.
   - **Lexer behavior tweaks:**
     - `read_token_lt_gt` consults `generic_depth` before looking at `Context::InType`: if depth > 0 it force-emits plain `<`/`>` tokens regardless of operator lookahead; if depth == 0 it evaluates normal operator merges (`>=`, `>>`, `>>>`, etc.).
     - `Context::InType` continues to gate other type-only behaviors (e.g., skipping JSX, parsing keywords like `infer`). The new depth counter merely narrows the `<`/`>` handling surface.
   - **Integration with speculative parsing:**
     - `try_parse_ts` and `ts_in_no_context` already rely on parser checkpoints. Because `generic_depth` lives in `State`, cloning/restoring checkpoints will automatically reset the counter; still, `with_generic_depth` should be implemented in terms of `State` mutations so that rewinds remain cheap.
     - When speculative parses fail, callers must ensure any partially consumed `<` doesn’t leave the depth > 0. Using the scoped helper (rather than manual increment/decrement) enforces this invariant.
     - Parser-side code that currently needs to “merge” `>` back into `>=` (e.g., `next_then_parse_ts_type`) instead ensures `generic_depth` has returned to zero before attempting to treat `>` as part of other operators.
   - **Rollback-aware helpers:** `do_in_generic` should integrate with speculative parsing (`try_parse_ts`). When a checkpoint is restored, both the lexer clone and the buffer state already carry the previous `generic_depth`, so no extra work is needed beyond making sure guards don’t panic during checkpoint discards.
3. **Parser ↔ Lexer API Surface**
   - **Guard struct:** introduce `GenericLexGuard<'a, T: Tokens>` inside `crates/swc_ecma_lexer/src/common/parser/mod.rs` (or a sibling module). Construction:
     ```rust
     struct GenericLexGuard<'a, T: Tokens<TokenAndSpan>> {
         buffer: &'a mut T,
         active: bool,
     }
     impl<'a, T: Tokens<TokenAndSpan>> GenericLexGuard<'a, T> {
         fn new(buffer: &'a mut T) -> Self {
             let prev = buffer.generic_depth();
             buffer.set_generic_depth(prev + 1);
             GenericLexGuard { buffer, active: true }
         }
     }
     impl<'a, T: Tokens<TokenAndSpan>> Drop for GenericLexGuard<'a, T> {
         fn drop(&mut self) {
             if self.active {
                 let prev = self.buffer.generic_depth();
                 debug_assert!(prev > 0);
                 self.buffer.set_generic_depth(prev - 1);
             }
         }
     }
     ```
     (The guard simply defers to the `Tokens` trait methods from Step 2; exact names can differ.)
   - **Parser helper:** add `fn do_in_generic<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T` on `Parser` that instantiates the guard on `self.input_mut()` and executes the closure. This mirrors `do_inside_of_context` so all existing call-sites can migrate with minimal churn.
   - **Call-site strategy:** for each location catalogued in Step 1:
     - Wrap the existing `<...>` parsing logic in `do_in_generic`. Examples: `parse_ts_type_args`, `parse_ts_type_params`, `parse_maybe_decorator_args`, `parse_subscript`’s TS branch, JSX type-arg parsing.
     - Remove the surrounding `Context::ShouldNotLexLtOrGtAsType` toggles once the guard is in place.
   - **Speculative parsing hooks:**
     - `try_parse_ts` should invoke `do_in_generic` inside its closure whenever it attempts to consume `<`. Because the guard unwinds automatically, failed attempts leave the depth unchanged.
     - `ts_in_no_context` only needs to ensure it doesn’t unintentionally reset `generic_depth`; wrapping the temporary token-context swap inside `do_in_generic` is unnecessary unless it consumes `<`.
   - **Deprecation path:** after all guard migrations land, delete `Context::ShouldNotLexLtOrGtAsType` and replace its usage with assertions that `generic_depth == 0` when the parser returns to expression mode.
4. **JSX Integration**
   - **Where generics are legal:**
     - JSX opening elements (`parse_jsx_opening_element_at`) permit `<Tag<T>>` immediately after parsing the tag name; only that sub-expression should run inside `do_in_generic`.
     - Closing tags never allow generics; ensure depth is zero before calling `parse_jsx_closing_element_at`.
     - JSX fragments (`<>`) and spread/attribute positions must never enter generic mode even if TypeScript is enabled.
   - **Token-context coordination:**
     - When `TokenContext::JSXOpeningTag` or `JSXClosingTag` is active, JSX lexing should take precedence regardless of `generic_depth` (e.g., `<div>` should still lex as JSX even if depth > 0 from an outer TS context).
     - Conversely, while parsing `<Tag<T>>`, temporarily push a “generic mode” only around the type argument list; once `>` closes the type args, restore depth before attributes/body parsing resumes so relational operators in attribute expressions behave normally.
   - **Implementation hooks:**
     - Wrap the `try_parse_ts` call inside `parse_jsx_opening_element_after_name` with `p.do_in_generic(...)` so `<`/`>` are forced to be single tokens only for JSX type args.
     - Ensure `parse_jsx_element` itself runs `do_outside_of_context(Context::ShouldNotLexLtOrGtAsType)` until that flag is removed; after migration, replace the context guard with assertions that `generic_depth == 0` before entering/exiting JSX parsing.
   - **Testing:**
     - Add regression cases covering `<Foo<T>>`, `<Foo<T>()>`, nested JSX with generics (`<Foo><Bar<T> /></Foo>`), and combinations like `<Foo<T> as any>` that mix JSX and TS expressions.
     - Verify fragments (`<>`) and `<div>{expr >= 1}</div>` still produce `>=` tokens—i.e., depth returns to zero before expression parsing resumes.
5. **Mechanics for `>` Recognition**
   - **Lexing rules:**
     - `read_token_lt_gt` inspects `generic_depth` first. When depth > 0 it:
       - Emits a single `<` or `>` token immediately, without attempting to merge with adjacent `<`, `>`, or `=` characters.
       - Leaves `>=`, `>>`, `>>>`, `<<`, etc. unavailable until depth returns to 0.
     - When `generic_depth == 0`, the existing operator-merging logic remains intact so comparison and shift operators are recognized normally.
   - **Guard semantics:**
     - The parser owns depth transitions via `do_in_generic`. Every time it consumes a literal `<` that starts a type argument/parameter list, it enters generic mode; when the list is fully parsed, the guard drops and depth decrements even if multiple `>` tokens were read inside.
     - Nested generics naturally stack because inner calls to `do_in_generic` increment depth again.
   - **Eliminating rescans:**
     - Remove `Buffer::merge_lt_gt` and `Buffer::cut_lshift`. With depth-aware lexing, there is no need to retroactively fuse or split tokens after the lexer has run.
     - `next_then_parse_ts_type` no longer calls `merge_lt_gt`; instead it asserts `generic_depth == 0` (meaning all `<...>` groups have been closed) before resuming expression parsing.
     - JSX-specific rescans such as `rescan_jsx_open_el_terminal_token` continue to exist because they operate on `>=` emitted at depth 0; the depth counter does not affect them.
   - **Shift/assign operators after generics:**
     - Ensure the guard scope ends before the parser inspects tokens following the closing `>` so that `>=`, `>>`, etc. can be lexed normally. For example, `Foo<Bar> >= 1` should exit generic mode as soon as the `>` closing the outer type args is consumed; the immediately following `>=` must then be parsed as `>=`.
6. **Incremental Migration**
   - **Feature gating:** add a cargo feature (e.g., `ts_generic_depth_lexing`) or environment-driven flag to `swc_ecma_parser`/`lexer` so downstream consumers can opt in early. Default it to `false` until all regressions are ironed out.
   - **Phase 1 – TS-only parsing:**
     - Enable `do_in_generic` for purely TypeScript type contexts (`parse_ts_type_args`, `parse_ts_type_params`, declaration signatures) while leaving expression/JSX code paths on `ShouldNotLexLtOrGtAsType`.
     - Behind the feature flag, have `read_token_lt_gt` respect `generic_depth` but only when `Context::InType` is also set. Run focused TS tests (type references, `next_then_parse_ts_type` cases).
   - **Phase 2 – Expression paths:**
     - Migrate `parse_subscript`, `parse_member_expr_or_new_expr`, decorators, `import()` instantiations, etc., to `do_in_generic`.
     - Remove corresponding `ShouldNotLexLtOrGtAsType` guards as each site lands; keep a temporary compatibility layer that sets the context flag when the feature flag is off.
   - **Phase 3 – JSX/TSX:**
     - Apply the JSX-specific changes from Step 4, again behind the feature flag.
     - Once JSX regression tests pass (JSX + TSX suites), enable the feature by default in nightly/dev builds.
   - **Phase 4 – Cleanup:**
     - Delete `Context::ShouldNotLexLtOrGtAsType` and old helpers (`merge_lt_gt`, `cut_lshift`) once all sites use the new APIs and CI runs both feature-on/off paths cleanly.
     - Remove the transitional flag and ship the new behavior as default in the next semver release (communicate via CHANGELOG).
7. **Testing & Validation**
   - **Unit tests (within `swc_ecma_lexer`/`swc_ecma_parser`):**
     - Add targeted lexer tests verifying that, with `generic_depth > 0`, sequences like `<`, `>`, `>>`, `>=`, etc., emit the expected single-char tokens, and that exiting generic mode restores normal operator merging.
     - Extend parser tests for:
       - `next_then_parse_ts_type` scenarios (`i as number >= 5`, `async <T>() => {}`) to ensure no regressions.
       - Deeply nested generics (`Foo<Bar<Baz<C>>>`) to confirm depth stacking.
       - JSX/TSX fixtures (`<Foo<T>>`, `<Foo<T extends Bar>>() => {}`).
   - **Feature-flag matrix:** run CI with the new feature both enabled and disabled to guarantee compatibility while the flag is optional.
   - **Integration suites:**
     - `cargo test -p swc_ecma_parser --features typescript,jsx`.
     - Deno AST compatibility tests (`cargo test -p deno_ast` if available).
     - `dprint-plugin-typescript`: run `cargo test` plus smoke tests via `dprint fmt`.
   - **Downstream consumers:**
     - Coordinate with repos known to embed SWC (e.g., deno, biome) to run their CI against the feature branch.
     - Provide sample fixtures demonstrating the fixed behavior so external contributors can validate quickly.
   - **Regression harness:**
     - Capture the original failing snippets (`i as number >= 5`, `i as this >= 5`, `<Foo<T> >= 1`, etc.) as automated tests to prevent backslides.
     - Add property-based fuzzing (optional) to ensure alternating `<`/`>` patterns don’t panic when depth bookkeeping mismatches.
8. **Documentation & Cleanup**
   - **Docs:**
     - Refresh `docs/context-flags.md` to describe the new depth-based invariants, clarifying that `InType` now pairs with `generic_depth` instead of the negative opt-out flag.
     - Add a dedicated “Generic Depth Lexing” section to the developer guide (or `ARCHITECTURE.md`) describing how `do_in_generic` interacts with parser contexts, JSX, and checkpoints.
   - **CHANGELOG / release notes:**
     - Document the behavior change (lexer now treats `<`/`>` as opt-in generics) and highlight the bug fixes it unlocks (`as number >=`, TSX generics, etc.).
     - Provide upgrade guidance for downstream crates that relied on `ShouldNotLexLtOrGtAsType` (e.g., if any custom parsers used it directly).
   - **Code cleanup:**
     - Remove `Context::ShouldNotLexLtOrGtAsType` and any remaining references after migration.
     - Delete dead helpers (`merge_lt_gt`, `cut_lshift`) and related comments.
     - Search for inline TODOs referencing “should not lex < as type” and update them.
   - **Contributor guidance:**
     - Update CONTRIBUTING docs to mention `do_in_generic` whenever new syntax introduces `<...>` constructs.
     - Optionally add lint/check scripts (or dev documentation) ensuring future call-sites follow the depth-aware approach.

## Progress

### Step 1 – Generic Entry Point Audit ✅
- **Type expressions & import-like constructs**
  - `parse_ts_type_ref` consumes `<...>` after entity names and already toggles the flag (`crates/swc_ecma_lexer/src/common/parser/typescript.rs:480-525`).
  - `parse_ts_import_type` and `parse_ts_type_query` allow `<T>` after the qualifier/import target (`crates/swc_ecma_lexer/src/common/parser/typescript.rs:2180-2290`).
  - Type literal members, call/construct signatures, and function/constructor types all call `try_parse_ts_type_params` (`crates/swc_ecma_lexer/src/common/parser/typescript.rs:1395-1765`).
- **Expression & callee instantiations**
  - `parse_subscript` handles `<T>` for call expressions, tagged templates, TS instantiations, and optional chaining (`crates/swc_ecma_lexer/src/common/parser/expr.rs:600-760`).
  - `parse_member_expr_or_new_expr` / `parse_call_expr` cover `new Foo<T>()`, decorator calls, and `import()`/`super()` continuations (`crates/swc_ecma_lexer/src/common/parser/expr.rs:1040-1750`).
  - Decorator helpers (`parse_maybe_decorator_args`, `parse_super_class`) also accept `<...>` immediately after identifiers (`crates/swc_ecma_lexer/src/common/parser/class_and_fn.rs:40-230`).
- **Type parameter producers**
  - Declarations such as type aliases, interfaces, classes, and generic signatures invoke `parse_ts_type_params` / `try_parse_ts_type_params` (`crates/swc_ecma_lexer/src/common/parser/typescript.rs:720-1750`, `crates/swc_ecma_lexer/src/common/parser/class_and_fn.rs:220-260`, `crates/swc_ecma_lexer/src/common/parser/class_and_fn.rs:1028-1600`).
  - Generic async arrow detection (`try_parse_ts_generic_async_arrow_fn`) consumes `<T>` before `async` arrows (`crates/swc_ecma_lexer/src/common/parser/typescript.rs:2830-2860`).
- **JSX / TSX integration points**
  - JSX opening/closing parsing re-enters TS type-arg mode just for tag type arguments (`crates/swc_ecma_lexer/src/common/parser/jsx.rs:220-360`).
  - The parser mirror in `swc_ecma_parser` does the same (`crates/swc_ecma_parser/src/parser/jsx/mod.rs:320-380`).

These call-sites are where the new depth counter must be incremented/decremented; everywhere else should keep `<`/`>` in operator mode. Next action: design the lexer-side `generic_depth` storage and helper APIs (Step 2).
