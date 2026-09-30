//! Real APP prefix cadence and owned boundaries, without production fault hooks.
mod abandon;

use super::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use wow_map::{MapManager, MapTickCoordinationStateLikeCpp, ObjectMapTickError};

struct Catalogs {
    metadata: CanonicalSpawnMetadataLikeCpp,
    conditions: wow_data::ConditionEntriesByTypeStore,
    maps: wow_data::MapStore,
    caches: LoadedGridCreatureRespawnCachesLikeCpp,
}

impl Catalogs {
    fn empty() -> Self {
        Self {
            metadata: CanonicalSpawnMetadataLikeCpp::new(wow_map::SpawnStore::new(), BTreeMap::new()),
            conditions: wow_data::ConditionEntriesByTypeStore::from_conditions_like_cpp([]),
            maps: wow_data::MapStore::from_entries([]),
            caches: LoadedGridCreatureRespawnCachesLikeCpp {
                realm_id: 1,
                template_store: Default::default(),
                sparring_store: Default::default(),
                difficulty_store: Default::default(),
                base_stats_store: Default::default(),
                chr_classes_store: Arc::new(wow_data::character_progression::ChrClassesStore::from_entries([])),
                power_type_store: Arc::new(wow_data::character_progression::PowerTypeStore::from_entries([])),
                health_rates: Default::default(),
                display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
                model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
                model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([])),
                creature_equipment_store: Default::default(),
                creature_addon_store: Default::default(),
                spell_x_spell_visual_store: Arc::new(wow_data::SpellXSpellVisualStore::from_entries([])),
                vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
                vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
                vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts([], [])),
                gameobject_template_store: Default::default(),
                gameobject_override_store: Default::default(),
            },
        }
    }

    fn begin(&self, manager: &mut MapManager, plan: wow_map::MapTickPlanLikeCpp,
        scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp)
        -> Result<CanonicalObjectWork, ObjectWorkBeginFailure>
    {
        CanonicalObjectWork::try_begin(manager, None, plan, scheduler, &self.metadata,
            &self.conditions, &self.maps, &self.caches)
    }
}

fn admitted() -> (MapManager, wow_map::MapTickPlanLikeCpp) {
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    (manager, plan)
}

#[test]
fn before_prefix_returns_original_plan_without_consuming_respawn_cadence() {
    let catalogs = Catalogs::empty();
    let (owner, plan) = admitted();
    let (mut foreign, _foreign_plan) = admitted();
    let epoch = plan.epoch_like_cpp();
    let participants = plan.updated_maps_like_cpp().as_ptr();
    let destroyed = plan.destroyed_maps_like_cpp().as_ptr();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
    let plan = match catalogs.begin(&mut foreign, plan, &mut scheduler) {
        Err(ObjectWorkBeginFailure::BeforePrefix { plan }) => plan,
        _ => panic!("foreign bool preflight must retain the original plan"),
    };
    assert_eq!(plan.epoch_like_cpp(), epoch);
    assert_eq!(plan.effective_diff_ms(), 200);
    assert_eq!(plan.updated_maps_like_cpp().as_ptr(), participants);
    assert_eq!(plan.destroyed_maps_like_cpp().as_ptr(), destroyed);
    assert!(owner.can_resume_tick(&plan));
    assert_eq!(scheduler.timer_ms(), 500);
    assert_eq!(foreign.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch));
    assert!(foreign.find_map(1, 0).unwrap().delayed_update_calls().is_empty());
}

#[test]
fn successful_begin_keeps_timer_only_prefix_even_with_empty_summary() {
    let catalogs = Catalogs::empty();
    let (mut manager, plan) = admitted();
    let epoch = plan.epoch_like_cpp();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
    let work = match catalogs.begin(&mut manager, plan, &mut scheduler) {
        Ok(work) => work,
        Err(_) => panic!("own admitted plan must begin after the same prefix"),
    };
    assert_eq!(scheduler.timer_ms(), 300);
    assert_eq!(work.respawn_summary.maps_evaluated, 0);
    assert!(work.respawn_summary.respawn_db_saves.is_empty());
    assert!(work.respawn_summary.respawn_db_deletes.is_empty());
    assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Resuming(epoch));
    assert_eq!(manager.updater.wait_calls(), 0);
    assert!(manager.find_map(1, 0).unwrap().delayed_update_calls().is_empty());
}

#[test]
fn successful_due_begin_retains_actual_prefix_summary_and_cadence_reset() {
    let catalogs = Catalogs::empty();
    let (mut manager, plan) = admitted();
    let epoch = plan.epoch_like_cpp();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(200);
    let work = match catalogs.begin(&mut manager, plan, &mut scheduler) {
        Ok(work) => work,
        Err(_) => panic!("the actual due prefix must precede object BEGIN"),
    };
    assert_eq!(scheduler.timer_ms(), 200);
    assert_eq!(work.respawn_summary.maps_evaluated, 1);
    assert_eq!(work.respawn_summary.outcomes, 0);
    assert_eq!(manager.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::Resuming(epoch));
    assert_eq!(manager.updater.wait_calls(), 0);
}

#[test]
fn after_prefix_packaging_keeps_actual_summary_and_plan_without_replaying_begin() {
    let catalogs = Catalogs::empty();
    let (mut owner, plan) = admitted();
    let mut foreign = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    let epoch = plan.epoch_like_cpp();
    let participants = plan.updated_maps_like_cpp().as_ptr();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(200);
    let respawn_summary = canonical_map_tick_respawn_phase_like_cpp(&mut owner,
        plan.updated_maps_like_cpp(), None, plan.effective_diff_ms(), &mut scheduler,
        &catalogs.metadata, &catalogs.conditions, &catalogs.maps, &catalogs.caches);
    let (error, plan) = match foreign.try_begin_object_tick(plan) {
        Err(rejected) => rejected,
        Ok(_) => panic!("foreign manager must return the original admitted plan"),
    };
    // Boundary packaging only: normal try_begin cannot change manager origin
    // between its bool preflight and BEGIN under the same exclusive borrow.
    // This manually pairs a real prefix result with a real foreign rejection;
    // it does NOT simulate a reachable production AfterPrefix failure.
    let failure = CanonicalObjectResumeFailure::BeginRejected {
        failure: ObjectWorkBeginFailure::AfterPrefix { error, plan, respawn_summary },
    };
    match failure.retry(&mut owner, &catalogs.metadata, &catalogs.maps, &catalogs.caches) {
        Err(CanonicalObjectResumeFailure::BeginRejected { failure: ObjectWorkBeginFailure::AfterPrefix {
            error: ObjectMapTickError::OriginMismatch { plan_epoch }, plan, respawn_summary,
        } }) => {
            assert_eq!(plan_epoch, epoch);
            assert_eq!(plan.updated_maps_like_cpp().as_ptr(), participants);
            assert!(owner.can_resume_tick(&plan));
            assert_eq!(respawn_summary.maps_evaluated, 1);
            assert_eq!(respawn_summary.outcomes, 0);
        }
        _ => panic!("BEGIN retry must return the whole original failure unchanged"),
    }
    assert_eq!(scheduler.timer_ms(), 200);
    assert_eq!(owner.tick_coordination_like_cpp(), MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch));
    assert_eq!(owner.updater.wait_calls(), 0);
    assert!(owner.find_map(1, 0).unwrap().delayed_update_calls().is_empty());
}
