// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Account-data and client-state packet handlers.
//!
//! The account-data handlers and their registration moved to
//! `wow-world-lifecycle` under #1263 F5. The session keeps only these
//! test-only entry points so the existing scenario modules can drive one
//! handler without composing a dispatch table.

mod client_state;

#[cfg(any(test, feature = "test-fixtures"))]
mod session_shims {
    use wow_packet::WorldPacket;
    use wow_world_lifecycle::AccountDataHandlerCxLikeCpp;

    use crate::session::WorldSession;

    impl WorldSession {
        fn account_data_test_cx_like_cpp(&mut self) -> AccountDataHandlerCxLikeCpp<'_> {
            let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
            AccountDataHandlerCxLikeCpp::new(lifecycle, hub)
        }

        pub(crate) async fn handle_request_account_data(&mut self, pkt: WorldPacket) {
            self.account_data_test_cx_like_cpp()
                .handle_request_account_data(pkt)
                .await;
        }

        pub(crate) async fn handle_update_account_data(&mut self, pkt: WorldPacket) {
            self.account_data_test_cx_like_cpp()
                .handle_update_account_data(pkt)
                .await;
        }

        pub(crate) async fn handle_addon_list(&mut self, pkt: WorldPacket) {
            self.account_data_test_cx_like_cpp().handle_addon_list(pkt);
        }

        pub(crate) async fn handle_save_cuf_profiles(&mut self, pkt: WorldPacket) {
            self.account_data_test_cx_like_cpp()
                .handle_save_cuf_profiles(pkt);
        }

        pub(crate) async fn handle_tutorial(&mut self, pkt: WorldPacket) {
            self.account_data_test_cx_like_cpp().handle_tutorial(pkt);
        }
    }
}

#[cfg(test)]
#[path = "../../unit_tests/handlers/account_data/tests/mod.rs"]
mod tests;
