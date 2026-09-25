//! Test-only support-system configuration represented by a detached Session.

/// Mirrors the five process-wide C++ support feature switches for tests.
pub(crate) struct SupportFeatureTestFixtureLikeCpp {
    pub(in crate::session) represented_support_enabled_like_cpp: bool,
    pub(in crate::session) represented_support_tickets_enabled_like_cpp: bool,
    pub(in crate::session) represented_support_bugs_enabled_like_cpp: bool,
    pub(in crate::session) represented_support_complaints_enabled_like_cpp: bool,
    pub(in crate::session) represented_support_suggestions_enabled_like_cpp: bool,
}

impl Default for SupportFeatureTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            represented_support_enabled_like_cpp: true,
            represented_support_tickets_enabled_like_cpp: false,
            represented_support_bugs_enabled_like_cpp: false,
            represented_support_complaints_enabled_like_cpp: false,
            represented_support_suggestions_enabled_like_cpp: false,
        }
    }
}
