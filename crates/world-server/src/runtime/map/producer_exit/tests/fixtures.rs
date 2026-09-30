//! Actual admitted plan, real bool rejection and the existing producer inputs.
use super::*;
use crate::runtime::map::{
    CanonicalRespawnConditionSchedulerLikeCpp, LoadedGridCreatureRespawnCachesLikeCpp,
};
use crate::runtime::map_tick::object_work::{CanonicalObjectWork, ObjectWorkBeginFailure};
use std::collections::BTreeMap;
use wow_map::MapManager;

pub(super) fn catalogs() -> (
    crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    wow_data::ConditionEntriesByTypeStore,
    wow_data::MapStore,
    LoadedGridCreatureRespawnCachesLikeCpp,
) {
    (
        crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            wow_map::SpawnStore::new(),
            BTreeMap::new(),
        ),
        wow_data::ConditionEntriesByTypeStore::from_conditions_like_cpp([]),
        wow_data::MapStore::from_entries([]),
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
            spell_x_spell_visual_store: Arc::new(
                wow_data::SpellXSpellVisualStore::from_entries([]),
            ),
            vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
            vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
            vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
                [],
                [],
            )),
            gameobject_template_store: Default::default(),
            gameobject_override_store: Default::default(),
        },
    )
}

pub(super) fn rejected_begin() -> (
    MapManager,
    CanonicalObjectResumeFailure,
    *const wow_map::MapTickParticipantLikeCpp,
    *const wow_map::MapTickParticipantLikeCpp,
) {
    let (metadata, conditions, maps, caches) = catalogs();
    let mut owner = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    owner.create_world_map(1, 0);
    owner.create_world_map(2, 0);
    owner.create_map_entry(
        33,
        7,
        1,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    owner.find_map_mut(33, 7).unwrap().set_can_unload(true);
    let plan = owner.begin_tick_like_cpp(200).into_started().unwrap();
    let updated = plan.updated_maps_like_cpp().as_ptr();
    let destroyed = plan.destroyed_maps_like_cpp().as_ptr();
    let mut foreign = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(500);
    let failure = match CanonicalObjectWork::try_begin(
        &mut foreign,
        None,
        plan,
        &mut scheduler,
        &metadata,
        &conditions,
        &maps,
        &caches,
    ) {
        Err(failure @ ObjectWorkBeginFailure::BeforePrefix { .. }) => {
            CanonicalObjectResumeFailure::BeginRejected { failure }
        }
        _ => panic!("actual foreign preflight must return its original plan"),
    };
    (owner, failure, updated, destroyed)
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
        Box::pin(async { panic!("disabled game-event deadline must not execute IO") })
    }
}

pub(super) fn spawn_empty_producer(
    manager: Arc<Mutex<MapManager>>,
    registry: Arc<crate::ActiveWorldSessionRegistryLikeCpp>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<CanonicalMapProducerExit> {
    let (metadata, conditions, maps, caches) = catalogs();
    crate::runtime::map::spawn_canonical_map_update_loop(
        manager,
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new())),
        1,
        u32::MAX,
        Arc::new(Mutex::new(metadata)),
        Arc::new(conditions),
        Arc::new(maps),
        Arc::new(NoGameEventIo),
        crate::RespawnDbWriterSenderLikeCpp::new_like_cpp(),
        Arc::new(Mutex::new(())),
        stop,
        caches,
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
