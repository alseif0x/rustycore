//! Concrete empty catalogs and actual post-session map work.
use super::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub(super) struct Catalogs {
    pub(super) metadata: CanonicalSpawnMetadataLikeCpp,
    pub(super) conditions: wow_data::ConditionEntriesByTypeStore,
    pub(super) maps: wow_data::MapStore,
    pub(super) caches: LoadedGridCreatureRespawnCachesLikeCpp,
}

impl Catalogs {
    pub(super) fn empty() -> Self {
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

    pub(super) fn retry(
        &self,
        failure: CanonicalObjectResumeFailure,
        manager: &mut MapManager,
    ) -> Result<Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp>, CanonicalObjectResumeFailure>
    {
        failure.retry(manager, &self.metadata, &self.maps, &self.caches)
    }
}

pub(super) fn setup(ids: &[u32]) -> (MapManager, CanonicalObjectWork) {
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    for id in ids {
        manager.create_world_map(*id, 0);
    }
    manager.updater.activate(1);
    let plan = manager.begin_tick_like_cpp(200).into_started().unwrap();
    let object_tick = manager.begin_object_tick(plan).unwrap();
    (
        manager,
        CanonicalObjectWork {
            object_tick,
            respawn_summary: Default::default(),
        },
    )
}

pub(super) fn seed_summary(
    work: &mut CanonicalObjectWork,
) -> *const (u32, u32, ObjectGuid, ObjectGuid) {
    work.respawn_summary
        .expired_pvp_combat_refs
        .push((1, 0, ObjectGuid::EMPTY, ObjectGuid::EMPTY));
    work.respawn_summary.respawn_db_delete_failed = 17;
    work.respawn_summary.expired_pvp_combat_refs.as_ptr()
}

pub(super) fn delivered(
    result: Result<
        Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp>,
        CanonicalObjectResumeFailure,
    >,
) -> CanonicalSpawnGroupConditionTickSummaryLikeCpp {
    match result {
        Ok(Some(summary)) => summary,
        _ => panic!("original nonempty summary must be delivered"),
    }
}

pub(super) struct NoGameEventIo;

impl wow_persistence::GameEventPersistencePortLikeCpp for NoGameEventIo {
    fn load_condition_saves_like_cpp<'a>(
        &'a self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp,
    > {
        Box::pin(async { panic!("empty producer must not load game-event data") })
    }

    fn execute_mutation_like_cpp<'a>(
        &'a self,
        _: wow_persistence::GameEventPersistenceMutationLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::GameEventPersistenceMutationOutcomeLikeCpp,
    > {
        Box::pin(async { panic!("disabled game-event deadline must not execute DB IO") })
    }
}

pub(super) fn spawn_empty_producer(
    manager: Arc<Mutex<MapManager>>,
    registry: Arc<crate::ActiveWorldSessionRegistryLikeCpp>,
    stop: Arc<std::sync::atomic::AtomicBool>,
) -> tokio::task::JoinHandle<crate::runtime::map::CanonicalMapProducerExit> {
    let catalogs = Catalogs::empty();
    crate::runtime::map::spawn_canonical_map_update_loop(
        manager,
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new())),
        1,
        u32::MAX,
        Arc::new(Mutex::new(catalogs.metadata)),
        Arc::new(catalogs.conditions),
        Arc::new(catalogs.maps),
        Arc::new(NoGameEventIo),
        crate::RespawnDbWriterSenderLikeCpp::new_like_cpp(),
        Arc::new(Mutex::new(())),
        stop,
        catalogs.caches,
        Arc::new(wow_data::AreaTriggerTemplateStore::default()),
        crate::runtime::CanonicalGameEventSchedulerLikeCpp::start_system(u64::from(u32::MAX)),
        Arc::new(wow_world::session::directory::PlayerRegistry::new()),
        registry,
        Arc::new(wow_data::BattlemasterListStore::from_entries([])),
        Arc::new(Mutex::new(
            crate::spawn_store_loader::WorldStateMgrLikeCpp::default(),
        )),
    )
}
