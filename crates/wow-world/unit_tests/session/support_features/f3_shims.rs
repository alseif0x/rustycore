// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_support_enabled_like_cpp(&self) -> bool {
        self.interaction.represented_support_enabled_like_cpp()
    }
    #[cfg(test)]
    pub fn set_represented_support_enabled_like_cpp(&mut self, enabled: bool) {
        self.interaction
            .set_represented_support_enabled_like_cpp(enabled)
    }
    #[cfg(test)]
    pub(crate) fn represented_support_bugs_enabled_like_cpp(&self) -> bool {
        self.interaction.represented_support_bugs_enabled_like_cpp()
    }
    #[cfg(test)]
    pub fn set_represented_support_bugs_enabled_like_cpp(&mut self, enabled: bool) {
        self.interaction
            .set_represented_support_bugs_enabled_like_cpp(enabled)
    }
    #[cfg(test)]
    pub(crate) fn represented_bug_system_status_like_cpp(&self) -> bool {
        self.interaction.represented_bug_system_status_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_support_complaints_enabled_like_cpp(&self) -> bool {
        self.interaction
            .represented_support_complaints_enabled_like_cpp()
    }
    #[cfg(test)]
    pub fn set_represented_support_complaints_enabled_like_cpp(&mut self, enabled: bool) {
        self.interaction
            .set_represented_support_complaints_enabled_like_cpp(enabled)
    }
    #[cfg(test)]
    pub(crate) fn represented_complaint_system_status_like_cpp(&self) -> bool {
        self.interaction
            .represented_complaint_system_status_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_support_suggestions_enabled_like_cpp(&self) -> bool {
        self.interaction
            .represented_support_suggestions_enabled_like_cpp()
    }
    #[cfg(test)]
    pub fn set_represented_support_suggestions_enabled_like_cpp(&mut self, enabled: bool) {
        self.interaction
            .set_represented_support_suggestions_enabled_like_cpp(enabled)
    }
    #[cfg(test)]
    pub(crate) fn represented_suggestion_system_status_like_cpp(&self) -> bool {
        self.interaction
            .represented_suggestion_system_status_like_cpp()
    }
}
