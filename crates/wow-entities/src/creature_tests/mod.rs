//! Creature regressions.
//!
//! Separated from the creature_tests.rs root under #648.

//! Behaviour tests for [`super`].
//!
//! Extracted from `creature.rs`, which was 6,940 lines of which
//! 3,090 — 45% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::*;
use crate::MovementGeneratorKind;
use crate::{
    AURA_STATE_DEFENSIVE, AURA_STATE_DEFENSIVE_2, AppliedAuraRef, AuraRef, CurrentSpellRef,
    CurrentSpellSlot, DIMINISHING_STUN, DiminishingLevel, OwnedAuraRef,
};
use wow_constants::SpellState;
use wow_core::guid::HighGuid;

fn formation_info_like_cpp(leader_spawn_id: u64) -> CreatureFormationInfoLikeCpp {
    CreatureFormationInfoLikeCpp {
        leader_spawn_id,
        follow_dist: 7.0,
        follow_angle_radians: 1.25,
        group_ai: 3,
        leader_waypoint_ids: [11, 12],
    }
}

fn owned_loot_fixture_like_cpp(
    coins: u32,
    unlooted_count: u8,
    allowed_looters: Vec<ObjectGuid>,
) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins,
        unlooted_count,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters,
        items: Vec::new(),
        looted_by_player: false,
    }
}

fn poll_immediately_ready<F: std::future::Future>(future: F) -> F::Output {
    struct NoopWake;

    impl std::task::Wake for NoopWake {
        fn wake(self: std::sync::Arc<Self>) {}
    }

    let waker = std::task::Waker::from(std::sync::Arc::new(NoopWake));
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => panic!("expected the uncontended claim to be ready"),
    }
}

fn creature_lifecycle_template() -> CreatureTemplateLifecycleRecord {
    let mut spells = [0; MAX_CREATURE_SPELLS];
    spells[0] = 133;
    spells[3] = 116;
    CreatureTemplateLifecycleRecord {
        entry: 1001,
        original_entry: 9001,
        difficulty_id: 2,
        name: "lifecycle wolf".to_string(),
        ai_name: "SmartAI".to_string(),
        script_name: "npc_lifecycle_wolf".to_string(),
        required_expansion: 2,
        unit_class: 1,
        trainer_class: 4,
        faction: 14,
        npc_flags: 0x1_0000_0040,
        display_id: 2001,
        model_dimensions: Some(CreatureModelDimensions {
            bounding_radius: 0.4,
            combat_reach: 1.2,
        }),
        scale: 1.5,
        speed_walk: 0.8,
        speed_run: 1.25,
        spells,
        classification: 3,
        damage_school: wow_constants::spell::SpellSchools::Nature as u8,
        unit_flags: UnitFlags::IMMUNE_TO_NPC.bits(),
        unit_flags2: UnitFlags2::FEIGN_DEATH.bits(),
        unit_flags3: UnitFlags3::AI_OBSTACLE.bits(),
        flags_extra: CreatureFlagsExtra::CIVILIAN.bits()
            | CreatureFlagsExtra::USE_OFFHAND_ATTACK.bits(),
        static_flags: [0; 8],
        creature_type: 9,
        type_flags: 0x20,
        loot_id: 7_001,
        skin_loot_id: 7_002,
        gold_min: 17,
        gold_max: 29,
        movement_type: MovementGeneratorType::Idle,
        ground_movement_type: CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: CreatureFlightMovementType::DisableGravity as u8,
        rooted: false,
        chase_movement_type: CreatureChaseMovementType::Run as u8,
        random_movement_type: CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        min_level: 70,
        max_level: 72,
        equipment_id: 4,
        original_equipment_id: -4,
    }
}

fn creature_lifecycle_spawn() -> CreatureSpawnLifecycleRecord {
    CreatureSpawnLifecycleRecord {
        spawn_id: 44_000,
        map_id: 571,
        instance_id: 3,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        home_position: Position::new(5.0, 6.0, 7.0, 1.0),
        phase_id: Some(169),
        phase_group: Some(12),
        terrain_swap_map: Some(609),
        spawn_group_id: Some(77),
        spawn_group_name: Some("lifecycle group".to_string()),
        pool_id: Some(88),
        equipment_id: Some(9),
        original_equipment_id: Some(-9),
        wander_distance: 12.5,
        respawn_delay: 45,
        respawn_time: 123_456,
        movement_type: MovementGeneratorType::Idle,
        string_id: Some("creature-string".to_string()),
        is_active: false,
        inactive_by_spawn_group: true,
        duplicate_spawn_found: true,
        add_to_map: true,
        respawn_compatibility_mode: true,
    }
}

fn vehicle_seat_def(
    seat_index: i8,
    can_enter_or_exit: bool,
) -> (i8, VehicleSeatInfo, VehicleSeatAddon) {
    (
        seat_index,
        VehicleSeatInfo {
            id: 10_000 + u32::from(seat_index.unsigned_abs()),
            attachment_offset: Position::ZERO,
            can_enter_or_exit,
            usable_by_override: false,
            can_control: false,
            can_switch_from_seat: false,
            ejectable: false,
            disables_gravity: false,
            passenger_not_selectable: false,
            keep_pet: false,
        },
        VehicleSeatAddon::default(),
    )
}

fn creature_lifecycle_create_record() -> CreatureCreateLifecycleRecord {
    CreatureCreateLifecycleRecord {
        guid: ObjectGuid::new(8, 1001),
        entry: 1001,
        map_id: 571,
        instance_id: 3,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        dynamic: false,
        vehicle_id: Some(101),
        vehicle_kit_create_input: Some(VehicleKitCreateInputLikeCpp {
            vehicle_id: 101,
            creature_entry: 1001,
            loading: true,
            seat_defs: vec![vehicle_seat_def(0, true), vehicle_seat_def(2, false)],
        }),
        add_to_world_vehicle_reset_context: None,
        template: creature_lifecycle_template(),
        spawn: None,
        selected_level: 71,
        stats: CreatureLifecycleStats::new(5_000, 4_500, 1_000, 750),
        selected_display_id: 3001,
        selected_model_dimensions: Some(CreatureModelDimensions {
            bounding_radius: 0.5,
            combat_reach: 2.0,
        }),
        selected_equipment_id: 6,
        selected_original_equipment_id: -6,
        selected_virtual_items: [(10_001, 3, 4), (10_002, 5, 6), (0, 0, 0)],
        corpse_delay: 90,
        ignore_corpse_decay_ratio: true,
        addon: None,
    }
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
mod scenarios_4;
