// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the cross-domain group handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! splits its social and lifecycle state from the hub.

use wow_world_application::{
    GroupHandlerCxLikeCpp, GroupHandlerHostLikeCpp, GroupPublicationTailLikeCpp,
};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl GroupHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn group_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> GroupHandlerCxLikeCpp<'a> {
        let (social, lifecycle, hub) = crate::session::split_social_lifecycle_mut(self);
        GroupHandlerCxLikeCpp::new(social, lifecycle, hub)
    }

    fn run_group_publication_tail_like_cpp(
        &mut self,
        tail: wow_world_application::GroupPublicationTailLikeCpp,
    ) {
        self.apply_group_publication_tail_like_cpp(tail);
    }
}

impl WorldSession {
    /// Runs the deferred group publication tail in C++ order (#1263 F5).
    ///
    /// The registry-state sync and the visible-gameobject refresh still need the
    /// World session's stats/loot/control and map providers, so the application
    /// owner returns the exact remaining sequence and the session runs it here,
    /// between the canonical transition and the final packets.
    pub(crate) fn apply_group_publication_tail_like_cpp(
        &mut self,
        tail: GroupPublicationTailLikeCpp,
    ) {
        use wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP;
        match tail {
            GroupPublicationTailLikeCpp::None => {}
            GroupPublicationTailLikeCpp::VisibilityOnly => {
                let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            }
            GroupPublicationTailLikeCpp::PartyUpdateOnly { group } => {
                self.send_group_party_update_after_owner_like_cpp(&group);
            }
            GroupPublicationTailLikeCpp::SyncThenPartyUpdate { group } => {
                self.sync_player_registry_state_like_cpp();
                self.send_group_party_update_after_owner_like_cpp(&group);
            }
            GroupPublicationTailLikeCpp::SyncThenVisibilityThenGroupUninvite => {
                self.sync_player_registry_state_like_cpp();
                let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
                self.send_packet_realm(&wow_packet::packets::party::GroupUninvite);
            }
            GroupPublicationTailLikeCpp::SyncThenVisibilityThenGroupDestroyed { group_guid } => {
                self.sync_player_registry_state_like_cpp();
                let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
                self.send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
                self.send_destroyed_group_party_update_like_cpp(
                    group_guid,
                    GROUP_CATEGORY_HOME_LIKE_CPP,
                );
            }
        }
    }

    fn send_group_party_update_after_owner_like_cpp(&self, group: &wow_social::group::GroupInfo) {
        let Some(registry) = self.player_registry() else {
            return;
        };
        wow_world_social::group_fanout::send_party_update(
            group,
            registry,
            self.core.virtual_realm_address(),
        );
    }
}
