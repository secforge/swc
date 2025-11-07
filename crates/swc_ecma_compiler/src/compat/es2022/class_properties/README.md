# Class Properties Module - Port Status

## Current Status: INITIAL SKELETON ✅

The basic infrastructure for the class_properties module has been established.

### Completed ✅

1. **Directory Structure Created**
   - `/Users/kdy1/projects/chores/crates/swc_ecma_compiler/src/compat/es2022/class_properties/`

2. **Files Created**
   - `mod.rs` - Main module with `ClassProperties` struct and `VisitMutHook` trait implementation (skeleton)
   - `utils.rs` - Basic utility functions (ported)
   - `PORTING_PLAN.md` - Detailed porting strategy and guide
   - `README.md` - This file

3. **Integration**
   - Updated `/Users/kdy1/projects/chores/crates/swc_ecma_compiler/src/compat/es2022/mod.rs` to export the module
   - Module compiles successfully with Rust compiler

### What's In The Skeleton

**mod.rs** contains:
- `ClassPropertiesOptions` struct with `loose` option
- `ClassProperties<'ctx>` struct with basic fields:
  - Options: `set_public_class_fields`, `private_fields_as_properties`, `transform_static_blocks`, `remove_class_fields_without_initializer`
  - Context reference: `ctx: &'ctx TransformCtx`
- Empty `VisitMutHook` implementation (ready for visitor methods)
- Comprehensive documentation from oxc

**utils.rs** contains:
- `create_variable_declaration()` - Creates a `var` declaration
- `exprs_into_stmts()` - Converts expressions to statements
- `create_underscore_ident_name()` - Creates identifier for `_`

### Next Steps

The remaining work is substantial and follows the phased approach in `PORTING_PLAN.md`:

#### Phase 1: Foundation (Estimated: 2-3 hours)
- [ ] Port `class_bindings.rs` (146 lines)
- [ ] Port `class_details.rs` (283 lines)

#### Phase 2: Property Transformations (Estimated: 4-5 hours)
- [ ] Port `computed_key.rs` (164 lines)
- [ ] Port `prop_decl.rs` (391 lines)
- [ ] Port `instance_prop_init.rs` (231 lines)
- [ ] Port `constructor.rs` (797 lines)

#### Phase 3: Static Context (Estimated: 3-4 hours)
- [ ] Port `super_converter.rs` (650 lines)
- [ ] Port `static_block_and_prop_init.rs` (550 lines)

#### Phase 4: Private Members (Estimated: 5-6 hours)
- [ ] Port `private_method.rs` (173 lines)
- [ ] Port `private_field.rs` (2854 lines) ⚠️ LARGEST FILE

#### Phase 5: Integration (Estimated: 2-3 hours)
- [ ] Port `class.rs` (903 lines)
- [ ] Complete `mod.rs` with all visitor methods

#### Phase 6: Testing & Debug (Estimated: 4-5 hours)
- [ ] Add unit tests
- [ ] Add integration tests
- [ ] Compare output with Babel
- [ ] Fix edge cases

**Total Estimated Effort: 20-26 hours**

### Key Challenges

1. **AST Differences**
   - Oxc uses arena allocator with lifetimes: `Expression<'a>`, `Box<'a, T>`
   - SWC uses owned types: `Box<Expr>`, `Stmt`
   - Need to replace `TakeIn`, arena allocations

2. **Visitor Pattern**
   - Oxc: `Traverse<'a>` trait with `enter_*/exit_*` methods
   - SWC: `VisitMutHook` trait with different method signatures
   - Need to adapt all visitor methods

3. **Context & Scoping**
   - Oxc has sophisticated `TraverseCtx` with scope management
   - SWC's `TransformCtx` is simpler
   - May need custom scope tracking

4. **Helper Functions**
   - Oxc has built-in UID generation: `ctx.generate_uid()`
   - SWC needs custom implementations
   - Helper loading mechanism differs

### Resources

- **Original Implementation**: `/Users/kdy1/projects/chores/crates/swc_ecma_compiler/src/oxc/es2022/class_properties/`
- **Porting Guide**: `PORTING_PLAN.md` in this directory
- **SWC Hooks API**: `/Users/kdy1/projects/chores/crates/swc_ecma_hooks/src/lib.rs`
- **Example Ported Module**: `/Users/kdy1/projects/chores/crates/swc_ecma_compiler/src/compat/es2021/logical_assignment_operators.rs`

### Testing Strategy

For each ported file:
1. Ensure it compiles
2. Write unit tests for individual functions
3. Add integration tests for transformations
4. Compare output with Babel's output
5. Test edge cases (nested classes, multiple super calls, etc.)

### Contributing

When porting files:
1. Follow the dependency order in `PORTING_PLAN.md`
2. Maintain the same logic and structure as oxc
3. Update comments to reflect SWC patterns
4. Add tests as you go
5. Run `cargo fmt` before committing
6. Update this README with progress

### Questions & Support

For questions about:
- **SWC AST**: See `swc_ecma_ast` documentation
- **Hooks API**: See `swc_ecma_hooks` crate
- **Original Logic**: Refer to oxc implementation with extensive comments
- **Babel Behavior**: Check Babel test suite and documentation

---

**Last Updated**: 2025-11-07
**Status**: Initial skeleton complete, ready for incremental porting
