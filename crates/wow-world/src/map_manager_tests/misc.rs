//! Misc scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn create_data_from_canonical_keeps_base_mana_distinct_from_non_mana_power_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 9003);
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9003);
    creature.set_power_type(PowerType::Focus);
    creature.unit_mut().set_create_mana_like_cpp(600);
    creature.unit_mut().set_max_power(PowerType::Focus, 100);
    creature.unit_mut().set_power(PowerType::Focus, 25);

    let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);

    assert_eq!(create_data.display_power, PowerType::Focus as u8);
    assert_eq!(create_data.base_mana, 600);
    assert_eq!(create_data.max_power[0], 100);
    assert_eq!(create_data.power[0], 25);
}
#[test]
fn equal_rng_bounds_still_consume_shared_runtime_draws_like_cpp() {
    let seed = 0xE011_A1_u64;
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70008);
    let mut creature = test_creature(guid);
    creature.seed_runtime_rng_like_cpp(seed);
    creature.creature.ai_ownership_mut().min_damage = 7;
    creature.creature.ai_ownership_mut().max_damage = 7;
    let mut expected_rng = StdRng::seed_from_u64(seed);

    assert_eq!(
        creature.random_creature_spell_delay_like_cpp(5_000, 5_000),
        Some(5_000)
    );
    let _ = expected_rng.next_u32();
    assert_eq!(creature.roll_damage(), Some(7));
    let _ = expected_rng.next_u32();
    assert_eq!(
        creature.random_creature_spell_hit_roll_like_cpp(),
        Some(expected_rng.gen_range(0..=9_999))
    );
}
#[test]
fn test_player_enter_leave() {
    let mut grid = Grid::new(0, 0);
    let player = ObjectGuid::create_player(1, 1);

    grid.player_enter(player);
    assert!(grid.player_guids.contains(&player));

    grid.player_leave(player);
    assert!(!grid.player_guids.contains(&player));
}
/// `drain_ready_respawns` returns only entries whose `respawn_at <= now`.
#[test]
fn drain_returns_only_ready_entries_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();
    let past = now - Duration::from_secs(5);
    let future = now + Duration::from_secs(60);

    map.push_respawn(make_pending_respawn(past));
    map.push_respawn(make_pending_respawn(future));

    let ready = map.drain_ready_respawns(now);
    assert_eq!(ready.len(), 1);
    assert_eq!(map.respawn_queue_len(), 1);
}
/// Entries that are not yet ready remain in the queue after drain.
#[test]
fn future_entries_remain_after_drain_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let future = Instant::now() + Duration::from_secs(60);

    map.push_respawn(make_pending_respawn(future));

    let ready = map.drain_ready_respawns(Instant::now());
    assert_eq!(ready.len(), 0);
    assert_eq!(map.respawn_queue_len(), 1);
}
/// Ready entries are returned in insertion order.
#[test]
fn drain_preserves_insertion_order_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let t0 = Instant::now() - Duration::from_secs(10);
    let t1 = Instant::now() - Duration::from_secs(5);
    let t2 = Instant::now() - Duration::from_secs(1);

    // Insert in REVERSE temporal order (t2, t1, t0) — all in the past, all ready.
    // drain must return them in INSERTION order, not sorted by respawn_at, mirroring
    // the original Vec partition in run_creatures_tick (session.rs:20189-20201).
    map.push_respawn(make_pending_respawn(t2));
    map.push_respawn(make_pending_respawn(t1));
    map.push_respawn(make_pending_respawn(t0));

    let now = Instant::now();
    let ready = map.drain_ready_respawns(now);

    assert_eq!(ready.len(), 3);
    // Insertion order (t2, t1, t0), distinct from temporal order (t0, t1, t2).
    assert_eq!(ready[0].respawn_at, t2);
    assert_eq!(ready[1].respawn_at, t1);
    assert_eq!(ready[2].respawn_at, t0);
}
