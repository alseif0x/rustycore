//! Behaviour of the application-owned stages, using the real MapManager.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::CanonicalObjectWork;
use crate::runtime::map::{
    CanonicalRespawnConditionSchedulerLikeCpp, LoadedGridCreatureRespawnCachesLikeCpp,
};
use crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp;
use wow_map::map::LoadedGridRespawnRecordsLikeCpp;
use wow_map::{
    Map, MapKey, MapManager, MapTickCoordinationStateLikeCpp, MapTickPlanLikeCpp,
    ObjectMapUpdateToken, SpawnId, SpawnObjectType,
};

struct RespawnCatalogs {
    metadata: CanonicalSpawnMetadataLikeCpp,
    conditions: wow_data::ConditionEntriesByTypeStore,
    maps: wow_data::MapStore,
    caches: LoadedGridCreatureRespawnCachesLikeCpp,
}

impl RespawnCatalogs {
    fn empty() -> Self {
        Self {
            metadata: CanonicalSpawnMetadataLikeCpp::new(
                wow_map::SpawnStore::new(),
                BTreeMap::new(),
            ),
            conditions: wow_data::ConditionEntriesByTypeStore::from_conditions_like_cpp([]),
            maps: wow_data::MapStore::from_entries([]),
            caches: LoadedGridCreatureRespawnCachesLikeCpp {
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
                model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries(
                    [],
                )),
                creature_equipment_store: Default::default(),
                creature_addon_store: Default::default(),
                spell_x_spell_visual_store: Arc::new(
                    wow_data::SpellXSpellVisualStore::from_entries([]),
                ),
                vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
                vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
                vehicle_accessory_store: Arc::new(
                    wow_data::VehicleAccessoryStoreLikeCpp::from_parts([], []),
                ),
                gameobject_template_store: Default::default(),
                gameobject_override_store: Default::default(),
            },
        }
    }

    fn begin(
        &self,
        manager: &mut MapManager,
        plan: MapTickPlanLikeCpp,
        scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    ) -> Option<CanonicalObjectWork> {
        CanonicalObjectWork::begin(
            manager,
            None,
            plan,
            scheduler,
            &self.metadata,
            &self.conditions,
            &self.maps,
            &self.caches,
        )
    }
}

fn admitted_plan(manager: &mut MapManager, diff_ms: u32) -> MapTickPlanLikeCpp {
    manager
        .begin_tick_like_cpp(diff_ms)
        .into_started()
        .expect("tick admitted")
}

fn no_record(
    _: &mut Map,
    _: SpawnObjectType,
    _: SpawnId,
) -> Option<LoadedGridRespawnRecordsLikeCpp> {
    None
}

fn finish_without_record(
    work: &mut CanonicalObjectWork,
    manager: &mut MapManager,
    token: ObjectMapUpdateToken,
) -> Option<()> {
    let mut load_record = no_record;
    work.finish_map(manager, token, None, &mut load_record)
}

#[test]
fn foreign_plan_is_rejected_before_respawn_cadence_is_consumed() {
    let catalogs = RespawnCatalogs::empty();
    let mut owner = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    let mut recipient = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    owner.create_world_map(1, 0);
    recipient.create_world_map(1, 0);
    let foreign = admitted_plan(&mut owner, 200);
    let own = admitted_plan(&mut recipient, 200);
    let epoch = own.epoch_like_cpp();
    assert_eq!(foreign.epoch_like_cpp(), epoch);
    assert_eq!(foreign.updated_maps_like_cpp(), own.updated_maps_like_cpp());
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);

    assert!(
        catalogs
            .begin(&mut recipient, foreign, &mut scheduler)
            .is_none()
    );
    assert_eq!(scheduler.timer_ms(), 500);
    assert_eq!(
        recipient.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
    );
    assert!(recipient.can_resume_tick(&own));
    assert!(
        recipient
            .find_map(1, 0)
            .unwrap()
            .delayed_update_calls()
            .is_empty()
    );

    let work = catalogs
        .begin(&mut recipient, own, &mut scheduler)
        .expect("own plan accepted");
    assert_eq!(scheduler.timer_ms(), 300);
    drop(work);
}

