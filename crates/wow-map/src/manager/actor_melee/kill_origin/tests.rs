//! Original engine transitions with real map-owned Actors and admitted tokens.
use super::*;
use crate::manager::actor_tick_access::fixtures::*;
use crate::manager::{MapManager, MapObjectUpdateSelectionLikeCpp};
use crate::map::{CreatureMeleeCatalogsLikeCpp, CreatureMeleeReadiness, creature_melee_readiness};

mod failures;
mod fixtures;
mod ordering;
use fixtures::*;

#[test]
fn nonlethal_zero_not_ready_and_rejected_swings_leave_the_slot_empty() {
    for scenario in 0..4 {
        let (mut manager, root, victim) = setup(
            2400 + scenario * 10,
            75,
            false,
            if scenario == 1 { 0 } else { 10 },
        );
        if scenario == 2 {
            let actor = manager
                .find_map_mut(1, 0)
                .unwrap()
                .map_mut()
                .creature_actor_mut(root)
                .unwrap();
            actor.creature.ai_ownership_mut().swing_timer_ms = 1;
        }
        if scenario == 3 {
            manager
                .find_map_mut(1, 0)
                .unwrap()
                .map_mut()
                .get_typed_creature_mut(victim)
                .unwrap()
                .unit_mut()
                .world_mut()
                .relocate(wow_core::Position::xyz(1000.0, 20.0, 30.0));
        }
        let (tick, token) = start(
            &mut manager,
            200,
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
        );
        let execution = manager
            .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
            .ok()
            .unwrap();
        assert!(!execution.batch().is_reserved());
        assert!(execution.batch().occurrences().is_empty());
        let (_, token, _) = execution.into_parts();
        assert!(token.actor_operation.is_none());
    }
}

#[test]
fn primary_kill_freezes_the_same_actor_source_and_original_token() {
    let (mut manager, root, victim) = setup(2450, 5, false, 10);
    let map = manager.find_map(1, 0).unwrap().map();
    let target_address = map.creature_actor(victim).unwrap() as *const _ as usize;
    let witness = map.creature_actor_witness(victim).unwrap();
    let authority = map
        .creature_actor(victim)
        .unwrap()
        .creature
        .loot_authority_like_cpp()
        .clone();
    let (tick, token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let identity = (token.key(), token.incarnation(), token.effective_diff_ms());
    let execution = manager
        .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
        .ok()
        .unwrap();
    assert_eq!(execution.outcome().canonical_creature_hits, 1);
    assert_eq!(execution.outcome().syncs.len(), 1);
    assert_eq!(execution.outcome().events.len(), 2);
    assert_eq!(execution.batch().root_guid(), root);
    assert!(execution.batch().is_reserved());
    assert_eq!(execution.batch().occurrences().len(), 1);
    let occurrence = &execution.batch().occurrences()[0];
    assert_eq!(occurrence.phase(), MeleeKillPhase::Primary);
    let capture = captured(occurrence);
    assert_eq!(capture.target_guid(), victim);
    assert!(capture.target_witness.same_actor(&witness));
    assert!(capture.authority().shares_storage_like_cpp(&authority));
    let map = manager.find_map(1, 0).unwrap().map();
    let actor = map.creature_actor(victim).unwrap();
    assert_eq!(actor as *const _ as usize, target_address);
    assert!(!capture.source().is_alive());
    assert_eq!(capture.source().entry(), actor.entry());
    assert_eq!(capture.source().tappers(), actor.creature.tap_list());
    assert_eq!(
        capture.source().loot_lifecycle_revision(),
        actor.creature.loot_lifecycle_revision_like_cpp()
    );
    assert_eq!(
        capture.object_generation(),
        actor
            .creature
            .loot_authority_like_cpp()
            .generation_like_cpp()
    );
    assert_eq!(
        capture.health_revision(),
        actor.creature.unit().health_state_revision_like_cpp()
    );
    let (_, mut token, batch) = execution.into_parts();
    assert_eq!(
        (token.key(), token.incarnation(), token.effective_diff_ms()),
        identity
    );
    assert!(
        token
            .actor_operation
            .as_ref()
            .unwrap()
            .matches_token(&token)
    );
    assert_eq!(token.actor_operation.as_ref().unwrap().guid, root);
    let root_witness = batch.root_witness.clone();
    drop(batch);
    assert!(token.actor_operation.is_some());
    // Explicit disposal accounting in the fixture; no production settlement API.
    manager
        .complete_actor_operation(&tick, &mut token, root, &root_witness)
        .unwrap();
    assert!(token.actor_operation.is_none());
}

#[test]
fn late_damage_target_is_captured_without_expanding_nearby_update_selection() {
    let (mut manager, root) = manager_with_actor(2470);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    let before = manager.selected_actor_guids(&tick, &mut token).unwrap();
    assert_eq!(before, vec![root]);
    let victim = target(&mut manager, 2471, 5, false);
    arm(&mut manager, root, victim, 10);
    let execution = manager
        .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
        .ok()
        .unwrap();
    assert_eq!(
        captured(&execution.batch().occurrences()[0]).target_guid(),
        victim
    );
    assert_eq!(health(&manager, victim), 0);
    let (_, mut token, _) = execution.into_parts();
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        before
    );
}

