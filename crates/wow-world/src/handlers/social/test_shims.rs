// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the social contact handlers moved to
//! `wow-world-social` (#1263 F5).
//!
//! The scenario suites still build a `WorldSession` here, so these bounded
//! delegates construct the social owner's context from the session's
//! lifecycle state and hub.

use wow_world_social::SocialContactsHandlerCxLikeCpp;

use super::*;

impl WorldSession {
    pub async fn handle_del_ignore(&mut self, ignore: DelIgnore) {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        let persistence = lifecycle.social_persistence_port_like_cpp();
        SocialContactsHandlerCxLikeCpp::new(hub, persistence)
            .handle_del_ignore(ignore)
            .await;
    }

    pub async fn handle_social_contract_request(&mut self) {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        let persistence = lifecycle.social_persistence_port_like_cpp();
        SocialContactsHandlerCxLikeCpp::new(hub, persistence)
            .handle_social_contract_request()
            .await;
    }

    pub async fn handle_accept_social_contract(&mut self, accept: AcceptSocialContract) {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        let persistence = lifecycle.social_persistence_port_like_cpp();
        SocialContactsHandlerCxLikeCpp::new(hub, persistence)
            .handle_accept_social_contract(accept)
            .await;
    }

    pub async fn handle_account_notification_acknowledged(
        &mut self,
        packet: AccountNotificationAcknowledged,
    ) {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        let persistence = lifecycle.social_persistence_port_like_cpp();
        SocialContactsHandlerCxLikeCpp::new(hub, persistence)
            .handle_account_notification_acknowledged(packet)
            .await;
    }
}
