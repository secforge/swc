use serde::Deserialize;

/// Proposal transformation options
///
/// This module provides options for ECMAScript proposal features
/// that are not yet part of the standard.
///
/// Currently, most proposal transformations have been standardized
/// and moved to their respective ES version modules (e.g., ES2020, ES2021).
/// This struct is kept for future proposal transformations.
#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ProposalOptions {
    // Add proposal options as needed when new proposals are implemented
}