#[test]
fn foreign_work_and_token_leave_the_recipient_inflight_slot_intact() {
    let catalogs = RespawnCatalogs::empty();
    let mut first = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    let mut second = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    first.create_world_map(1, 0);
    second.create_world_map(1, 0);
    let first_plan = admitted_plan(&mut first, 200);
    let second_plan = admitted_plan(&mut second, 200);
    let epoch = first_plan.epoch_like_cpp();
    assert_eq!(epoch, second_plan.epoch_like_cpp());
    let mut first_scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
    let mut second_scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
    let mut first_work = catalogs
        .begin(&mut first, first_plan, &mut first_scheduler)
        .unwrap();
    let mut second_work = catalogs
        .begin(&mut second, second_plan, &mut second_scheduler)
        .unwrap();

    assert!(first_work.prepare_next(&mut second).is_none());
    let first_token = first_work.prepare_next(&mut first).unwrap().unwrap();
    let second_token = second_work.prepare_next(&mut second).unwrap().unwrap();
    assert_eq!(first_token.key(), second_token.key());
    assert_eq!(first_token.incarnation(), second_token.incarnation());
    assert!(finish_without_record(&mut second_work, &mut second, first_token).is_none());
    assert!(second_work.prepare_next(&mut second).is_none());
    assert_eq!(
        second.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(epoch)
    );

    finish_without_record(&mut second_work, &mut second, second_token).unwrap();
    assert!(second_work.prepare_next(&mut second).unwrap().is_none());
    second_work.complete(&mut second, &catalogs.maps).unwrap();
    assert!(first_work.complete(&mut second, &catalogs.maps).is_none());
    assert_eq!(
        second.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    assert_eq!(
        first.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(epoch)
    );
    assert!(first.begin_tick_like_cpp(1).is_busy());
}

#[test]
fn one_inflight_map_preserves_order_and_diff_until_successful_finalization() {
    let catalogs = RespawnCatalogs::empty();
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(2, 0);
    manager.create_world_map(1, 0);
    let plan = admitted_plan(&mut manager, 200);
    let epoch = plan.epoch_like_cpp();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(200);
    let mut work = catalogs.begin(&mut manager, plan, &mut scheduler).unwrap();
    let first = work.prepare_next(&mut manager).unwrap().unwrap();
    assert_eq!(first.key(), MapKey::new(1, 0));
    assert_eq!(first.effective_diff_ms(), 200);
    assert!(work.prepare_next(&mut manager).is_none());
    assert!(manager.begin_tick_like_cpp(999).is_busy());

    finish_without_record(&mut work, &mut manager, first).unwrap();
    let second = work.prepare_next(&mut manager).unwrap().unwrap();
    assert_eq!(second.key(), MapKey::new(2, 0));
    assert_eq!(second.effective_diff_ms(), 200);
    finish_without_record(&mut work, &mut manager, second).unwrap();
    assert!(work.prepare_next(&mut manager).unwrap().is_none());
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Resuming(epoch)
    );
    for id in [1, 2] {
        assert!(
            manager
                .find_map(id, 0)
                .unwrap()
                .delayed_update_calls()
                .is_empty()
        );
    }

    let summary = work.complete(&mut manager, &catalogs.maps).unwrap();
    assert_eq!(summary.maps_evaluated, 2);
    assert_eq!(scheduler.timer_ms(), 200);
    assert_eq!(
        manager.tick_coordination_like_cpp(),
        MapTickCoordinationStateLikeCpp::Idle
    );
    for id in [1, 2] {
        assert_eq!(
            manager.find_map(id, 0).unwrap().delayed_update_calls(),
            [200]
        );
    }
    assert!(manager.begin_tick_like_cpp(199).into_started().is_none());
    assert_eq!(admitted_plan(&mut manager, 1).effective_diff_ms(), 200);
}

