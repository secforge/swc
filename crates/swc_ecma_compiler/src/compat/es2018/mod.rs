//! ES2018 compatibility transformations.
//!
//! This module provides transformations for ES2018 features:
//!
//! - **Object rest/spread**: Transform object rest/spread syntax
//! - **Async generator functions**: Transform async generator functions and
//!   for-await loops
//!
//! ## Note
//!
//! These are stub implementations ported from oxc. The oxc implementations are
//! extremely complex (over 1500 lines combined) and rely heavily on:
//! - Arena allocation and lifetime management
//! - oxc's Traverse trait and TraverseCtx
//! - Complex scope and symbol management
//! - Statement injection mechanisms
//!
//! Full porting to SWC's VisitMutHook pattern would require:
//! 1. Rewriting using SWC's owned AST types
//! 2. Adapting to SWC's visitor pattern
//! 3. Implementing scope management using SWC's APIs
//! 4. Handling helper function injection
//! 5. Managing temporary variable generation
//!
//! For production use, prefer SWC's built-in transforms in
//! `swc_ecma_transforms_compat::es2018`.

pub mod async_generator_functions;
pub mod object_rest_spread;
pub mod options;

pub use async_generator_functions::AsyncGeneratorFunctions;
pub use object_rest_spread::{ObjectRestSpread, ObjectRestSpreadOptions};
pub use options::ES2018Options;
