// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::SessionCore;
use wow_data::progression_rewards::{FactionEntry, FriendshipRepReactionStore};

/// Narrow reads of the canonical Player values used by item-level valuation.
/// The capability retains no Player or manager guard.
pub struct InventoryValuationAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn inventory_valuation_access_like_cpp(&self) -> InventoryValuationAccessLikeCpp<'_> {
        InventoryValuationAccessLikeCpp { core: self }
    }
}

impl InventoryValuationAccessLikeCpp<'_> {
    pub fn packet_publication_access_like_cpp(&self) -> crate::session::PacketPublicationAccessLikeCpp<'_> {
        self.core.packet_publication_access_like_cpp()
    }

    pub fn player_guid_like_cpp(&self) -> Option<wow_core::ObjectGuid> {
        self.core.player_guid()
    }

    pub fn realm_id_like_cpp(&self) -> u16 {
        self.core.realm_id
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    /// Snapshot the canonical Unit gates used by inventory equipability.
    /// Missing canonical ownership retains the caller's existing false defaults.
    pub fn stunned_and_charmed_like_cpp(&self) -> (bool, bool) {
        self.core
            .canonical_player_snapshot_like_cpp(|player| {
                (
                    player
                        .unit()
                    .has_unit_state(wow_constants::UnitState::STUNNED.bits()),
                    player.unit().subsystems().control.is_charmed(),
                )
            })
            .unwrap_or((false, false))
    }

    pub fn is_charmed_like_cpp(&self) -> bool {
        self.core
            .canonical_player_snapshot_like_cpp(|player| {
                player.unit().subsystems().control.is_charmed()
            })
            .unwrap_or(false)
    }

    pub fn battleground_state_snapshot_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture: &crate::session::BattlegroundState,
    ) -> Option<wow_entities::PlayerBattlegroundState> {
        self.core
            .player_battleground_state_snapshot_with_fixture_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture,
            )
    }

    pub fn resolved_in_combat_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_in_combat: &bool,
    ) -> Option<bool> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.unit().subsystems().combat.has_combat()
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return Some(*fixture_in_combat);
        }
        None
    }

    pub fn using_pvp_item_levels_like_cpp(&self) -> Option<bool> {
        self.core.with_owned_player_like_cpp(|player| {
            player.gameplay_state().using_pvp_item_levels
        })
    }

    pub fn activate_pvp_item_levels_like_cpp(&self, active: bool) -> bool {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.activate_pvp_item_levels_like_cpp(active)
        }).is_some()
    }

    pub fn equip_capabilities_like_cpp(&self) -> Option<(bool, bool)> {
        self.core.with_owned_player_like_cpp(|player| {
            (
                player.unit().can_dual_wield_like_cpp(),
                player.can_titan_grip(),
            )
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    pub fn player_level_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
    ) -> u8 {
        self.core
            .player_level_with_fixture_like_cpp(#[cfg(any(test, feature = "test-fixtures"))] fixture_level)
    }

    /// Resolve the required-reputation item gate without exposing the owned
    /// reputation manager or retaining its canonical-player borrow.
    pub fn reputation_rank_for_faction_like_cpp(
        &self,
        faction: &FactionEntry,
        player_race: u8,
        player_class: u8,
        friendship_store: Option<&FriendshipRepReactionStore>,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_reputation: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> Option<u32> {
        let rank = |standing| {
            u32::from(
                wow_progression::reputation_to_rank_like_cpp(
                    faction,
                    standing,
                    friendship_store,
                )
                .as_u8(),
            )
        };
        let canonical_standing = self.core.with_owned_player_like_cpp(|player| {
            let manager = wow_progression::ReputationMgrLikeCpp::borrowing_like_cpp(
                player.reputation_like_cpp(),
            );
            manager.reputation_for_faction_like_cpp(
                faction,
                player_race,
                player_class,
            )
        });
        if let Some(standing) = canonical_standing {
            return Some(rank(standing));
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            let manager =
                wow_progression::ReputationMgrLikeCpp::borrowing_like_cpp(fixture_reputation);
            let standing = manager.reputation_for_faction_like_cpp(
                faction,
                player_race,
                player_class,
            );
            return Some(rank(standing));
        }
        None
    }
}