#[test]
fn stale_token_finishes_without_running_the_replacement_object_tail() {
    let catalogs = RespawnCatalogs::empty();
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    let plan = admitted_plan(&mut manager, 200);
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
    let mut work = catalogs.begin(&mut manager, plan, &mut scheduler).unwrap();
    let token = work.prepare_next(&mut manager).unwrap().unwrap();
    let admitted_incarnation = token.incarnation();
    assert!(manager.destroy_map(1, 0));
    manager.create_world_map(1, 0);
    assert_ne!(
        manager.map_incarnation_like_cpp(MapKey::new(1, 0)).unwrap(),
        admitted_incarnation
    );
    let mut unexpected_load =
        |_: &mut Map, _: SpawnObjectType, _: SpawnId| -> Option<LoadedGridRespawnRecordsLikeCpp> {
            panic!("stale finish must not call the respawn record loader")
        };

    work.finish_map(&mut manager, token, None, &mut unexpected_load)
        .unwrap();
    let replacement = manager.find_map(1, 0).unwrap();
    assert!(
        !replacement
            .last_map_update_tail_summary_like_cpp()
            .script_hook
            .invoked
    );
    assert_eq!(replacement.last_creatures_update_summary().visited, 0);
    assert!(replacement.delayed_update_calls().is_empty());
    assert!(work.prepare_next(&mut manager).unwrap().is_none());
    work.complete(&mut manager, &catalogs.maps).unwrap();
    assert_eq!(
        manager.find_map(1, 0).unwrap().delayed_update_calls(),
        [200]
    );
}

#[test]
fn incomplete_or_inflight_completion_does_not_finalize_the_tick() {
    for prepare_map in [false, true] {
        let catalogs = RespawnCatalogs::empty();
        let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
        manager.create_world_map(1, 0);
        let plan = admitted_plan(&mut manager, 200);
        let epoch = plan.epoch_like_cpp();
        let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
        let mut work = catalogs.begin(&mut manager, plan, &mut scheduler).unwrap();
        let token = prepare_map.then(|| work.prepare_next(&mut manager).unwrap().unwrap());

        assert!(work.complete(&mut manager, &catalogs.maps).is_none());
        assert_eq!(
            manager.tick_coordination_like_cpp(),
            MapTickCoordinationStateLikeCpp::Resuming(epoch)
        );
        assert!(manager.begin_tick_like_cpp(1).is_busy());
        assert!(
            manager
                .find_map(1, 0)
                .unwrap()
                .delayed_update_calls()
                .is_empty()
        );
        drop(token);
    }
}

#[test]
fn dropping_work_or_prepared_token_does_not_make_the_manager_idle() {
    for prepare_map in [false, true] {
        let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
        manager.create_world_map(1, 0);
        let plan = admitted_plan(&mut manager, 200);
        let epoch = plan.epoch_like_cpp();
        let mut work = {
            let catalogs = RespawnCatalogs::empty();
            let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
            catalogs.begin(&mut manager, plan, &mut scheduler).unwrap()
        };
        // All begin-only metadata/cache borrows have already ended here.
        if prepare_map {
            let token = work.prepare_next(&mut manager).unwrap().unwrap();
            drop(token);
            assert!(work.prepare_next(&mut manager).is_none());
        }
        drop(work);

        assert_eq!(
            manager.tick_coordination_like_cpp(),
            MapTickCoordinationStateLikeCpp::Resuming(epoch)
        );
        assert!(manager.begin_tick_like_cpp(1).is_busy());
        assert!(
            manager
                .find_map(1, 0)
                .unwrap()
                .delayed_update_calls()
                .is_empty()
        );
        assert!(
            !manager
                .find_map(1, 0)
                .unwrap()
                .last_map_update_tail_summary_like_cpp()
                .script_hook
                .invoked
        );
    }
}
