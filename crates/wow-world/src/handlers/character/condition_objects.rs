// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Condition-object projections used by character handlers.
//!
//! These builders are kept beside the character state adapter because they
//! translate the canonical Player/Creature projections into the condition
//! engine's immutable snapshots; they do not own condition evaluation.

use super::*;

impl WorldSession {
    pub(crate) fn build_condition_player_object_like_cpp(&self) -> Option<WorldObject> {
        let mut player = WorldObject::new(
            false,
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );
        player.object_mut().create(self.player_guid()?);
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let _ = player.set_map(u32::from(self.player_map_id_like_cpp()), instance_id);
        let (zone_id, area_id) = self.player_zone_area_like_cpp()?;
        player.set_zone_and_area(zone_id, area_id);
        if let Some(position) = self.player_position_like_cpp() {
            player.relocate(position);
        }
        Some(player)
    }

    pub(crate) fn build_condition_creature_object_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
    ) -> Option<(WorldObject, wow_conditions::ConditionUnitSnapshot)> {
        self.mutate_world_creature(npc_guid, |creature| {
            let mut source =
                WorldObject::new(false, TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
            source.object_mut().create(creature.guid());
            source.object_mut().set_entry(creature.entry());
            let _ = source.set_map(creature.map_id(), creature.instance_id());
            source.relocate(creature.position());
            *source.phase_shift_mut() = creature.phase_shift().clone();
            let snapshot = wow_conditions::ConditionUnitSnapshot {
                level: u32::from(creature.level()),
                health: u64::from(creature.current_hp()),
                max_health: u64::from(creature.max_hp()),
                class_mask: 0,
                race: 0,
                creature_type: None,
                is_alive: creature.is_alive(),
                is_charmed: false,
                in_water: false,
                unit_state: 0,
                stand_state: UnitStandStateType::Stand as u32,
            };
            (source, snapshot)
        })
    }

    pub(crate) fn condition_player_unit_snapshot_like_cpp(
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

    pub(crate) fn condition_player_snapshot_like_cpp(
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
