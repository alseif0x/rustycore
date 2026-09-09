//! Spell scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn spell_effect_summon_object_wild_creates_spell_go_without_owner_like_cpp() {
    let mut map = test_map();
    let caster = test_player_for_viewpoint(4822501);
    let caster_guid = caster.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(caster).unwrap())
        .unwrap();
    let template = summon_gameobject_template_like_cpp(4822502, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);
    let position = Position::new(14.0, 15.0, 16.0, 1.5);

    let outcome = map.spell_effect_summon_object_wild_like_cpp(
        caster_guid,
        4822510,
        template,
        position,
        23_456,
    );

    assert_eq!(
        outcome.status,
        SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
    );
    assert_eq!(outcome.low_guid, Some(1));
    assert_eq!(outcome.respawn_time_secs, Some(23));
    assert!(!outcome.phase_inherit_represented);
    assert!(outcome.execute_log_represented);
    assert!(!outcome.owner_linked);
    assert!(!outcome.flagdrop_type);
    assert!(!outcome.flagdrop_player_branch_reached);
    assert!(!outcome.flagdrop_battleground_update_represented);
    assert!(outcome.linked_trap_guid.is_none());
    assert!(!outcome.linked_trap_side_effect_represented);
    assert!(outcome.add_to_map.as_ref().is_some_and(|add| add.inserted));

    let guid = outcome.guid.unwrap();
    let gameobject = map
        .map_object_record(guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(gameobject.spell_id(), 4822510);
    assert_eq!(gameobject.respawn_time(), 23);
    assert_eq!(gameobject.world().position(), position);
    assert!(gameobject.world().object().is_in_world());
    assert!(!gameobject.spawned_by_default());
    let caster = map
        .map_object_record(caster_guid)
        .and_then(MapObjectRecord::player)
        .unwrap();
    assert!(
        caster
            .unit()
            .subsystems()
            .control
            .owned_gameobjects
            .is_empty()
    );
}
#[test]
fn spell_effect_summon_object_wild_flagdrop_records_unrepresented_bg_branch_like_cpp() {
    let mut map = test_map();
    let caster = test_player_for_viewpoint(4822601);
    let caster_guid = caster.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(caster).unwrap())
        .unwrap();
    let template = summon_gameobject_template_like_cpp(4822602, GAMEOBJECT_TYPE_FLAGDROP);

    let outcome = map.spell_effect_summon_object_wild_like_cpp(
        caster_guid,
        4822610,
        template,
        Position::xyz(1.0, 2.0, 3.0),
        0,
    );

    assert_eq!(
        outcome.status,
        SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
    );
    assert!(outcome.flagdrop_type);
    assert!(outcome.flagdrop_player_branch_reached);
    assert!(!outcome.flagdrop_battleground_update_represented);
    assert_eq!(outcome.respawn_time_secs, Some(0));
    let gameobject = map
        .map_object_record(outcome.guid.unwrap())
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(gameobject.data().type_id, GAMEOBJECT_TYPE_FLAGDROP as i8);
    assert_eq!(gameobject.spell_id(), 4822610);
    assert_eq!(gameobject.respawn_time(), 0);
}
#[test]
fn spell_effect_summon_object_wild_missing_caster_does_not_consume_guid_like_cpp() {
    let mut map = test_map();
    let template = summon_gameobject_template_like_cpp(4822702, GAMEOBJECT_TYPE_GENERIC_LIKE_CPP);

    let outcome = map.spell_effect_summon_object_wild_like_cpp(
        ObjectGuid::create_player(1, 4822701),
        4822710,
        template,
        Position::xyz(1.0, 2.0, 3.0),
        -1,
    );

    assert_eq!(
        outcome.status,
        SpellEffectSummonObjectWildStatusLikeCpp::MissingCaster
    );
    assert!(outcome.guid.is_none());
    assert!(outcome.add_to_map.is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
}
#[test]
fn far_spell_callbacks_drain_fifo_record_execution_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    map.add_far_spell_callback_like_cpp(RepresentedFarSpellCallbackLikeCpp {
        id: 10,
        action: RepresentedFarSpellCallbackActionLikeCpp::RecordExecution,
    });
    map.add_far_spell_callback_like_cpp(RepresentedFarSpellCallbackLikeCpp {
        id: 20,
        action: RepresentedFarSpellCallbackActionLikeCpp::RecordExecution,
    });

    let summary = map.drain_far_spell_callbacks_like_cpp();

    assert_eq!(summary.queued_before, 2);
    assert_eq!(summary.processed, 2);
    assert_eq!(summary.record_only, 2);
    assert_eq!(summary.queued_after, 0);
    assert_eq!(map.far_spell_callbacks_count_like_cpp(), 0);
    assert_eq!(
        map.represented_far_spell_callback_execution_log_like_cpp(),
        &[10, 20]
    );
}
#[test]
fn far_spell_callback_queue_object_remove_missing_records_stale_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let missing_guid = guid(HighGuid::DynamicObject, 487_001);
    map.add_far_spell_callback_like_cpp(RepresentedFarSpellCallbackLikeCpp {
        id: 1,
        action: RepresentedFarSpellCallbackActionLikeCpp::QueueObjectRemove { guid: missing_guid },
    });

    let summary = map.drain_far_spell_callbacks_like_cpp();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.remove_queue_attempted, 1);
    assert_eq!(summary.remove_queued, 0);
    assert_eq!(summary.remove_missing_or_stale, 1);
    assert_eq!(summary.remove_duplicates, 0);
    assert_eq!(summary.queued_after, 0);
}
#[test]
fn farsight_dynamic_object_create_missing_caster_does_not_mutate_or_consume_low_guid_like_cpp() {
    let mut map = test_map();
    let missing_player_guid = guid(HighGuid::Player, 4280201);

    let outcome = create_farsight_focus_for_tests(&mut map, missing_player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::MissingCasterPlayer
    );
    assert_eq!(outcome.dynamic_object_guid, None);
    assert_eq!(map.entity_world.len(), 0);
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        1
    );
}
#[test]
fn farsight_dynamic_object_create_untyped_caster_record_does_not_mutate_like_cpp() {
    let mut map = test_map();
    let mut player_object = world_object_with_counter(HighGuid::Player, 4280301, 571, 7, true);
    let player_guid = player_object.guid();
    player_object.object_mut().add_to_world();
    map.insert_map_object(AccessorObjectKind::Player, player_object)
        .unwrap();

    let outcome = create_farsight_focus_for_tests(&mut map, player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::MissingCasterPlayer
    );
    assert_eq!(map.entity_world.len(), 1);
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        1
    );
}
#[test]
fn farsight_dynamic_object_create_caster_not_in_world_or_wrong_map_do_not_mutate_like_cpp() {
    let mut not_in_world_map = test_map();
    let mut not_in_world_player = test_player_for_viewpoint(4280401);
    let not_in_world_guid = not_in_world_player.guid();
    not_in_world_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    not_in_world_map
        .insert_map_object_record(MapObjectRecord::new_player(not_in_world_player).unwrap())
        .unwrap();

    let not_in_world = create_farsight_focus_for_tests(&mut not_in_world_map, not_in_world_guid);

    assert_eq!(
        not_in_world.status,
        FarsightDynamicObjectCreateStatusLikeCpp::CasterNotInWorld
    );
    assert_eq!(not_in_world_map.entity_world.len(), 1);
    assert_eq!(
        not_in_world_map
            .get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        1
    );

    let mut wrong_map = test_map();
    let wrong_map_player = test_player_for_viewpoint(4280402);
    let wrong_map_guid = wrong_map_player.guid();
    wrong_map
        .insert_map_object_record(MapObjectRecord::new_player(wrong_map_player).unwrap())
        .unwrap();
    wrong_map.map_id = 530;

    let wrong_map_outcome = create_farsight_focus_for_tests(&mut wrong_map, wrong_map_guid);

    assert_eq!(
        wrong_map_outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::CasterWrongMap
    );
    assert_eq!(wrong_map.entity_world.len(), 1);
    assert_eq!(
        wrong_map
            .get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        1
    );
}
#[test]
fn dynamic_object_caster_viewpoint_apply_sets_player_and_toggles_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4260101);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260102);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

    assert_eq!(outcome.player_guid, player_guid);
    assert_eq!(outcome.dynamic_object_guid, dynamic_object_guid);
    assert!(outcome.apply);
    assert_eq!(
        outcome.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved
    );
    assert!(outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Applied
    );
    assert_eq!(outcome.player_set_viewpoint.set_world_object, None);
    assert!(outcome.player_set_viewpoint.update_visibility_requested);
    assert!(outcome.player_set_viewpoint.set_seer_requested);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
}
#[test]
fn dynamic_object_caster_viewpoint_apply_existing_viewpoint_only_toggles_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4260201);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4260209);
    player.set_farsight_object_like_cpp(existing_guid);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260202);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint
    );
    assert_eq!(outcome.player_set_viewpoint.set_world_object, None);
    assert!(!outcome.player_set_viewpoint.update_visibility_requested);
    assert!(!outcome.player_set_viewpoint.set_seer_requested);
    assert!(outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}
