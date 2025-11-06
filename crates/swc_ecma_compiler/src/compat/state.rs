//! Transform state for SWC-based compatibility transformations.
//!
//! Unlike the OXC-based transformations which use oxc_traverse's state
//! parameter, SWC transformations manage state directly as fields in the
//! visitor struct. This module is provided for structural consistency but is
//! not currently used in the SWC-based approach.
//!
//! If you need to share state across multiple transforms, consider adding
//! fields to the visitor struct (e.g., `CompilerImpl` in lib.rs) instead.

use std::marker::PhantomData;

/// Transform state placeholder for SWC-based transformations.
///
/// This struct exists for structural parity with the OXC implementation,
/// but is not actively used since SWC's `VisitMut` trait does not accept
/// a state parameter. State management in SWC is handled by storing fields
/// directly in the visitor implementation struct.
#[derive(Default)]
pub struct TransformState<'a> {
    data: PhantomData<&'a ()>,
}
