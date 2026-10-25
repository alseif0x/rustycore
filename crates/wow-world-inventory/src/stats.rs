// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_world_core::session::state::hub_support::RepresentedPlayerGearStatsLikeCpp;
use wow_world_core::session::{HubMut, HubRef, PlayerStatsAccessLikeCpp};

/// C++ `CombatRating::CR_HIT_MELEE` (`Unit.h:310`).
pub const CR_HIT_MELEE_LIKE_CPP: u8 = 5;

impl crate::InventoryState {
    pub fn apply_represented_shapeshift_base_attack_time_with_access_like_cpp(
        &self,
        player: &PlayerStatsAccessLikeCpp<'_>,
    ) -> bool {
        let regular = self
            .represented_player_gear_stats_with_access_like_cpp(player)
            .map(|gear| gear.base_attack_time);
        let combat_round_time = player.represented_shapeshift_combat_round_time_like_cpp();
        player.apply_shapeshift_base_attack_times_like_cpp(regular, combat_round_time)
    }

    pub fn represented_player_gear_stats_like_cpp(
        &self,
        hub: HubRef<'_>,
        _include_represented_item_bonuses: bool,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        // C++ keeps `_ApplyItemBonuses` on Player and uses that accumulator for
        // subsequent stats. Reading inventory rows here as well would count
        // those same contributions twice after an equip or swap.
        let bonuses = self.resolved_item_bonus_state_like_cpp(hub)?;
        Some(crate::stat_planning::gear_from_bonuses(&bonuses))
    }

    pub fn represented_player_gear_stats_with_access_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        let canonical = access.item_modifier_runtime_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let canonical = if canonical.is_none() && access.owner_handle_absent_like_cpp() {
            Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .clone(),
            )
        } else {
            canonical
        };
        let bonuses = canonical?.bonuses_snapshot_like_cpp();
        Some(crate::stat_planning::gear_from_bonuses(&bonuses))
    }

    pub fn apply_represented_shapeshift_base_attack_time_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> bool {
        let regular = self
            .represented_player_gear_stats_like_cpp(hub.shared(), true)
            .map(|gear| gear.base_attack_time);
        let combat_round_time = hub
            .shared()
            .represented_shapeshift_combat_round_time_like_cpp();
        hub.core
            .apply_shapeshift_base_attack_times_like_cpp(regular, combat_round_time)
    }
}