#[test]
fn dynamic_object_caster_viewpoint_remove_match_clears_player_and_toggles_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4260301);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260302);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, false);

    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert_eq!(outcome.player_set_viewpoint.set_world_object, None);
    assert!(!outcome.player_set_viewpoint.update_visibility_requested);
    assert!(outcome.player_set_viewpoint.set_seer_requested);
    assert!(outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
}
#[test]
fn dynamic_object_caster_viewpoint_remove_mismatch_only_toggles_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4260401);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4260409);
    player.set_farsight_object_like_cpp(existing_guid);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260402);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, false);

    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch
    );
    assert_eq!(outcome.player_set_viewpoint.set_world_object, None);
    assert!(!outcome.player_set_viewpoint.update_visibility_requested);
    assert!(!outcome.player_set_viewpoint.set_seer_requested);
    assert!(outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}
#[test]
fn dynamic_object_caster_viewpoint_missing_records_do_not_create_or_mutate_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4260501);
    let player_guid = player.guid();
    let missing_dynamic_object_guid = guid(HighGuid::DynamicObject, 4260502);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let missing_dynamic_object =
        map.apply_dynamic_object_caster_viewpoint_like_cpp(missing_dynamic_object_guid, true);

    assert_eq!(
        missing_dynamic_object.status,
        DynamicObjectCasterViewpointStatusLikeCpp::MissingDynamicObject
    );
    assert_eq!(
        missing_dynamic_object.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::MissingTarget
    );
    assert!(!missing_dynamic_object.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert_eq!(map.map_object_count(), 1);

    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260503);
    let dynamic_object_guid = dynamic_object.world().guid();
    let missing_player_guid = guid(HighGuid::Player, 4260504);
    dynamic_object.set_caster_guid(missing_player_guid);
    dynamic_object.bind_to_caster(missing_player_guid);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let missing_player =
        map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

    assert_eq!(
        missing_player.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterNotPlayer
    );
    assert_eq!(
        missing_player.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::MissingPlayer
    );
    assert!(!missing_player.dynamic_object_viewpoint_toggled);
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
    assert_eq!(map.map_object_count(), 2);
}
#[test]
fn dynamic_object_caster_viewpoint_absent_bound_caster_no_mutation_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4260601);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4260602);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

    assert_eq!(
        outcome.status,
        DynamicObjectCasterViewpointStatusLikeCpp::MissingCaster
    );
    assert_eq!(
        outcome.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::MissingPlayer
    );
    assert!(!outcome.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}
