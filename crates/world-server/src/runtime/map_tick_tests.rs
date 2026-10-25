use super::*;
use std::sync::Arc;
use std::time::Instant;
use wow_core::{ObjectGuid, Position};
use wow_entities::{AuraApplicationLikeCpp as AuraApplication, Player};

fn empty_respawn_caches() -> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Default::default(),
        sparring_store: Default::default(),
        difficulty_store: Default::default(),
        base_stats_store: Default::default(),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: Default::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([])),
        creature_equipment_store: Default::default(),
        creature_addon_store: Default::default(),
        spell_x_spell_visual_store: Arc::new(wow_data::SpellXSpellVisualStore::from_entries([])),
        vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
        vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Default::default(),
        gameobject_override_store: Default::default(),
    }
}

fn manager_with_pending_selected_aura(
    guid_counter: i64,
) -> (wow_map::MapManager, wow_map::PlayerHandle) {
    let key = wow_map::MapKey::new(571, 0);
    let guid = ObjectGuid::create_player(1, guid_counter);
    let mut player = Box::new(Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().set_level(80);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .insert_runtime_application_like_cpp(AuraApplication {
            spell_id: 20_584,
            difficulty_id: 0,
            caster_guid: guid,
            slot: 7,
            duration_total: -1,
            duration_remaining: -1,
            stack_count: 1,
            aura_flags: 1,
            effect_mask: 0b111,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        });
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .visible_auras_to_update
        .insert(7);

    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(key.map_id, key.instance_id);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, key, Position::ZERO)
        .unwrap();
    (manager, handle)
}

fn tick_inputs() -> (
    crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    wow_data::ConditionEntriesByTypeStore,
    wow_data::MapStore,
    LoadedGridCreatureRespawnCachesLikeCpp,
) {
    (
        crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::default(),
        wow_data::ConditionEntriesByTypeStore::default(),
        wow_data::MapStore::from_entries(std::iter::empty()),
        empty_respawn_caches(),
    )
}

fn selected_aura_is_pending(manager: &wow_map::MapManager, handle: wow_map::PlayerHandle) -> bool {
    manager
        .with_player_like_cpp(handle, |player| {
            player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .contains(&7)
        })
        .unwrap()
}

#[test]
fn map_tick_flushes_selected_aura_when_creature_owner_is_absent() {
    let (mut manager, handle) = manager_with_pending_selected_aura(8_420_001);

    let plan =
        canonical_map_tick_begin_like_cpp(&mut manager, 100).expect("canonical map tick begins");
    let (metadata, conditions, maps, caches) = tick_inputs();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);

    let summary = canonical_map_tick_resume_with_creature_owner_like_cpp(
        &mut manager,
        plan.plan,
        &mut scheduler,
        &metadata,
        &conditions,
        &maps,
        &caches,
        None,
    )
    .expect("same map tick resumed without a Creature callback");

    assert_eq!(summary.selected_aura_updates.len(), 1);
    let aura_opcode = (wow_constants::ServerOpcodes::AuraUpdate as u16).to_le_bytes();
    assert_eq!(
        summary.selected_aura_updates[0].packet_bytes().get(..2),
        Some(aura_opcode.as_slice())
    );
    assert!(
        manager
            .with_player_like_cpp(handle, |player| player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .is_empty())
            .unwrap()
    );
}

#[test]
fn map_tick_recovery_prelude_continues_once_without_creature_owner() {
    let (mut manager, handle) = manager_with_pending_selected_aura(8_420_002);
    let plan =
        canonical_map_tick_begin_like_cpp(&mut manager, 100).expect("canonical map tick begins");
    let (metadata, conditions, maps, caches) = tick_inputs();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);
    let prelude = prepare_map_recovery_prelude(
        &mut manager,
        plan.plan,
        &mut scheduler,
        &metadata,
        &conditions,
        &maps,
        &caches,
    )
    .expect("admitted respawn prelude is prepared");

    assert_eq!(prelude.plan().epoch_like_cpp(), plan_epoch(&manager));
    assert_eq!(prelude.summary().maps_evaluated, 0);
    assert!(selected_aura_is_pending(&manager, handle));

    let summary =
        continue_map_recovery_prelude(&mut manager, prelude, &metadata, &maps, &caches, None)
            .expect("prepared tick continues without a Creature callback");

    assert_eq!(summary.selected_aura_updates.len(), 1);
    assert!(!selected_aura_is_pending(&manager, handle));
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        wow_map::MapTickCoordinationStateLikeCpp::Idle
    );
}

