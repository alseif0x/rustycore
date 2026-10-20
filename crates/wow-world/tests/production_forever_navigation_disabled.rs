// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: legacy pathfinder feature-off regression.
#![cfg(all(target_os = "linux", not(feature = "forever-navigation")))]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use wow_core::Position;
use wow_entities::PhaseShift;
use wow_recastdetour::{DetourOwnerCapabilitiesLikeCpp, PathQueryFilterContext};
use wow_world::{WorldMMapPathfinderWorkerLikeCpp, pathfinder::WorldMMapPathRequestLikeCpp};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn default_feature_off_worker_keeps_legacy_no_mesh_result() {
    let data_dir = std::env::temp_dir().join(format!(
        "rustycore-navigation-disabled-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&data_dir).unwrap();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn(&data_dir);
    let result = worker
        .calculate_path_like_cpp(WorldMMapPathRequestLikeCpp {
            start: Position::xyz(5.0, 5.0, 0.0),
            destination: Position::xyz(25.0, 25.0, 0.0),
            mesh_map_id: 48,
            instance_map_id: 48,
            instance_id: 9,
            filter_context: PathQueryFilterContext::creature(true, false, false, false),
            owner: DetourOwnerCapabilitiesLikeCpp::default(),
            previous_poly_refs: Vec::new(),
            force_destination: false,
            point_path_limit: 64,
            phase_shift: PhaseShift::default(),
        })
        .unwrap();

    assert!(result.is_none());
    drop(worker);
    fs::remove_dir_all(PathBuf::from(data_dir)).unwrap();
}