#[test]
fn remove_from_map_like_cpp_dynamic_object_caster_viewpoint_match_cleans_player_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4270101);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4270102);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    let viewpoint = removed.dynamic_object_caster_viewpoint.unwrap();
    assert_eq!(viewpoint.player_guid, player_guid);
    assert_eq!(viewpoint.dynamic_object_guid, dynamic_object_guid);
    assert!(!viewpoint.apply);
    assert_eq!(
        viewpoint.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved
    );
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(!viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(viewpoint.player_set_viewpoint.set_seer_requested);
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert!(map.map_object_record(dynamic_object_guid).is_none());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(!removed.object.unwrap().object().is_in_world());
}
#[test]
fn remove_from_map_like_cpp_dynamic_object_caster_viewpoint_mismatch_keeps_player_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4270201);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4270209);
    player.set_farsight_object_like_cpp(existing_guid);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4270202);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, true)
        .unwrap();

    let viewpoint = removed.dynamic_object_caster_viewpoint.unwrap();
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch
    );
    assert!(!viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(!viewpoint.player_set_viewpoint.set_seer_requested);
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}
#[test]
fn remove_from_map_like_cpp_dynamic_object_aura_and_caster_cleanup_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4300101);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4300102);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(caster_guid);
    dynamic_object.set_aura_bound();
    dynamic_object.bind_to_caster(caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert_eq!(
        removed.dynamic_object_remove_cleanup,
        Some(DynamicObjectRemoveCleanupOutcomeLikeCpp {
            had_aura: true,
            removed_aura_pending_delete: true,
            unbound_caster: Some(caster_guid),
        })
    );
    assert!(!removed.object.unwrap().object().is_in_world());
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}
#[test]
fn remove_from_map_like_cpp_dynamic_object_without_aura_or_caster_reports_no_cleanup_like_cpp() {
    let mut map = test_map();
    let dynamic_object = test_dynamic_object_for_viewpoint(4300201);
    let dynamic_object_guid = dynamic_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert_eq!(
        removed.dynamic_object_remove_cleanup,
        Some(DynamicObjectRemoveCleanupOutcomeLikeCpp {
            had_aura: false,
            removed_aura_pending_delete: false,
            unbound_caster: None,
        })
    );
    assert!(!removed.object.unwrap().object().is_in_world());
}
#[test]
fn remove_from_map_like_cpp_dynamic_object_not_in_world_skips_aura_and_caster_cleanup_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4300301);
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4300302);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_caster_guid(caster_guid);
    dynamic_object.set_aura_bound();
    dynamic_object.bind_to_caster(caster_guid);
    dynamic_object.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert_eq!(removed.dynamic_object_remove_cleanup, None);
    assert!(!removed.was_in_world);
    assert!(!removed.object.unwrap().object().is_in_world());
}
#[test]
fn remove_from_map_like_cpp_dynamic_object_viewpoint_aura_and_caster_order_evidence_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4300401);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4300402);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    dynamic_object.set_aura_bound();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, false)
        .unwrap();

    let viewpoint = removed.dynamic_object_caster_viewpoint.unwrap();
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert_eq!(
        removed.dynamic_object_remove_cleanup,
        Some(DynamicObjectRemoveCleanupOutcomeLikeCpp {
            had_aura: true,
            removed_aura_pending_delete: true,
            unbound_caster: Some(player_guid),
        })
    );
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(!removed.object.unwrap().object().is_in_world());
}
#[test]
fn remove_all_area_triggers_for_caster_removes_only_matching_caster_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4340601);
    let other_caster_guid = guid(HighGuid::Player, 4340602);

    let mut matching_area_trigger = test_area_trigger_for_update(4340603, 1_000, true);
    let matching_guid = matching_area_trigger.world().guid();
    matching_area_trigger.set_caster_guid(caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(matching_area_trigger).unwrap())
        .unwrap();

    let mut other_area_trigger = test_area_trigger_for_update(4340604, 1_000, true);
    let other_guid = other_area_trigger.world().guid();
    other_area_trigger.set_caster_guid(other_caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(other_area_trigger).unwrap())
        .unwrap();

    let outcome = map.remove_all_area_triggers_for_caster_like_cpp(caster_guid);

    assert_eq!(outcome.caster_guid, caster_guid);
    assert_eq!(outcome.candidates, 1);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.missing_or_stale, 0);
    assert_eq!(outcome.remove_errors, 0);
    assert!(map.map_object_record(matching_guid).is_none());
    assert!(map.map_object_record(other_guid).is_some());
}
#[test]
fn remove_all_area_triggers_for_caster_ignores_other_object_kinds_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4340611);
    let creature = test_creature_for_spawn(4340612, 4340612, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcome = map.remove_all_area_triggers_for_caster_like_cpp(caster_guid);

    assert_eq!(outcome.candidates, 0);
    assert_eq!(outcome.removed, 0);
    assert!(map.map_object_record(creature_guid).is_some());
}
#[test]
fn scene_object_update_with_creator_and_aura_updates_without_queue_like_cpp() {
    let mut map = test_map();
    let scene_object = test_scene_object_for_update(4370101, true, guid(HighGuid::Cast, 4370102));
    let scene_object_guid = scene_object.world().guid();
    let owner_guid = scene_object.owner_guid();
    let cast_guid = scene_object.created_by_spell_cast();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(scene_object).unwrap())
        .unwrap();

    let outcome = map.update_scene_object_like_cpp(
        scene_object_guid,
        250,
        SceneObjectUpdateContextLikeCpp {
            creator_exists: true,
            linked_aura_exists: true,
        },
    );

    assert_eq!(outcome.status, SceneObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.owner_guid, Some(owner_guid));
    assert_eq!(outcome.created_by_spell_cast, Some(cast_guid));
    assert!(outcome.creator_exists);
    assert!(outcome.linked_aura_exists);
    assert!(outcome.world_update_would_run);
    assert!(!outcome.should_be_removed);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(scene_object_guid).is_some());
}
#[test]
fn scene_object_update_missing_linked_aura_queues_remove_like_cpp() {
    let mut map = test_map();
    let scene_object = test_scene_object_for_update(4370301, true, guid(HighGuid::Cast, 4370302));
    let scene_object_guid = scene_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(scene_object).unwrap())
        .unwrap();

    let outcome = map.update_scene_object_like_cpp(
        scene_object_guid,
        250,
        SceneObjectUpdateContextLikeCpp {
            creator_exists: true,
            linked_aura_exists: false,
        },
    );

    assert_eq!(outcome.status, SceneObjectUpdateStatusLikeCpp::RemoveQueued);
    assert!(outcome.world_update_would_run);
    assert!(outcome.should_be_removed);
    assert_eq!(outcome.remove_list.unwrap().queued, true);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn dynamic_object_update_non_aura_decrements_duration_without_queue_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4290101);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_duration(1_000);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(outcome.dynamic_object_guid, dynamic_object_guid);
    assert_eq!(outcome.elapsed_ms, 250);
    assert_eq!(outcome.status, DynamicObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(750));
    assert!(outcome.script_update_would_run);
    assert_eq!(outcome.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .duration_ms(),
        750
    );
    assert!(
        !map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .world()
            .object()
            .is_destroyed_object()
    );
}
#[test]
fn dynamic_object_update_non_aura_expiry_queues_remove_list_and_preserves_record_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4290201);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_duration(250);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(
        outcome.status,
        DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(outcome.duration_before_ms, Some(250));
    assert_eq!(outcome.duration_after_ms, Some(250));
    assert!(!outcome.script_update_would_run);
    let remove_list = outcome.remove_list.unwrap();
    assert_eq!(remove_list.guid, dynamic_object_guid);
    assert!(remove_list.queued);
    assert!(!remove_list.duplicate);
    assert!(!remove_list.missing_or_stale);
    assert_eq!(remove_list.unsupported_kind, None);
    assert_eq!(remove_list.cleanup_before_delete_count, 1);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(dynamic_object_guid).is_some());
    let dynamic_object = map.get_typed_dynamic_object(dynamic_object_guid).unwrap();
    assert_eq!(dynamic_object.duration_ms(), 250);
    assert!(dynamic_object.world().object().is_destroyed_object());
    assert_eq!(dynamic_object.cleanup_before_delete_count(), 1);
}
