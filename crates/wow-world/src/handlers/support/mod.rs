// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Support, GM-ticket and client object-update handlers.
//!
//! The support handlers and their registration moved to
//! `wow-world-lifecycle` under #1263 F5. The session keeps only these
//! test-only entry points so the existing scenario module can drive one
//! handler without composing a dispatch table.

#[cfg(test)]
mod session_shims {
    use wow_packet::WorldPacket;
    use wow_world_lifecycle::SupportHandlerCxLikeCpp;

    use crate::session::WorldSession;

    impl WorldSession {
        pub(crate) async fn handle_gm_ticket_get_case_status(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_gm_ticket_get_case_status(pkt)
                .await;
        }

        pub(crate) async fn handle_gm_ticket_get_system_status(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_gm_ticket_get_system_status(pkt)
                .await;
        }

        pub(crate) async fn handle_gm_ticket_acknowledge_survey(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_gm_ticket_acknowledge_survey(pkt)
                .await;
        }

        pub(crate) async fn handle_complaint(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_complaint(pkt)
                .await;
        }

        pub(crate) async fn handle_submit_user_feedback(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_submit_user_feedback(pkt)
                .await;
        }

        pub(crate) async fn handle_support_ticket_submit_bug(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_support_ticket_submit_bug(pkt)
                .await;
        }

        pub(crate) async fn handle_support_ticket_submit_complaint(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_support_ticket_submit_complaint(pkt)
                .await;
        }

        pub(crate) async fn handle_support_ticket_submit_suggestion(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_support_ticket_submit_suggestion(pkt)
                .await;
        }

        pub(crate) async fn handle_bug_report(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_bug_report(pkt)
                .await;
        }

        pub(crate) async fn handle_object_update_failed(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_object_update_failed(pkt)
                .await;
        }

        pub(crate) async fn handle_object_update_rescued(&mut self, pkt: WorldPacket) {
            let policy = self.support_feature_policy_for_test_like_cpp();
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            SupportHandlerCxLikeCpp::new(lifecycle, hub, &policy)
                .handle_object_update_rescued(pkt)
                .await;
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/support/tests/mod.rs"]
mod tests;
