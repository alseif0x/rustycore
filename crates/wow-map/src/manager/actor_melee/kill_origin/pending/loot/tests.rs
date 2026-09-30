//! Same real producer/token/Actor installer, never a second TARGET reservation.
use super::*;
use crate::manager::MapObjectUpdateSelectionLikeCpp;
use crate::manager::actor_melee::kill_origin::{MeleeKillCapture, MeleeKillOccurrence};
use crate::manager::actor_tick_access::fixtures::*;
mod fixtures;
use fixtures::*;

fn operation(
    manager: &mut MapManager,
    root: ObjectGuid,
) -> (MapObjectTickContinuation, PreparedMeleeLoot) {
    let (tick, token) = start(
        manager,
        200,
        MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
    );
    let pending = manager
        .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
        .ok()
        .unwrap()
        .into_pending();
    let prepared = manager
        .prepare_next_melee_kill(&tick, pending)
        .ok()
        .unwrap();
    let operation = manager.prepare_melee_loot(&tick, prepared).ok().unwrap();
    (tick, operation)
}
fn pool(owner: ObjectGuid) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::create_world_object(HighGuid::LootObject, 0, 1, 1, 0, 0, 1),
        coins: 7,
        unlooted_count: 0,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![owner],
        items: Vec::new(),
        looted_by_player: false,
    }
}

#[test]
fn install_uses_the_same_target_and_preserves_the_original_root_slot_and_buffers() {
    let (mut manager, root, victim) = setup(4500, 5, false, 10);
    let (tick, mut op) = operation(&mut manager, root);
    let identity = (
        op.pending.token.key(),
        op.pending.token.incarnation(),
        op.pending.token.effective_diff_ms(),
    );
    let buffers = (
        op.pending.outcome.events.as_ptr(),
        op.pending.outcome.syncs.as_ptr(),
        op.pending.batch.occurrences.as_ptr(),
    );
    let authority = op.authority().clone();
    manager
        .install_melee_loot(&tick, &mut op, Some(pool(root)), HashMap::new())
        .unwrap();
    assert_eq!(op.target_guid(), victim);
    assert_eq!(
        (
            op.pending.token.key(),
            op.pending.token.incarnation(),
            op.pending.token.effective_diff_ms()
        ),
        identity
    );
    assert_eq!(
        (
            op.pending.outcome.events.as_ptr(),
            op.pending.outcome.syncs.as_ptr(),
            op.pending.batch.occurrences.as_ptr()
        ),
        buffers
    );
    assert!(
        op.pending
            .token
            .actor_operation
            .as_ref()
            .unwrap()
            .matches_token(&op.pending.token)
    );
    assert_eq!(
        op.pending.token.actor_operation.as_ref().unwrap().guid,
        root
    );
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(root)
            .unwrap()
            .loot
            .coins,
        7
    );
    assert_eq!(health(&manager, victim), 0);
    assert_eq!(op.into_pending().cursor(), 0);
}

#[test]
fn each_counter_allocation_revalidates_target_revision_before_consuming_the_counter() {
    let (mut manager, root, victim) = setup(4510, 5, false, 10);
    let (tick, mut op) = operation(&mut manager, root);
    let before = manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .get_max_low_guid_like_cpp(HighGuid::LootObject)
        .unwrap();
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(victim)
        .unwrap()
        .unit_mut()
        .set_max_health(101);
    assert!(matches!(manager.next_melee_loot_counter(&tick, &mut op),
        Err(MeleeLootError::Phase(MeleeKillPhaseError::HealthRevisionConflict { guid, .. })) if guid == victim));
    assert_eq!(
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .get_max_low_guid_like_cpp(HighGuid::LootObject)
            .unwrap(),
        before
    );
    assert_eq!(
        op.pending.token.actor_operation.as_ref().unwrap().guid,
        root
    );
    assert_eq!(op.into_pending().cursor(), 0);
}

