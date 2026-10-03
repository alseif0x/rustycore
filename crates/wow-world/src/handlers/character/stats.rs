// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character equipment aggregation and effective stat publication.
//!
//! This module owns the application-side projection from equipped item data and
//! Player-owned item modifiers into the pure `wow-data` stat calculation. The
//! resulting snapshot is published to the canonical Player; packet formatting
//! remains in the character handler adapters.

use super::*;
pub(crate) use crate::session::hub_support::RepresentedPlayerGearStatsLikeCpp;
use crate::session::hub_support::{
    SPELL_SCHOOL_MASK_ALL_LIKE_CPP, SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
    SPELL_SCHOOL_MASK_SPELL_LIKE_CPP,
};

impl WorldSession {
    pub(super) fn stats_application_cx_like_cpp(
        &mut self,
    ) -> wow_world_application::CharacterStatsApplicationCxLikeCpp<'_> {
        let (inventory, hub) = crate::session::split_inventory_mut(self);
        let core = &*hub.core;
        let publication = core.packet_publication_access_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let player = core.player_stats_access_with_fixture_refs_like_cpp(
            hub.catalogs,
            hub.config,
            &hub.fixtures.identity.player_race,
            &hub.fixtures.identity.player_class,
            &hub.fixtures.identity.player_level,
            wow_world_core::session::StatsFixtureRefs::new_like_cpp(
                wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                    &mut hub.fixtures.combat.player_health_like_cpp,
                    &mut hub.fixtures.combat.player_max_health_like_cpp,
                    &mut hub.fixtures.combat.player_alive_like_cpp,
                    &mut hub.fixtures.combat.represented_player_powers_like_cpp[0],
                    &mut hub.fixtures.combat.represented_player_max_powers_like_cpp[0],
                    &mut hub.fixtures.combat.represented_player_base_mana_like_cpp,
                ),
                wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                    &hub.fixtures.auras.represented_shapeshift_form_like_cpp,
                    &hub.fixtures.auras.player_aura_authority_complete_like_cpp,
                    &hub.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                    &hub.fixtures.auras.visible_auras,
                    &hub.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                ),
            ),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let player = core.player_stats_access_like_cpp(hub.catalogs, hub.config);
        wow_world_application::CharacterStatsApplicationCxLikeCpp::new(
            player,
            inventory,
            publication,
        )
    }

    pub(super) fn represented_player_gear_stats_like_cpp(
        &mut self,
        _include_represented_item_bonuses: bool,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        self.stats_application_cx_like_cpp()
            .represented_player_gear_stats_like_cpp()
    }

    pub(super) fn player_stat_system_projection_like_cpp(
        &mut self,
        race: u8,
        class: u8,
        level: u8,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> Option<PlayerStatSystemProjectionLikeCpp> {
        self.stats_application_cx_like_cpp()
            .player_stat_system_projection_like_cpp(race, class, level, gear)
    }

    pub(crate) fn apply_represented_shapeshift_base_attack_time_like_cpp(&mut self) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_represented_shapeshift_base_attack_time_like_cpp(&mut hub)
    }

    /// C++ `Player::InitDataForForm` plus the `UpdateDamagePhysical` refresh at
    /// a shapeshift aura apply/removal: reinstall the base attack times and
    /// republish the weapon ranges the form rescales.
    pub(crate) fn sync_represented_shapeshift_form_like_cpp(
        &mut self,
        mutation: crate::session::RepresentedShapeshiftMutationLikeCpp,
    ) {
        if !self.apply_represented_shapeshift_base_attack_time_like_cpp() {
            return;
        }
        // C++ `AuraEffect::HandleShapeshiftBoosts`
        // (`SpellAuraEffects.cpp:1394-1464`) owns the form's bonus spells and
        // the stance-gated self-aura sweep on both directions.
        match mutation {
            crate::session::RepresentedShapeshiftMutationLikeCpp::Applied { form_id } => {
                self.apply_represented_shapeshift_boosts_like_cpp(form_id);
            }
            crate::session::RepresentedShapeshiftMutationLikeCpp::Removed {
                removed_form,
                new_form,
            } => {
                self.remove_represented_shapeshift_boosts_like_cpp(removed_form, new_form);
            }
        }
        // C++ `Player::InitDataForForm` (`Player.cpp:22090`) updates the displayed
        // power before refreshing the equipped items' form-gated spells and
        // item-set auras, and `UpdateAttackPowerAndDamage` republishes after.
        self.sync_represented_display_power_like_cpp();
        self.refresh_represented_item_effects_at_form_change_like_cpp();
        let _ = self.send_stat_update();
    }

}
