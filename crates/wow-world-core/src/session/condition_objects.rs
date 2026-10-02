// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::state::hub_support::{player_class_mask, player_team_for_race_cpp};
use wow_constants::{TypeId, TypeMask, UnitStandStateType};
use wow_entities::WorldObject;

impl crate::session::HubRef<'_> {
    pub fn build_condition_player_object_like_cpp(&self) -> Option<WorldObject> {
        let mut player = WorldObject::new(
            false,
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );
        player.object_mut().create(self.core.player_guid()?);
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let _ = player.set_map(u32::from(self.core.player_map_id_like_cpp()), instance_id);
        let (zone_id, area_id) = self.player_zone_area_like_cpp()?;
        player.set_zone_and_area(zone_id, area_id);
        if let Some(position) = self.player_position_like_cpp() {
            player.relocate(position);
        }
        Some(player)
    }

    pub fn condition_player_unit_snapshot_like_cpp(
        &self,
    ) -> Option<wow_conditions::ConditionUnitSnapshot> {
        let (health, max_health, is_alive) = self.resolved_player_vitals_like_cpp()?;
        Some(wow_conditions::ConditionUnitSnapshot {
            level: u32::from(self.player_level_like_cpp()),
            health: u64::from(health),
            max_health: u64::from(max_health),
            class_mask: player_class_mask(self.player_class_like_cpp()),
            race: self.player_race_like_cpp(),
            creature_type: None,
            is_alive,
            is_charmed: false,
            in_water: false,
            unit_state: 0,
            stand_state: UnitStandStateType::Stand as u32,
        })
    }

    pub fn condition_player_snapshot_like_cpp(
        &self,
    ) -> wow_conditions::ConditionPlayerSnapshot {
        wow_conditions::ConditionPlayerSnapshot {
            team: player_team_for_race_cpp(self.player_race_like_cpp()) as u32,
            native_gender: u32::from(self.player_gender_like_cpp()),
            drunken_state: 0,
            can_be_game_master: false,
            is_game_master: false,
            pet_type: None,
            // A missing generation-checked Player cannot prove the negative;
            // keep condition evaluation fail-closed as if travel were active.
            is_in_flight: self.resolved_is_in_taxi_flight_like_cpp().unwrap_or(true),
        }
    }
}
