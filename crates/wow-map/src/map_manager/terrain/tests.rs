use super::LiveTerrainHeights;
use crate::map::MapWorldObjectEnvironment;
use crate::{
    SharedStaticVMapLineOfSightProvider, StaticVMapLineOfSightProvider, VMapLineOfSightQuery,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use wow_constants::{TypeId, TypeMask};
use wow_core::Position;
use wow_entities::{LineOfSightOptions, LineOfSightQuery, WorldObject};

#[derive(Debug)]
struct RecordingLiveStaticVMapLos {
    result: bool,
    calls: Mutex<Vec<VMapLineOfSightQuery>>,
}

impl RecordingLiveStaticVMapLos {
    fn new(result: bool) -> Self {
        Self {
            result,
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl StaticVMapLineOfSightProvider for RecordingLiveStaticVMapLos {
    fn is_in_line_of_sight(&self, query: VMapLineOfSightQuery) -> bool {
        self.calls
            .lock()
            .expect("recording live vmap LOS calls poisoned")
            .push(query);
        self.result
    }
}

fn unique_temp_data_dir(test_name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!("rustycore-{test_name}-{unique}"));
    std::fs::create_dir_all(data_dir.join("maps")).expect("create maps test dir");
    data_dir
}

#[test]
fn live_terrain_wires_static_vmap_los_provider_into_map_cache_like_cpp() {
    let dir = unique_temp_data_dir("live-vmap-los-provider");
    let provider = Arc::new(RecordingLiveStaticVMapLos::new(false));
    let shared_provider: SharedStaticVMapLineOfSightProvider = provider.clone();
    let terrain_cache =
        LiveTerrainHeights::new_with_static_vmap_line_of_sight(&dir, shared_provider);

    let terrain = terrain_cache.terrain_for_map(1);
    let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    source.relocate(Position::new(10.0, 10.0, 1.0, 0.0));
    let query = LineOfSightQuery::to_position_like_cpp(
        &source,
        Position::new(20.0, 10.0, 1.0, 0.0),
        LineOfSightOptions::default(),
    );

    assert!(
        !terrain.line_of_sight(query),
        "live terrain must not bypass an installed static VMAP LOS provider"
    );
    let calls = provider
        .calls
        .lock()
        .expect("recording live vmap LOS calls poisoned");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].map_id, 1);

    let _ = std::fs::remove_dir_all(&dir);
}
