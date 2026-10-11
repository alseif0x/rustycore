//! Creature and respawn fixtures shared by the creature runtime tests.
//!
//! #1263 F6-8E-2: split out of the test module root when the legacy
//! `WorldCreature` fixture became a canonical `Creature` fixture.

use super::*;

/// The former `WorldCreature::new` fixture on the canonical creature. Its
/// cached projection launched splines at the projection's run rate (1.14286);
/// the canonical unit now carries that rate itself.
#[allow(clippy::too_many_arguments)]
pub(super) fn fixture_creature(
    guid: ObjectGuid,
    entry: u32,
    pos: Position,
    hp: u32,
    level: u8,
    min_dmg: u32,
    max_dmg: u32,
    aggro_radius: f32,
    display_id: u32,
    faction: u32,
    npc_flags: u32,
    unit_flags: u32,
) -> Creature {
    let mut creature = new_runtime_creature_like_cpp(
        guid,
        entry,
        pos,
        hp,
        level,
        min_dmg,
        max_dmg,
        aggro_radius,
        display_id,
        faction,
        npc_flags,
        unit_flags,
    );
    creature
        .unit_mut()
        .set_speed_rate_like_cpp(UnitMoveType::Run, 1.14286);
    creature
}

pub(super) fn test_creature(guid: ObjectGuid) -> Creature {
    fixture_creature(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    )
}

// ── Slice 4A.1a tests ────────────────────────────────────────────────────

// ── Slice 4A.2a: respawn queue tests ──────────────────────────────────────

pub(super) fn make_pending_respawn(respawn_at: Instant) -> PendingRespawn {
    use wow_packet::packets::update::CreatureCreateData;
    static NEXT_TEST_SPAWN_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let spawn_id = NEXT_TEST_SPAWN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        0,
        0,
        1,
        spawn_id as i64,
    );
    PendingRespawn {
        respawn_at,
        spawn_id,
        persistent_spawn: true,
        home_pos: Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: 0.0,
        },
        create_data: CreatureCreateData {
            guid,
            entry: 1,
            display_id: 1,
            native_display_id: 1,
            display_scale: 1.0,
            native_x_display_scale: 1.0,
            bounding_radius: 0.389,
            combat_reach: 1.5,
            health: 100,
            max_health: 100,
            level: 1,
            faction_template: 1,
            npc_flags: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            aura_state: Creature::health_aura_state_like_cpp(100, 100, true),
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            display_power: 1,
            power: [0; 10],
            max_power: [0; 10],
            base_mana: 0,
            virtual_items: [(0, 0, 0); 3],
            base_attack_time: 2000,
            ranged_attack_time: 0,
            movement_flags: 0,
            vehicle_id: 0,
            play_hover_anim: false,
            hover_height: 1.0,
            mount_display_id: 0,
            stand_state: 0,
            vis_flags: 0,
            anim_tier: 0,
            emote_state: 0,
            sheathe_state: wow_constants::unit::SheathState::Melee as u8,
            pvp_flags: 0,
            current_area_id: 0,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        },
        max_hp: 100,
        level: 1,
        min_dmg: 1,
        max_dmg: 5,
        combat_log_stats: wow_entities::CreatureCombatLogStatsLikeCpp::default(),
        spell_hit_aura_source_authority_like_cpp: false,
        spell_cast_log_aura_source_authority_like_cpp: false,
        aggro_radius: 10.0,
        wander_distance: 0.0,
        flags_extra: 0,
        static_flags: [0; 8],
        ai_name: String::new(),
        script_name: String::new(),
        string_id: None,
        addon: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        default_movement_type: MovementGeneratorType::Idle,
        waypoint_path_id: 0,
        npc_flags: 0,
        unit_flags: 0,
        map_id: 0,
        loot_id: 0,
        skin_loot_id: 0,
        gold_min: 0,
        gold_max: 0,
        respawn_delay_secs: 30,
        selected_equipment_id: 0,
        original_equipment_id: 0,
        boss_id: None,
        dungeon_encounter_id: 0,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        terrain_swap_map: -1,
        phase_shift: PhaseShift::default(),
    }
}

/// Unique temp `maps/` dir holding one synthetic constant-height tile.
pub(super) fn temp_dir_with_constant_tile(map_id: u32, gx: i32, gy: i32, height: f32) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("rustycore_live_terrain_{}_{n}", std::process::id()));
    std::fs::create_dir_all(dir.join("maps")).expect("create temp maps dir");

    // Minimal float `.map`: fileheader(44) + MHGT header(16) + V9 + V8, all = height.
    const V9: usize = 129 * 129;
    const V8: usize = 128 * 128;
    let mut b = Vec::new();
    b.extend_from_slice(b"MAPS");
    b.extend_from_slice(&10u32.to_le_bytes()); // version
    b.extend_from_slice(&0u32.to_le_bytes()); // build
    b.extend_from_slice(&0u32.to_le_bytes()); // areaMapOffset
    b.extend_from_slice(&0u32.to_le_bytes()); // areaMapSize
    b.extend_from_slice(&44u32.to_le_bytes()); // heightMapOffset
    for _ in 0..5 {
        b.extend_from_slice(&0u32.to_le_bytes());
    }
    b.extend_from_slice(b"MHGT");
    b.extend_from_slice(&0u32.to_le_bytes()); // flags = float
    b.extend_from_slice(&height.to_le_bytes()); // gridHeight
    b.extend_from_slice(&height.to_le_bytes()); // gridMaxHeight
    for _ in 0..(V9 + V8) {
        b.extend_from_slice(&height.to_le_bytes());
    }
    std::fs::write(
        dir.join("maps")
            .join(format!("{map_id:04}_{gx:02}_{gy:02}.map")),
        &b,
    )
    .expect("write tile");
    dir
}