#[test]
fn map_tick_prelude_rejects_stale_manager_epoch_before_effects() {
    let (mut admitted_manager, _) = manager_with_pending_selected_aura(8_420_003);
    let stale_plan = canonical_map_tick_begin_like_cpp(&mut admitted_manager, 100)
        .expect("source map tick begins");
    let (mut current_manager, current_handle) = manager_with_pending_selected_aura(8_420_004);
    let (metadata, conditions, maps, caches) = tick_inputs();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);

    assert!(
        prepare_map_recovery_prelude(
            &mut current_manager,
            stale_plan.plan,
            &mut scheduler,
            &metadata,
            &conditions,
            &maps,
            &caches,
        )
        .is_none()
    );
    assert_eq!(
        current_manager.tick_coordination_like_cpp(),
        wow_map::MapTickCoordinationStateLikeCpp::Idle
    );
    assert!(selected_aura_is_pending(&current_manager, current_handle));
}

#[test]
fn map_tick_continuation_rejects_stale_epoch_before_selected_collection_or_creature() {
    let (mut prelude_manager, _) = manager_with_pending_selected_aura(8_420_005);
    let plan = canonical_map_tick_begin_like_cpp(&mut prelude_manager, 100)
        .expect("prelude source map tick begins");
    let (metadata, conditions, maps, caches) = tick_inputs();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);
    let prelude = prepare_map_recovery_prelude(
        &mut prelude_manager,
        plan.plan,
        &mut scheduler,
        &metadata,
        &conditions,
        &maps,
        &caches,
    )
    .expect("prelude source is admitted");

    let (mut current_manager, current_handle) = manager_with_pending_selected_aura(8_420_006);
    let mut creature_called = false;
    let mut creature_phase = |_: &mut wow_map::MapManager, _: &wow_map::MapTickPlanLikeCpp| {
        creature_called = true;
    };
    assert!(
        continue_map_recovery_prelude(
            &mut current_manager,
            prelude,
            &metadata,
            &maps,
            &caches,
            Some(&mut creature_phase),
        )
        .is_none()
    );

    assert!(!creature_called);
    assert!(selected_aura_is_pending(&current_manager, current_handle));
    assert_eq!(
        current_manager.tick_coordination_like_cpp(),
        wow_map::MapTickCoordinationStateLikeCpp::Idle
    );
}

#[test]
fn map_tick_keeps_session_owned_death_timer_unchanged() {
    let (mut manager, handle) = manager_with_pending_selected_aura(8_420_007);
    manager
        .with_player_mut_like_cpp(handle, |player| {
            player
                .resurrection_state_mut_like_cpp()
                .death_timer_remaining_ms = 12_345;
        })
        .expect("canonical Player owner");

    let plan =
        canonical_map_tick_begin_like_cpp(&mut manager, 100).expect("canonical map tick begins");
    let (metadata, conditions, maps, caches) = tick_inputs();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);
    let prelude = prepare_map_recovery_prelude(
        &mut manager,
        plan.plan,
        &mut scheduler,
        &metadata,
        &conditions,
        &maps,
        &caches,
    )
    .expect("admitted respawn prelude is prepared");
    let _summary =
        continue_map_recovery_prelude(&mut manager, prelude, &metadata, &maps, &caches, None)
            .expect("prepared map tick continues without a Creature callback");

    assert_eq!(
        manager
            .with_player_like_cpp(handle, |player| player
                .resurrection_state_like_cpp()
                .death_timer_remaining_ms)
            .expect("canonical Player owner"),
        12_345
    );
}

#[test]
fn player_recovery_participants_follow_the_post_respawn_object_visit_plan() {
    let (mut manager, handle) = manager_with_pending_selected_aura(8_420_008);
    let plan = manager
        .begin_tick_like_cpp(100)
        .into_started()
        .expect("the map tick starts");
    let expected_epoch = plan.epoch_like_cpp();
    let expected_diff = plan.effective_diff_ms();
    let (metadata, conditions, maps, caches) = tick_inputs();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);
    let prelude = prepare_map_recovery_prelude(
        &mut manager,
        plan,
        &mut scheduler,
        &metadata,
        &conditions,
        &maps,
        &caches,
    )
    .expect("the admitted respawn prelude is prepared");

    let guid = manager
        .with_player_like_cpp(handle, |player| player.guid())
        .unwrap();
    let participants = player_recovery_pass_maps(&manager, &prelude);
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0].key, wow_map::MapKey::new(571, 0));
    assert_eq!(
        participants[0].incarnation,
        prelude.plan().updated_maps_like_cpp()[0].incarnation
    );
    assert_eq!(participants[0].participants.len(), 1);
    assert_eq!(participants[0].participants[0].guid, guid);
    assert_eq!(prelude.plan().epoch_like_cpp(), expected_epoch);
    assert_eq!(prelude.plan().effective_diff_ms(), expected_diff);

    manager.abandon_tick_like_cpp(prelude.into_plan());
}

fn plan_epoch(manager: &wow_map::MapManager) -> u64 {
    match manager.tick_coordination_like_cpp() {
        wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch) => epoch,
        state => panic!("prepared prelude should remain awaiting its session phase: {state:?}"),
    }
}
