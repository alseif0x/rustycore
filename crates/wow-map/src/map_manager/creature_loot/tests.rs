//! Identity, lifecycle and partial-write contracts for the single loot motor.
use super::*;
use wow_constants::DeathState;

fn actor() -> WorldCreature {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 42, 91001);
    WorldCreature::new(
        guid,
        42,
        Position::xyz(1.0, 2.0, 3.0),
        100,
        20,
        2,
        3,
        20.0,
        1,
        14,
        0,
        0,
    )
}

fn pool(guid: ObjectGuid) -> CreatureLoot {
    CreatureLoot {
        loot_guid: guid,
        coins: 0,
        unlooted_count: 0,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: Vec::new(),
        looted_by_player: false,
    }
}

fn corpse(actor: &mut WorldCreature) {
    actor.creature.unit_mut().set_health(0);
    actor
        .creature
        .unit_mut()
        .set_death_state(DeathState::Corpse);
}

#[test]
fn installation_rejects_alive_lifetime_storage_and_generation_without_resurrection() {
    let mut actor = actor();
    let authority = actor.creature.loot_authority_like_cpp().clone();
    let lifetime = actor.creature.loot_lifecycle_revision_like_cpp();
    assert!(!actor.install_kill_loot(
        &authority,
        0,
        lifetime,
        Some(pool(actor.guid())),
        HashMap::new()
    ));
    corpse(&mut actor);
    assert!(!actor.install_kill_loot(&authority, 0, lifetime + 1, None, HashMap::new()));
    assert!(!actor.install_kill_loot(
        &OwnedLootAuthority::new(),
        0,
        lifetime,
        None,
        HashMap::new()
    ));
    assert!(!actor.install_kill_loot(&authority, 1, lifetime, None, HashMap::new()));
    assert_eq!(authority.generation_like_cpp(), 0);
    assert!(authority.is_retired_like_cpp());
    assert!(actor.install_kill_loot(
        &authority,
        0,
        lifetime,
        Some(pool(actor.guid())),
        HashMap::new()
    ));
    assert!(!actor.install_kill_loot(
        &authority,
        authority.generation_like_cpp(),
        lifetime,
        None,
        HashMap::new()
    ));
}

#[test]
fn retired_lifetime_aba_rejects_old_source_and_accepts_current_generation() {
    let mut actor = actor();
    corpse(&mut actor);
    let authority = actor.creature.loot_authority_like_cpp().clone();
    let lifetime = actor.creature.loot_lifecycle_revision_like_cpp();
    assert!(actor.install_kill_loot(
        &authority,
        0,
        lifetime,
        Some(pool(actor.guid())),
        HashMap::new()
    ));
    let generation = authority.generation_like_cpp();
    actor.creature.clear_loot_like_cpp();
    assert!(!actor.install_kill_loot(&authority, generation, lifetime, None, HashMap::new()));
    let current_lifetime = actor.creature.loot_lifecycle_revision_like_cpp();
    let retired_generation = authority.generation_like_cpp();
    assert!(actor.install_kill_loot(
        &authority,
        retired_generation,
        current_lifetime,
        Some(pool(actor.guid())),
        HashMap::new()
    ));
    assert!(authority.generation_like_cpp() > generation);
}

#[test]
fn installation_keeps_entity_health_and_runtime_identity() {
    let mut actor = actor();
    corpse(&mut actor);
    actor.advance_runtime_clock_like_cpp(7000);
    let entity = &actor.creature as *const _;
    let health_revision = actor.creature.unit().health_state_revision_like_cpp();
    let authority = actor.creature.loot_authority_like_cpp().clone();
    let lifetime = actor.creature.loot_lifecycle_revision_like_cpp();
    assert!(actor.install_kill_loot(
        &authority,
        0,
        lifetime,
        Some(pool(actor.guid())),
        HashMap::new()
    ));
    assert_eq!(&actor.creature as *const _, entity);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 7000);
    assert_eq!(
        actor.creature.unit().health_state_revision_like_cpp(),
        health_revision
    );
    assert!(
        actor
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&authority)
    );
}

