//! Runtime public texture library configuration.

use crate::config::RuntimeConfig;

pub use crate::config::definitions::{
    TEXTURE_LIBRARY_ENABLED_KEY, TEXTURE_LIBRARY_REVIEW_REQUIRED_KEY,
};

/// Whether the public texture library is open by default; defaults to true so
/// self-hosted skin sites expose browsing out of the box.
pub const DEFAULT_TEXTURE_LIBRARY_ENABLED: bool = true;
/// Whether library entries require moderator review before appearing by
/// default; defaults to true so public listings stay curated unless the
/// operator opts out.
pub const DEFAULT_TEXTURE_LIBRARY_REVIEW_REQUIRED: bool = true;

/// Snapshot of the runtime texture library policy flags, resolved once per
/// request path instead of re-reading individual keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeTextureLibraryPolicy {
    /// Whether the public texture library is open for browsing.
    pub enabled: bool,
    /// Whether entries must pass review before being listed publicly.
    pub review_required: bool,
}

impl RuntimeTextureLibraryPolicy {
    /// Resolves both policy flags from the runtime config store, applying the
    /// documented defaults when keys are missing.
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        Self {
            enabled: runtime_config
                .get_bool_or(TEXTURE_LIBRARY_ENABLED_KEY, DEFAULT_TEXTURE_LIBRARY_ENABLED),
            review_required: runtime_config.get_bool_or(
                TEXTURE_LIBRARY_REVIEW_REQUIRED_KEY,
                DEFAULT_TEXTURE_LIBRARY_REVIEW_REQUIRED,
            ),
        }
    }
}
