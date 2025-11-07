# Class Properties Module - Porting Plan

## Overview

This document outlines the plan for porting the class_properties module from oxc to SWC.
The module consists of 13 files with approximately 4000+ lines of code implementing
ES2022 class properties transformation.

## Module Structure

### Files to Port (in dependency order):

1. **utils.rs** (61 lines) - ✅ COMPLETED
   - Basic utility functions
   - No dependencies on other module files

2. **class_bindings.rs** (146 lines)
   - Manages bindings for class names and temp vars
   - Depends on: none

3. **class_details.rs** (283 lines)
   - Structures for storing class transformation state
   - Depends on: class_bindings.rs

4. **computed_key.rs** (164 lines)
   - Transform computed property keys
   - Depends on: class_bindings.rs, utils.rs

5. **prop_decl.rs** (391 lines)
   - Transform property declarations (instance/static)
   - Depends on: utils.rs, computed_key.rs

6. **instance_prop_init.rs** (231 lines)
   - Reparent instance property initializer scopes
   - Depends on: class_bindings.rs

7. **constructor.rs** (797 lines)
   - Insert property initializers into constructors
   - Depends on: utils.rs, instance_prop_init.rs

8. **static_block_and_prop_init.rs** (550 lines)
   - Transform static properties and blocks
   - Depends on: utils.rs, super_converter.rs

9. **super_converter.rs** (650 lines)
   - Transform `super` expressions in class properties
   - Depends on: class_bindings.rs

10. **private_method.rs** (173 lines)
    - Transform private methods
    - Depends on: super_converter.rs

11. **private_field.rs** (2854 lines) - LARGEST FILE
    - Transform private field expressions
    - Depends on: class_details.rs, utils.rs

12. **class.rs** (903 lines)
    - Main class transformation logic
    - Depends on: ALL above files

13. **mod.rs** (434 lines)
    - Main module entry point with visitor
    - Depends on: ALL above files

## Key Porting Challenges

### 1. AST Differences

**Oxc** uses:
- `oxc_ast::ast::*`
- Arena allocator with `'a` lifetime
- `TakeIn` for moving values
- `Box<'a, T>` for arena-allocated boxes

**SWC** uses:
- `swc_ecma_ast::*`
- Owned types
- Standard `Box<T>`
- No arena allocator

### 2. Visitor Pattern Differences

**Oxc** uses:
```rust
impl<'a> Traverse<'a> for ClassProperties<'a, '_> {
    fn enter_class_body(&mut self, body: &mut ClassBody<'a>, ctx: &mut TraverseCtx<'a>);
    fn exit_class(&mut self, class: &mut Class<'a>, ctx: &mut TraverseCtx<'a>);
}
```

**SWC** uses:
```rust
impl VisitMutHook for ClassProperties<'_> {
    fn enter_class(&mut self, class: &mut Class);
    fn exit_class(&mut self, class: &mut Class);
}
```

### 3. Context Differences

**Oxc** has:
- `TraverseCtx` with scope management
- `TransformCtx` with helper loading
- Built-in temp var generation
- Scope reparenting utilities

**SWC** has:
- `TransformCtx` (simpler)
- Manual scope tracking may be needed
- Different helper loading mechanism

### 4. Specific Transformations

The module handles:
- Public and private instance properties
- Public and private static properties
- Static blocks
- Private methods (instance and static)
- Private getters/setters
- Computed keys
- Super expressions in static context
- Constructor parameter scope handling
- Multiple `super()` calls in constructor

## Porting Strategy

### Phase 1: Foundation (Files 1-3)
- ✅ utils.rs - DONE
- [ ] class_bindings.rs
- [ ] class_details.rs

These provide the core data structures needed by all other files.

### Phase 2: Property Transformations (Files 4-7)
- [ ] computed_key.rs
- [ ] prop_decl.rs
- [ ] instance_prop_init.rs
- [ ] constructor.rs

These handle the main property transformation logic.

### Phase 3: Static Context (Files 8-9)
- [ ] super_converter.rs
- [ ] static_block_and_prop_init.rs

These handle static properties, static blocks, and super expressions.

### Phase 4: Private Members (Files 10-11)
- [ ] private_method.rs
- [ ] private_field.rs (LARGEST - needs extra care)

These transform private fields and methods.

### Phase 5: Integration (Files 12-13)
- [ ] class.rs
- [ ] mod.rs

These integrate all the transformations into a cohesive visitor.

## Key Mapping Guide

### Type Mappings

| Oxc | SWC |
|-----|-----|
| `Expression<'a>` | `Box<Expr>` |
| `Statement<'a>` | `Stmt` |
| `Class<'a>` | `Class` |
| `ClassBody<'a>` | `Vec<ClassMember>` |
| `PropertyDefinition<'a>` | `ClassProp` |
| `MethodDefinition<'a>` | `ClassMethod` |
| `PrivateIdentifier<'a>` | `PrivateName` |
| `BoundIdentifier<'a>` | `Ident` |
| `ctx.ast.vec()` | `Vec::new()` or `vec![]` |
| `ctx.ast.alloc(x)` | `Box::new(x)` |
| `expr.take_in(ctx.ast)` | `std::mem::take(expr)` or `std::mem::replace` |

### Method Mappings

| Oxc | SWC |
|-----|-----|
| `ctx.generate_uid()` | Custom UID generation needed |
| `ctx.ast.expression_*()` | Construct `Expr::*` directly |
| `ctx.ast.statement_*()` | Construct `Stmt::*` directly |
| `ctx.scoping()` | May need custom scope tracking |
| `ctx.scoping_mut()` | May need custom scope tracking |

## Testing Strategy

After porting each phase:
1. Add unit tests for individual functions
2. Add integration tests for the complete transformation
3. Compare output with Babel's output for equivalence
4. Test edge cases (nested classes, multiple super calls, etc.)

## Notes

- The original oxc implementation is well-documented with extensive comments
- Most logic can be preserved; main work is AST/API adaptation
- Some helper functions may need to be created for SWC
- Scope management is a key challenge that may need custom solutions
- The private_field.rs file is massive and may benefit from being split

## Estimated Effort

- Foundation: 2-3 hours
- Property Transformations: 4-5 hours
- Static Context: 3-4 hours
- Private Members: 5-6 hours
- Integration: 2-3 hours
- Testing & Debug: 4-5 hours

**Total: 20-26 hours of focused work**

This is a significant undertaking that should be done carefully and incrementally.