#[test]
fn record_kill_reports_no_actor_without_losing_damage_events_or_sync() {
    let (mut manager, root, victim) = setup(2480, 5, true, 10);
    let (tick, token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let execution = manager
        .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
        .ok()
        .unwrap();
    assert_eq!(health(&manager, victim), 0);
    assert_eq!(execution.outcome().canonical_creature_hits, 1);
    assert_eq!(execution.outcome().syncs.len(), 1);
    assert_eq!(execution.outcome().events.len(), 2);
    assert!(execution.batch().is_reserved());
    assert!(matches!(
        execution.batch().occurrences()[0].capture(),
        MeleeKillCapture::Unavailable(MeleeKillCaptureError::NoActor)
    ));
}

#[test]
fn root_self_death_reserves_without_a_late_alive_gate_and_still_rearms() {
    let (mut manager, root, _) = setup(2490, 75, false, 100);
    arm(&mut manager, root, root, 100);
    let (tick, token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let execution = manager
        .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
        .ok()
        .unwrap();
    assert_eq!(execution.outcome().canonical_creature_hits, 1);
    assert!(execution.batch().is_reserved());
    assert_eq!(
        captured(&execution.batch().occurrences()[0]).target_guid(),
        root
    );
    let actor = manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(root)
        .unwrap();
    assert_eq!(actor.creature.unit().data().health, 0);
    assert_eq!(
        actor.creature.unit().death_state(),
        wow_constants::DeathState::JustDied
    );
    assert_eq!(
        actor.creature.ai_ownership().swing_timer_ms,
        actor.create_data.base_attack_time as u64
    );
}

#[test]
fn original_canonical_entry_and_capture_entry_keep_the_same_engine_effects_and_queries() {
    let (mut old, old_root, old_victim) = setup(2500, 5, false, 10);
    let (mut new, new_root, new_victim) = setup(2500, 5, false, 10);
    let (old_tick, mut old_token) = start(
        &mut old,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let (new_tick, new_token) = start(
        &mut new,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let old_catalogs = Catalogs::default();
    let new_catalogs = Catalogs::default();
    let original = old
        .apply_selected_creature_melee(&old_tick, &mut old_token, old_root, &old_catalogs)
        .unwrap();
    let execution = new
        .apply_selected_creature_melee_with_kills(&new_tick, new_token, new_root, &new_catalogs)
        .ok()
        .unwrap();
    assert_eq!(
        original.canonical_creature_hits,
        execution.outcome().canonical_creature_hits
    );
    assert_eq!(original.events.len(), execution.outcome().events.len());
    assert_eq!(
        original.syncs[0].state.victim_health_after,
        execution.outcome().syncs[0].state.victim_health_after
    );
    assert_eq!(health(&old, old_victim), health(&new, new_victim));
    assert_eq!(*old_catalogs.calls.borrow(), *new_catalogs.calls.borrow());
    assert!(old_token.actor_operation.is_none());
    assert!(execution.batch().is_reserved());
}
