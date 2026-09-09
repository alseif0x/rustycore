//! Spawn scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn never_respawn_time_saturates_to_future_instant_instead_of_ready_now_like_cpp() {
    let now = Instant::now();
    let deadline = instant_from_respawn_time_like_cpp(i64::MAX, now, 1_700_000_000);

    assert!(deadline > now);
}
/// A newly created `MapInstance` starts with an empty respawn queue.
#[test]
fn respawn_queue_starts_empty_like_cpp() {
    let map = MapInstance::new(0, 0);
    assert_eq!(map.respawn_queue_len(), 0);
}
/// Pushing one entry increments the length to 1.
#[test]
fn push_respawn_increments_len_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();
    map.push_respawn(make_pending_respawn(now));
    assert_eq!(map.respawn_queue_len(), 1);
}
#[test]
fn push_respawn_replaces_later_duplicate_spawn_id_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();
    let later = now + Duration::from_secs(60);
    let earlier = now + Duration::from_secs(10);

    let mut first = make_pending_respawn(later);
    first.spawn_id = 42;
    let mut replacement = make_pending_respawn(earlier);
    replacement.spawn_id = 42;

    map.push_respawn(first);
    map.push_respawn(replacement);

    assert_eq!(map.respawn_queue_len(), 1);
    let ready = map.drain_ready_respawns(now + Duration::from_secs(11));
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].spawn_id, 42);
}
#[test]
fn push_respawn_ignores_later_duplicate_spawn_id_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();
    let earlier = now + Duration::from_secs(10);
    let later = now + Duration::from_secs(60);

    let mut first = make_pending_respawn(earlier);
    first.spawn_id = 77;
    let mut duplicate = make_pending_respawn(later);
    duplicate.spawn_id = 77;

    map.push_respawn(first);
    map.push_respawn(duplicate);

    assert_eq!(map.respawn_queue_len(), 1);
    let ready = map.drain_ready_respawns(now + Duration::from_secs(11));
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].spawn_id, 77);
}
#[test]
fn pending_respawn_rebuild_preserves_zero_wander_distance_like_cpp() {
    let mut pending = make_pending_respawn(Instant::now());
    pending.respawn_delay_secs = 45;
    pending.selected_equipment_id = 6;
    pending.original_equipment_id = -1;
    pending.string_id = Some("respawn-string".to_string());

    let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

    assert_eq!(
        creature.creature.ai_ownership().wander_radius,
        0.0,
        "C++ respawn uses CreatureData::wander_distance; idle spawns must not regain an invented wander radius"
    );
    assert_eq!(
        creature.creature.ai_ownership().respawn_time_secs,
        45,
        "C++ Creature::LoadFromDB copies CreatureData::spawntimesecs into m_respawnDelay"
    );
    assert_eq!(
        creature.creature.equipment_id(),
        6,
        "C++ LoadEquipment mutates m_equipmentId to the selected equipment template"
    );
    assert_eq!(
        creature.creature.original_equipment_id(),
        -1,
        "C++ InitEntry keeps CreatureData::equipmentId in m_originalEquipmentId before random equipment selection mutates the selected id"
    );
    assert_eq!(
        creature.creature.lifecycle_metadata().string_id.as_deref(),
        Some("respawn-string"),
        "C++ respawn reloads CreatureData::StringId through Creature::LoadFromDB"
    );
    assert!(!creature.should_wander());
}
#[test]
fn respawn_ground_snap_uses_real_terrain_like_cpp() {
    // World (0,0) → raw tile (32,32). Ground at 77.0; spawn hovering above it.
    let dir = temp_dir_with_constant_tile(0, 32, 32, 77.0);
    let terrain = LiveTerrainHeights::new(&dir);

    let mut pending = make_pending_respawn(Instant::now());
    pending.home_pos.z = 80.0; // above ground; probe accepts the surface
    let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    assert!((creature.creature.unit().world().position().z - 80.0).abs() < 1e-3);

    snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

    // Grounded, non-hovering: snapped exactly onto the surface (+0 hover).
    assert!(
        (creature.creature.unit().world().position().z - 77.0).abs() < 1e-2,
        "respawn must sit on the .map ground like Creature::Respawn/UpdateAllowedPositionZ"
    );
    // C++ SetHomePosition takes the snapped Z too.
    assert!((creature.home_position().z - 77.0).abs() < 1e-2);

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn respawn_ground_snap_noop_without_terrain_tile_like_cpp() {
    // Empty maps dir → no tile → GetGridHeight invalid → Z untouched.
    let dir = temp_dir_with_constant_tile(0, 10, 10, 5.0); // tile for a different grid
    let terrain = LiveTerrainHeights::new(&dir);

    let mut pending = make_pending_respawn(Instant::now());
    pending.home_pos.z = 80.0;
    let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

    assert!(
        (creature.creature.unit().world().position().z - 80.0).abs() < 1e-3,
        "no terrain under the spawn → C++ leaves Z unchanged"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn pending_respawn_preserves_flags_extra_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 42);
    let mut creature = test_creature(guid);
    creature.creature.set_spawn_id(42);
    creature
        .creature
        .set_flags_extra_runtime_like_cpp(CreatureFlagsExtra::CIVILIAN.bits());
    let mut static_flags = [0; 8];
    static_flags[0] = wow_constants::creature::CreatureStaticFlags::NO_MELEE_FLEE.bits();
    creature
        .creature
        .set_static_flags_runtime_like_cpp(static_flags);
    creature
        .creature
        .set_ai_identity_names_runtime_like_cpp("SmartAI", "npc_respawn_identity");
    creature
        .creature
        .set_spawn_string_id_runtime_like_cpp(Some("respawn-string".to_string()));
    creature.creature.set_flight_movement_type_runtime_like_cpp(
        wow_constants::CreatureFlightMovementType::CanFly as u8,
    );
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::None as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(false);

    let mut pending = pending_respawn_from_world_creature_like_cpp(&creature, Instant::now(), 0);
    pending.create_data.hover_height = 1.5;
    pending.ground_movement_type = wow_constants::CreatureGroundMovementType::Hover as u8;
    pending.addon = Some(CreatureAddonLifecycleRecordLikeCpp {
        path_id: 88_001,
        visibility_distance_type: wow_entities::VisibilityDistanceTypeLikeCpp::Large,
        auras: vec![70_020],
        ..CreatureAddonLifecycleRecordLikeCpp::default()
    });
    assert_eq!(
        pending.spawn_id, 42,
        "creature respawn must preserve C++ RespawnInfo::spawnId from Creature::GetSpawnId"
    );
    assert_eq!(pending.flags_extra, CreatureFlagsExtra::CIVILIAN.bits());
    assert_eq!(pending.static_flags[0], static_flags[0]);
    assert_eq!(pending.ai_name, "SmartAI");
    assert_eq!(pending.script_name, "npc_respawn_identity");
    assert_eq!(pending.string_id.as_deref(), Some("respawn-string"));
    assert_eq!(
        pending.ground_movement_type,
        wow_constants::CreatureGroundMovementType::Hover as u8
    );
    assert!(!pending.swim_allowed);
    assert_eq!(
        pending.flight_movement_type,
        wow_constants::CreatureFlightMovementType::CanFly as u8
    );

    let respawned = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    assert_eq!(
        respawned.creature.spawn_id(),
        42,
        "C++ Creature::LoadFromDB restores m_spawnId before registering the respawned creature"
    );
    assert!(
        respawned.creature.is_civilian_like_cpp(),
        "map-owned respawn must keep C++ flags_extra gates"
    );
    assert_eq!(respawned.creature.lifecycle_metadata().ai_name, "SmartAI");
    assert_eq!(
        respawned.creature.lifecycle_metadata().script_name,
        "npc_respawn_identity"
    );
    assert_eq!(
        respawned.creature.lifecycle_metadata().string_id.as_deref(),
        Some("respawn-string")
    );
    assert!(respawned.creature.can_walk_like_cpp());
    assert!(!respawned.creature.can_enter_water_like_cpp());
    assert!(respawned.creature.can_fly_like_cpp());
    assert_eq!(
        respawned.position().z,
        1.5,
        "C++ Creature::Create adds GetHoverOffset() to Z when respawn reloads a hovering creature"
    );
    assert_eq!(respawned.creature.unit().data().hover_height, 1.5);
    assert_eq!(
        respawned.creature.waypoint_path_id_like_cpp(),
        88_001,
        "C++ respawn goes back through Creature::LoadFromDB and reapplies LoadCreaturesAddon PathId"
    );
    assert!(
        respawned
            .creature
            .unit()
            .unit_flags2_like_cpp()
            .contains(wow_constants::unit::UnitFlags2::LARGE_AOI),
        "C++ LoadCreaturesAddon reapplies addon visibility/AOI flags on respawn"
    );
    assert!(
        respawned
            .creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_020),
        "C++ LoadCreaturesAddon reapplies addon auras on respawn"
    );
}
#[test]
fn pending_respawn_keeps_guid_counter_queue_only_for_legacy_zero_spawn_id() {
    let first_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 43);
    let second_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 44);
    let first = test_creature(first_guid);
    let second = test_creature(second_guid);

    assert_eq!(first.creature.spawn_id(), 0);
    assert_eq!(second.creature.spawn_id(), 0);

    let first_pending = pending_respawn_from_world_creature_like_cpp(&first, Instant::now(), 0);
    let second_pending = pending_respawn_from_world_creature_like_cpp(&second, Instant::now(), 0);

    assert_eq!(first_pending.spawn_id, first_guid.low_value() as u64);
    assert_eq!(second_pending.spawn_id, second_guid.low_value() as u64);
    assert_ne!(first_pending.spawn_id, second_pending.spawn_id);
    assert!(!first_pending.persistent_spawn);
    let respawned = world_creature_from_pending_respawn_like_cpp(&first_pending, 0);
    assert_eq!(respawned.creature.spawn_id(), 0);
    assert!(
        !respawned
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp(),
        "generic queue-only respawns must remain fail-closed"
    );
}