#[test]
fn stale_release_preserves_normal_force_but_not_corpse_mutation() {
    let mut actor = actor();
    corpse(&mut actor);
    let authority = actor.creature.loot_authority_like_cpp().clone();
    authority.replace_like_cpp(Some(pool(actor.guid())), HashMap::new());
    let observation = authority
        .fully_looted_unviewed_lifecycle_observation_like_cpp()
        .unwrap();
    actor.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
    let forced = actor.force_loot_flags();
    assert!(
        actor
            .release_looted_corpse(
                &authority,
                observation.object_generation + 1,
                observation.lifecycle_revision,
                false,
                1.0,
                CreatureLootReleasePhase::Normal
            )
            .is_none()
    );
    assert!(actor.has_lootable_dynamic_flag_like_cpp());
    assert_eq!(actor.creature.unit().values_update(), forced);
}

#[test]
fn detached_release_rejects_late_viewer_before_any_flag_write() {
    let mut actor = actor();
    corpse(&mut actor);
    let authority = actor.creature.loot_authority_like_cpp().clone();
    authority.replace_like_cpp(Some(pool(actor.guid())), HashMap::new());
    let observation = authority
        .fully_looted_unviewed_lifecycle_observation_like_cpp()
        .unwrap();
    actor.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
    let before = actor.creature.unit().values_update();
    let player = ObjectGuid::create_player(1, 91);
    let open = authority.add_viewer_like_cpp(player).unwrap();
    assert!(
        actor
            .release_looted_corpse(
                &authority,
                observation.object_generation,
                observation.lifecycle_revision,
                false,
                1.0,
                CreatureLootReleasePhase::Detached
            )
            .is_none()
    );
    assert_eq!(actor.creature.unit().values_update(), before);
    assert!(actor.has_lootable_dynamic_flag_like_cpp());
    authority
        .close_viewer_if_generation_like_cpp(open.generation, player)
        .unwrap();
    assert!(
        actor
            .release_looted_corpse(
                &authority,
                observation.object_generation,
                observation.lifecycle_revision,
                false,
                1.0,
                CreatureLootReleasePhase::Detached
            )
            .is_some()
    );
    assert!(!actor.has_lootable_dynamic_flag_like_cpp());
}

#[test]
fn normal_release_keeps_other_viewers_and_expired_corpse_partial_writes() {
    let mut actor = actor();
    corpse(&mut actor);
    let authority = actor.creature.loot_authority_like_cpp().clone();
    authority.replace_like_cpp(Some(pool(actor.guid())), HashMap::new());
    let observation = authority
        .fully_looted_unviewed_lifecycle_observation_like_cpp()
        .unwrap();
    let player = ObjectGuid::create_player(1, 92);
    authority.add_viewer_like_cpp(player).unwrap();
    actor.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
    let corpse_time = actor.creature.corpse_remove_time();
    let elapsed = actor.runtime_elapsed_ms_like_cpp();
    let (marked, _) = actor
        .release_looted_corpse(
            &authority,
            observation.object_generation,
            observation.lifecycle_revision,
            false,
            1.0,
            CreatureLootReleasePhase::Normal,
        )
        .unwrap()
        .into_parts();
    assert_eq!(marked, None);
    assert_eq!(actor.creature.corpse_remove_time(), corpse_time);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), elapsed);
    assert!(!actor.has_lootable_dynamic_flag_like_cpp());
    assert!(
        authority
            .fully_looted_unviewed_lifecycle_observation_like_cpp()
            .is_none()
    );
}

#[test]
fn foreign_authority_cannot_release_and_legacy_snapshot_shares_original_storage() {
    let mut actor = actor();
    corpse(&mut actor);
    let authority = actor.creature.loot_authority_like_cpp().clone();
    authority.replace_like_cpp(Some(pool(actor.guid())), HashMap::new());
    let observation = authority
        .fully_looted_unviewed_lifecycle_observation_like_cpp()
        .unwrap();
    actor.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
    assert!(
        actor
            .release_looted_corpse(
                &OwnedLootAuthority::new(),
                observation.object_generation,
                observation.lifecycle_revision,
                false,
                1.0,
                CreatureLootReleasePhase::Normal
            )
            .is_none()
    );
    assert!(actor.has_lootable_dynamic_flag_like_cpp());
    let (_, snapshot) = actor
        .release_legacy_looted_corpse(
            &authority,
            observation.object_generation,
            observation.lifecycle_revision,
            false,
            1.0,
            CreatureLootReleasePhase::Normal,
        )
        .unwrap();
    assert!(
        snapshot
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&authority)
    );
    assert!(
        !snapshot
            .unit()
            .world()
            .object()
            .has_dynamic_flag(wow_constants::UnitDynFlags::Lootable as u32)
    );
}