#[test]
fn install_failure_keeps_previously_consumed_counter_and_original_partial_outcome() {
    let (mut manager, root, victim) = setup(4520, 5, false, 10);
    let (tick, mut op) = operation(&mut manager, root);
    let before = manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .get_max_low_guid_like_cpp(HighGuid::LootObject)
        .unwrap();
    assert_eq!(
        manager.next_melee_loot_counter(&tick, &mut op).unwrap().0,
        MapKey::new(1, 0)
    );
    op.authority().retire_like_cpp();
    assert!(
        matches!(manager.install_melee_loot(&tick, &mut op, Some(pool(root)), HashMap::new()),
        Err(MeleeLootError::Phase(MeleeKillPhaseError::GenerationConflict { guid, .. })) if guid == victim)
    );
    assert_eq!(
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .get_max_low_guid_like_cpp(HighGuid::LootObject)
            .unwrap(),
        before + 1
    );
    assert_eq!(op.pending.outcome.canonical_creature_hits, 1);
    assert_eq!(op.pending.outcome.events.len(), 2);
    assert_eq!(op.pending.outcome.syncs.len(), 1);
    assert!(op.authority().snapshot_for_player_like_cpp(root).is_none());
    assert_eq!(
        op.pending.token.actor_operation.as_ref().unwrap().guid,
        root
    );
    assert_eq!(op.into_pending().cursor(), 0);
}

#[test]
fn foreign_pending_root_slot_is_never_cleared_replaced_or_retried() {
    let (mut manager, root, victim) = setup(4530, 5, false, 10);
    let (tick, mut op) = operation(&mut manager, root);
    manager
        .complete_actor_operation(
            &tick,
            &mut op.pending.token,
            root,
            &op.pending.batch.root_witness,
        )
        .unwrap();
    let foreign = manager
        .begin_actor_operation(&tick, &mut op.pending.token, victim, None)
        .unwrap();
    assert!(matches!(manager.next_melee_loot_counter(&tick, &mut op),
        Err(MeleeLootError::Phase(MeleeKillPhaseError::RootStale(
            ActorTickAccessError::OperationMismatch { guid }))) if guid == root));
    assert_eq!(
        op.pending.token.actor_operation.as_ref().unwrap().guid,
        victim
    );
    assert!(
        op.pending
            .token
            .actor_operation
            .as_ref()
            .unwrap()
            .witness
            .same_actor(&foreign)
    );
    assert_eq!(op.pending.outcome.canonical_creature_hits, 1);
    assert_eq!(op.into_pending().cursor(), 0);
}

#[test]
fn dead_root_can_allocate_and_install_without_a_new_alive_gate() {
    let (mut manager, root, _) = setup(4540, 75, false, 100);
    arm(&mut manager, root, root, 100);
    let (tick, mut op) = operation(&mut manager, root);
    assert_eq!(health(&manager, root), 0);
    assert!(manager.next_melee_loot_counter(&tick, &mut op).is_ok());
    manager
        .install_melee_loot(&tick, &mut op, Some(pool(root)), HashMap::new())
        .unwrap();
    assert_eq!(
        op.pending.token.actor_operation.as_ref().unwrap().guid,
        root
    );
    assert_eq!(op.into_pending().cursor(), 0);
}

#[test]
fn late_damage_target_can_allocate_and_install_without_growing_nearby_selection() {
    let (mut manager, root) = manager_with_actor(4550);
    let (tick, mut token) = start(
        &mut manager,
        200,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    assert_eq!(
        manager.selected_actor_guids(&tick, &mut token).unwrap(),
        vec![root]
    );
    let victim = target(&mut manager, 4551, 5, false);
    arm(&mut manager, root, victim, 10);
    let pending = manager
        .apply_selected_creature_melee_with_kills(&tick, token, root, &Catalogs::default())
        .ok()
        .unwrap()
        .into_pending();
    let prepared = manager
        .prepare_next_melee_kill(&tick, pending)
        .ok()
        .unwrap();
    let mut op = manager.prepare_melee_loot(&tick, prepared).ok().unwrap();
    assert!(manager.next_melee_loot_counter(&tick, &mut op).is_ok());
    manager
        .install_melee_loot(&tick, &mut op, Some(pool(root)), HashMap::new())
        .unwrap();
    assert_eq!(
        manager
            .selected_actor_guids(&tick, &mut op.pending.token)
            .unwrap(),
        vec![root]
    );
    assert_eq!(op.target_guid(), victim);
    assert_eq!(op.into_pending().cursor(), 0);
}
