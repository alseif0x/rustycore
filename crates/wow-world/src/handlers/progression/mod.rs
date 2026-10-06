// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Progression (reputation) command handlers.
//!
//! The reputation handlers and their registration moved to
//! `wow-world-application` under #1263 F5. The session keeps only these
//! test-only entry points so the existing scenario module can drive one
//! handler without composing a dispatch table.

#[cfg(any(test, feature = "test-fixtures"))]
mod session_shims {
    use wow_packet::WorldPacket;
    use wow_world_application::ReputationHandlerCxLikeCpp;

    use crate::session::WorldSession;

    impl WorldSession {
        fn reputation_test_cx_like_cpp(&mut self) -> ReputationHandlerCxLikeCpp<'_> {
            self.build_reputation_handler_cx_like_cpp()
        }

        pub(crate) async fn handle_request_forced_reactions(&mut self, pkt: WorldPacket) {
            self.reputation_test_cx_like_cpp()
                .handle_request_forced_reactions(pkt)
                .await;
        }

        pub(crate) async fn handle_set_faction_at_war(&mut self, pkt: WorldPacket) {
            self.reputation_test_cx_like_cpp()
                .handle_set_faction_at_war(pkt)
                .await;
        }

        pub(crate) async fn handle_set_faction_not_at_war(&mut self, pkt: WorldPacket) {
            self.reputation_test_cx_like_cpp()
                .handle_set_faction_not_at_war(pkt)
                .await;
        }

        pub(crate) async fn handle_set_faction_inactive(&mut self, pkt: WorldPacket) {
            self.reputation_test_cx_like_cpp()
                .handle_set_faction_inactive(pkt)
                .await;
        }

        pub(crate) async fn handle_set_watched_faction(&mut self, pkt: WorldPacket) {
            self.reputation_test_cx_like_cpp()
                .handle_set_watched_faction(pkt)
                .await;
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/progression/tests/mod.rs"]
mod tests;
