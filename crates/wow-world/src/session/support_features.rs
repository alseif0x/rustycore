// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Support features: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{FeatureSystemStatus, FeatureSystemStatusGlueScreen, WorldSession};

#[cfg(test)]
pub(crate) mod test_fixtures;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_support_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_support_tickets_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_tickets_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_tickets_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_tickets_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_support_bugs_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_bugs_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_bugs_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_bugs_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_bug_system_status_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp
            && self
                .support_feature_test_fixture_like_cpp
                .represented_support_bugs_enabled_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_support_complaints_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_complaints_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_complaints_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_complaints_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_complaint_system_status_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp
            && self
                .support_feature_test_fixture_like_cpp
                .represented_support_complaints_enabled_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_support_suggestions_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_suggestions_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_suggestions_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_suggestions_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_suggestion_system_status_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp
            && self
                .support_feature_test_fixture_like_cpp
                .represented_support_suggestions_enabled_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn feature_system_status_like_cpp(&self) -> FeatureSystemStatus {
        self.feature_system_status_with_policy_like_cpp(
            &self.support_feature_policy_for_test_like_cpp(),
        )
    }

    #[cfg(test)]
    pub(crate) fn feature_system_status_glue_screen_like_cpp(
        &self,
    ) -> FeatureSystemStatusGlueScreen {
        self.feature_system_status_glue_screen_with_policy_like_cpp(
            &self.support_feature_policy_for_test_like_cpp(),
        )
    }
}
