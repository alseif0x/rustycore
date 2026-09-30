//! Snapshot-clone resets and live-actor movement by value.

use super::fixtures::test_creature;
use crate::map_manager::RuntimeMovementGeneratorType;
use rand::RngCore;
use wow_core::{ObjectGuid, Position, guid::HighGuid};

#[test]
fn world_creature_snapshot_clone_resets_selector_and_keeps_runtime_state() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_021);
    let target = ObjectGuid::create_player(1, 7021);
    let mut actor = test_creature(guid);
    actor
        .begin_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
        .expect("launch point spline before taking a snapshot");
    actor.creature.unit_mut().subsystems_mut().motion.move_charge(42);
    actor.enter_combat(target);
    actor.tick_runtime_motion_master_like_cpp(50);
    actor.seed_runtime_rng_like_cpp(0xC10E_7021);
    for _ in 0..5 {
        actor.runtime.runtime_rng_like_cpp.next_u64();
    }
    actor.runtime.runtime_elapsed_ms_like_cpp = 5_000;
    actor.runtime.creature_spell_due_at_ms_like_cpp[0] = Some(5_500);
    actor.runtime.creature_spell_schedule_initialized_like_cpp = true;
    actor.runtime.creature_spell_engagement_epoch_like_cpp = 9;
    actor.runtime.pending_assistance_like_cpp.push((target, vec![target], 5_250));
    actor.runtime.active_random_path_poly_refs = vec![101, 102];
    actor.runtime.active_chase_path_poly_refs = vec![201, 202];
    actor.runtime.runtime_rng_authority_complete_like_cpp = false;
    actor.runtime.respawn_spell_hit_aura_source_authority_like_cpp = true;
    actor.runtime.respawn_spell_cast_log_aura_source_authority_like_cpp = true;

    assert_eq!(actor.runtime_motion_master_current_kind_like_cpp(), Some(RuntimeMovementGeneratorType::Point));
    assert_eq!(actor.runtime.runtime_chase_target, Some(target));
    assert!(actor.runtime.runtime_represented_active.is_some());
    let mut snapshot = actor.clone();

    // The cloned Creature still attacks its target, so the rebuilt selector
    // starts with chase rather than copying the live high-priority point stack.
    assert_eq!(snapshot.runtime_motion_master_current_kind_like_cpp(), Some(RuntimeMovementGeneratorType::Chase));
    assert_eq!(actor.runtime_motion_master_current_kind_like_cpp(), Some(RuntimeMovementGeneratorType::Point));
    assert!(snapshot.runtime.runtime_chase_target.is_none());
    assert!(snapshot.runtime.runtime_represented_active.is_none());
    assert!(snapshot.active_move_spline_like_cpp().is_some());
    assert_eq!(snapshot.runtime.active_random_path_poly_refs, vec![101, 102]);
    assert_eq!(snapshot.runtime.active_chase_path_poly_refs, vec![201, 202]);
    assert_eq!(snapshot.runtime.pending_assistance_like_cpp, vec![(target, vec![target], 5_250)]);
    assert_eq!(snapshot.runtime.runtime_elapsed_ms_like_cpp, 5_000);
    assert_eq!(snapshot.runtime.creature_spell_due_at_ms_like_cpp[0], Some(5_500));
    assert!(snapshot.runtime.creature_spell_schedule_initialized_like_cpp);
    assert_eq!(snapshot.runtime.creature_spell_engagement_epoch_like_cpp, 9);
    assert_eq!(snapshot.runtime_motion_master_ticks_like_cpp(), actor.runtime_motion_master_ticks_like_cpp());
    assert!(!snapshot.runtime_rng_authority_complete_like_cpp());
    assert!(snapshot.runtime.respawn_spell_hit_aura_source_authority_like_cpp);
    assert!(snapshot.runtime.respawn_spell_cast_log_aura_source_authority_like_cpp);
    for _ in 0..8 {
        assert_eq!(snapshot.runtime.runtime_rng_like_cpp.next_u64(), actor.runtime.runtime_rng_like_cpp.next_u64());
    }
}

#[test]
fn world_creature_owned_move_retains_selector_deadlines_and_rng_stream() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_022);
    let target = ObjectGuid::create_player(1, 7022);
    let mut actor = test_creature(guid);
    actor.begin_distract_movement_like_cpp(8_000, 1.25).expect("start finite distract before moving the actor");
    actor.enter_combat(target);
    actor.tick_runtime_motion_master_like_cpp(25);
    actor.seed_runtime_rng_like_cpp(0xA10E_7022);
    for _ in 0..5 {
        actor.runtime.runtime_rng_like_cpp.next_u64();
    }
    let mut expected_rng = actor.runtime.runtime_rng_like_cpp.clone();
    let represented_active = actor.runtime.runtime_represented_active;
    let ticks = actor.runtime_motion_master_ticks_like_cpp();
    actor.runtime.runtime_elapsed_ms_like_cpp = 7_000;
    actor.runtime.creature_spell_due_at_ms_like_cpp[1] = Some(7_600);
    actor.runtime.creature_spell_engagement_epoch_like_cpp = 11;
    actor.runtime.respawn_spell_hit_aura_source_authority_like_cpp = true;
    assert!(represented_active.is_some());
    assert_eq!(actor.runtime_motion_master_current_kind_like_cpp(), Some(RuntimeMovementGeneratorType::Distract));

    let mut ownership_slot = Some(actor);
    let mut moved = ownership_slot.take().expect("move the live actor once");
    assert!(ownership_slot.is_none());
    assert_eq!(moved.runtime_motion_master_current_kind_like_cpp(), Some(RuntimeMovementGeneratorType::Distract));
    assert_eq!(moved.runtime.runtime_chase_target, Some(target));
    assert_eq!(moved.runtime.runtime_represented_active, represented_active);
    assert_eq!(moved.runtime_motion_master_ticks_like_cpp(), ticks);
    assert_eq!(moved.runtime.runtime_elapsed_ms_like_cpp, 7_000);
    assert_eq!(moved.runtime.creature_spell_due_at_ms_like_cpp[1], Some(7_600));
    assert_eq!(moved.runtime.creature_spell_engagement_epoch_like_cpp, 11);
    assert!(moved.runtime.respawn_spell_hit_aura_source_authority_like_cpp);
    for _ in 0..8 {
        assert_eq!(moved.runtime.runtime_rng_like_cpp.next_u64(), expected_rng.next_u64());
    }
}
