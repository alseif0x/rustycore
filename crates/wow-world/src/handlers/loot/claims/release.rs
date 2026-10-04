// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot release transitions and close-out operations.

use super::*;

impl WorldSession {
    /// Build the application-owned release context from disjoint WorldSession
    /// borrows. The context owns the mutable release participants; the fixture
    /// bundle stays a single mutable owner so its stats pass and readonly
    /// projections cannot alias.
    pub(in crate::handlers::loot) fn loot_release_cx_like_cpp(
        &mut self,
    ) -> wow_world_application::LootReleaseCxLikeCpp<'_> {
        wow_world_application::LootReleaseCxLikeCpp::new(
            self.core.loot_release_owner_access_like_cpp(),
            &mut self.loot,
            &mut self.world_entities,
            &mut self.inventory,
            &mut self.lifecycle,
            cfg!(test),
            &self.instances,
            wow_world_core::session::LootReleaseStatsInputsLikeCpp::new_like_cpp(
                &self.catalogs,
                &self.config,
            ),
            &self.quest_state,
            &self.social,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut self.fixtures,
            self.catalogs.item_store(),
            self.catalogs.item_stats_store(),
        )
    }

    pub(crate) async fn do_loot_release_all_like_cpp(&mut self, player_guid: ObjectGuid) {
        self.loot_release_cx_like_cpp()
            .release_all_like_cpp(player_guid)
            .await;
    }

    pub(in crate::handlers::loot) async fn do_loot_release_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        self.loot_release_cx_like_cpp()
            .release_owner_like_cpp(owner_guid, player_guid)
            .await
    }
}

#[cfg(test)]
#[path = "../../../../unit_tests/handlers/loot/claims/release/f3_shims.rs"]
mod f3_shims;
