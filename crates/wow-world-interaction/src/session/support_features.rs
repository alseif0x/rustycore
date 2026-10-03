use crate::InteractionState;

impl InteractionState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_support_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_support_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_support_tickets_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_tickets_enabled_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_support_tickets_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_tickets_enabled_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_support_bugs_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_bugs_enabled_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_support_bugs_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_bugs_enabled_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_bug_system_status_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp
            && self
                .support_feature_test_fixture_like_cpp
                .represented_support_bugs_enabled_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_support_complaints_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_complaints_enabled_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_support_complaints_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_complaints_enabled_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_complaint_system_status_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp
            && self
                .support_feature_test_fixture_like_cpp
                .represented_support_complaints_enabled_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_support_suggestions_enabled_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_suggestions_enabled_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_support_suggestions_enabled_like_cpp(&mut self, enabled: bool) {
        self.support_feature_test_fixture_like_cpp
            .represented_support_suggestions_enabled_like_cpp = enabled;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_suggestion_system_status_like_cpp(&self) -> bool {
        self.support_feature_test_fixture_like_cpp
            .represented_support_enabled_like_cpp
            && self
                .support_feature_test_fixture_like_cpp
                .represented_support_suggestions_enabled_like_cpp
    }
}
