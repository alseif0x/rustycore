use super::fixtures::*;
// External application scenarios migrated with their original assertions.

use super::MapCorpseLoadOutcomeLikeCpp;
use super::fixtures::make_session;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use wow_map::MapManager;
use wow_persistence::{
    MapCorpseAuxiliaryLoadOutcomeLikeCpp,
    MapCorpseLoadOutcomeLikeCpp as PersistedMapCorpseLoadOutcomeLikeCpp,
    MapCorpseLoadRequestLikeCpp, MapCorpseLoadRowLikeCpp, MapCorpsePersistencePortLikeCpp,
    PersistenceFutureLikeCpp,
};

struct MapCorpseLoadPortFixtureLikeCpp {
    requests: Mutex<Vec<MapCorpseLoadRequestLikeCpp>>,
    outcomes: Mutex<VecDeque<PersistedMapCorpseLoadOutcomeLikeCpp>>,
}

impl MapCorpseLoadPortFixtureLikeCpp {
    fn new(outcomes: impl IntoIterator<Item = PersistedMapCorpseLoadOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: Mutex::new(Vec::new()),
            outcomes: Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<MapCorpseLoadRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl MapCorpsePersistencePortLikeCpp for MapCorpseLoadPortFixtureLikeCpp {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        request: MapCorpseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistedMapCorpseLoadOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one map-corpse outcome per request");
        Box::pin(async move { outcome })
    }
}

fn map_corpse_session_with_port_like_cpp(
    outcome: PersistedMapCorpseLoadOutcomeLikeCpp,
) -> (
    wow_world::session::WorldSession,
    Arc<Mutex<MapManager>>,
    Arc<MapCorpseLoadPortFixtureLikeCpp>,
) {
    let port = MapCorpseLoadPortFixtureLikeCpp::new([outcome]);
    let mut manager = MapManager::default();
    manager.create_world_map(571, 9);
    let manager = Arc::new(Mutex::new(manager));
    let (mut session, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.set_map_corpse_persistence_port_like_cpp(port.clone());
    (session, manager, port)
}

fn invalid_map_corpse_load_row_like_cpp() -> MapCorpseLoadRowLikeCpp {
    MapCorpseLoadRowLikeCpp {
        pos_x: 10.0,
        pos_y: 20.0,
        pos_z: 30.0,
        orientation: 1.5,
        map_id: 571,
        display_id: 12_345,
        item_cache: String::new(),
        race: 4,
        class: 1,
        sex: 0,
        flags: 0x20,
        dynamic_flags: 0x01,
        ghost_time: 1_000,
        corpse_type: 0,
        instance_id: 9,
        owner_guid: 77,
    }
}

// The Rust `Failed` port outcomes are synthetic. C++ evidence here covers a missing
// base-query result returning and empty auxiliary results being tolerated.
#[tokio::test]
async fn typed_map_corpse_base_failure_publishes_nothing_like_cpp() {
    let (session, manager, _) =
        map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Failed {
            reason: "base query failed".to_owned(),
        });

    let outcome = session
        .character_load_map_corpse_data_for_test(571, 9)
        .await;

    assert_eq!(outcome, MapCorpseLoadOutcomeLikeCpp::default());
    assert!(
        !manager
            .lock()
            .unwrap()
            .find_map(571, 9)
            .unwrap()
            .map()
            .corpse_data_loaded_like_cpp()
    );
}

#[tokio::test]
async fn typed_map_corpse_auxiliary_failures_are_independent_and_non_fatal_like_cpp() {
    let cases = [
        (
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "phase query failed".to_owned(),
            },
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
        ),
        (
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "customization query failed".to_owned(),
            },
        ),
    ];

    for (phases, customizations) in cases {
        let (session, manager, _) =
            map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Loaded {
                corpses: vec![invalid_map_corpse_load_row_like_cpp()],
                phases,
                customizations,
            });

        let outcome = session
            .character_load_map_corpse_data_for_test(571, 9)
            .await;

        assert_eq!(outcome.invalid_type_rows, 1);
        assert_eq!(outcome.corpses_added, 0);
        assert!(
            manager
                .lock()
                .unwrap()
                .find_map(571, 9)
                .unwrap()
                .map()
                .corpse_data_loaded_like_cpp()
        );
    }
}

// The loaded-empty marker is Rust's idempotence mechanism, not direct C++ field parity.
#[tokio::test]
async fn typed_map_corpse_empty_load_marks_the_map_once_like_cpp() {
    let (session, manager, port) =
        map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Loaded {
            corpses: Vec::new(),
            phases: MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
            customizations: MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
        });

    let outcome = session
        .character_load_map_corpse_data_for_test(571, 9)
        .await;

    assert_eq!(outcome, MapCorpseLoadOutcomeLikeCpp::default());
    assert_eq!(
        port.requests(),
        vec![MapCorpseLoadRequestLikeCpp {
            map_id: 571,
            instance_id: 9,
        }]
    );
    assert!(
        manager
            .lock()
            .unwrap()
            .find_map(571, 9)
            .unwrap()
            .map()
            .corpse_data_loaded_like_cpp()
    );
}
