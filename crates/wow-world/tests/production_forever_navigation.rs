// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: synthetic selected Forever navigation consumer tests.
#![cfg(all(feature = "forever-navigation", target_os = "linux"))]

#[path = "production_forever_navigation/support.rs"]
mod support;

use std::fs;

use support::{EMPTY, SELECTED_MAP, SyntheticNavigationPackage, WRITTEN_A, WRITTEN_B, WRITTEN_C};
use wow_core::Position;
use wow_entities::PhaseShift;
use wow_recastdetour::{
    DT_PARTIAL_RESULT_LIKE_CPP, DT_SUCCESS_LIKE_CPP, DetourOwnerCapabilitiesLikeCpp,
    PathQueryFilterContext,
};
#[cfg(feature = "test-fixtures")]
use wow_world::pathfinder::ForeverNavigationTestFailurePlan;
use wow_world::{
    WorldMMapPathfinderWorkerLikeCpp,
    pathfinder::{
        ForeverNavigationContext, ForeverNavigationError, ForeverNavigationLimits,
        ForeverNavigationQueryRequest, WorldDetourPathError, WorldMMapPathRequestLikeCpp,
    },
};

fn query(start: Position, destination: Position) -> ForeverNavigationQueryRequest {
    ForeverNavigationQueryRequest {
        map_id: SELECTED_MAP,
        instance_map_id: SELECTED_MAP,
        instance_id: 9,
        start,
        destination,
        phase_shift: PhaseShift::default(),
        filter_context: PathQueryFilterContext::creature(true, false, false, false),
        half_extents: [2.0, 4.0, 2.0],
        max_path: 256,
    }
}

#[test]
fn public_worker_prepares_unloads_and_returns_raw_complete_and_partial_corridors() {
    let package = SyntheticNavigationPackage::create();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        ForeverNavigationLimits::default(),
    )
    .expect("admit synthetic package on the selected worker");

    let initial = worker.forever_navigation_stats().unwrap();
    assert_eq!(initial.map_id, SELECTED_MAP);
    assert_eq!(initial.loaded_tiles, 0);
    assert_eq!(initial.instance_queries, 0);
    assert!(initial.global_artifact_bytes > 0);

    worker
        .prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A])
        .unwrap();
    worker
        .prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A])
        .unwrap();
    let prepared = worker.forever_navigation_stats().unwrap();
    assert_eq!(prepared.loaded_tiles, 1);
    assert!(prepared.loaded_tile_artifact_bytes > 0);

    let complete = worker
        .query_forever_navigation_corridor(query(
            Position::xyz(5.0, 5.0, 0.0),
            Position::xyz(25.0, 25.0, 0.0),
        ))
        .unwrap();
    assert!(complete.start.poly_ref.is_some());
    assert!(complete.destination.poly_ref.is_some());
    assert_ne!(complete.path.status & DT_SUCCESS_LIKE_CPP, 0);
    assert_eq!(complete.path.status & DT_PARTIAL_RESULT_LIKE_CPP, 0);
    assert!(complete.path.polys.len() > 1);

    let no_nearest = worker
        .query_forever_navigation_corridor(query(
            Position::xyz(5.0, 5.0, 0.0),
            Position::xyz(50.0, 50.0, 0.0),
        ))
        .unwrap_err();
    assert!(matches!(
        no_nearest,
        ForeverNavigationError::NoNearestPolygon {
            start,
            destination
        } if start.poly_ref.is_some() && destination.poly_ref.is_none()
    ));

    let grid_size = forever_navigation::tilebuilder::GRID_SIZE;
    let outside = worker
        .query_forever_navigation_corridor(query(
            Position::xyz(-2.0 * grid_size + 5.0, 5.0, 0.0),
            Position::xyz(5.0, 5.0, 0.0),
        ))
        .unwrap_err();
    assert!(matches!(
        outside,
        ForeverNavigationError::TileOutsideCoverage(tile)
            if tile.row == 33 && tile.column == 31
    ));

    let empty = worker
        .query_forever_navigation_corridor(query(
            Position::xyz(-grid_size + 5.0, 5.0, 0.0),
            Position::xyz(5.0, 5.0, 0.0),
        ))
        .unwrap_err();
    assert!(matches!(empty, ForeverNavigationError::DeclaredEmptyTile(tile) if tile == EMPTY));

    worker
        .prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_B])
        .unwrap();
    let partial = worker
        .query_forever_navigation_corridor(query(
            Position::xyz(-533.3333 + 5.0, -533.3333 + 5.0, 0.0),
            Position::xyz(5.0, 5.0, 0.0),
        ))
        .unwrap();
    assert!(partial.start.poly_ref.is_some());
    assert!(partial.destination.poly_ref.is_some());
    assert_ne!(partial.path.status & DT_SUCCESS_LIKE_CPP, 0);
    assert_ne!(partial.path.status & DT_PARTIAL_RESULT_LIKE_CPP, 0);

    worker
        .unload_forever_navigation_instance(SELECTED_MAP, SELECTED_MAP, 9)
        .unwrap();
    assert_eq!(
        worker.forever_navigation_stats().unwrap().instance_queries,
        0
    );
    worker
        .unload_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_B])
        .unwrap();
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 1);
    worker
        .prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_B])
        .unwrap();
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 2);

    let legacy = worker.calculate_path_like_cpp(WorldMMapPathRequestLikeCpp {
        start: Position::xyz(5.0, 5.0, 0.0),
        destination: Position::xyz(25.0, 25.0, 0.0),
        mesh_map_id: SELECTED_MAP,
        instance_map_id: SELECTED_MAP,
        instance_id: 9,
        filter_context: PathQueryFilterContext::creature(true, false, false, false),
        owner: DetourOwnerCapabilitiesLikeCpp::default(),
        previous_poly_refs: Vec::new(),
        force_destination: false,
        point_path_limit: 64,
        phase_shift: PhaseShift::default(),
    });
    assert!(matches!(
        legacy,
        Err(WorldDetourPathError::SelectedNavigationRequiresExplicitQuery)
    ));
}

#[test]
fn public_asymmetric_grid_keys_use_wow_x_as_row_and_wow_y_as_column() {
    let package = SyntheticNavigationPackage::create();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    let grid_size = forever_navigation::tilebuilder::GRID_SIZE;

    worker
        .prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_C])
        .unwrap();
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 1);
    let asymmetric = worker
        .query_forever_navigation_corridor(query(
            Position::xyz(5.0, -grid_size + 5.0, 0.0),
            Position::xyz(25.0, -grid_size + 25.0, 0.0),
        ))
        .unwrap();
    assert_ne!(asymmetric.path.status & DT_SUCCESS_LIKE_CPP, 0);
    assert_eq!(asymmetric.path.status & DT_PARTIAL_RESULT_LIKE_CPP, 0);

    worker
        .unload_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_C])
        .unwrap();
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);
    let transposed = worker
        .query_forever_navigation_corridor(query(
            Position::xyz(-grid_size + 5.0, 5.0, 0.0),
            Position::xyz(-grid_size + 25.0, 25.0, 0.0),
        ))
        .unwrap_err();
    assert!(matches!(
        transposed,
        ForeverNavigationError::DeclaredEmptyTile(tile) if tile == EMPTY
    ));
    let unloaded_written_counterpart = worker
        .query_forever_navigation_corridor(query(
            Position::xyz(5.0, -grid_size + 5.0, 0.0),
            Position::xyz(25.0, -grid_size + 25.0, 0.0),
        ))
        .unwrap_err();
    assert!(matches!(
        unloaded_written_counterpart,
        ForeverNavigationError::TileNotPrepared(tile) if tile == WRITTEN_C
    ));
}

#[test]
fn expected_context_and_tile_placement_are_checked_before_native_admission() {
    let package = SyntheticNavigationPackage::create();

    let mut absent_context = package.expected;
    absent_context.context = None;
    assert!(matches!(
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
            &package.output,
            absent_context,
            ForeverNavigationLimits::default(),
        ),
        Err(ForeverNavigationError::InvalidExpected { field: "context" })
    ));

    let mut wrong_build = package.expected;
    let mut context: ForeverNavigationContext = wrong_build.context.unwrap();
    context.build = 70_374;
    wrong_build.context = Some(context);
    assert!(matches!(
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
            &package.output,
            wrong_build,
            ForeverNavigationLimits::default(),
        ),
        Err(ForeverNavigationError::InvalidExpected { field: "build" })
    ));

    let mut wrong_plan = package.expected;
    let mut context = wrong_plan.context.unwrap();
    context.plan_sha256[0] ^= 1;
    wrong_plan.context = Some(context);
    assert!(matches!(
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
            &package.output,
            wrong_plan,
            ForeverNavigationLimits::default(),
        ),
        Err(ForeverNavigationError::InvalidExpected {
            field: "plan_sha256"
        })
    ));

    let mut wrong_selected_map = package.expected;
    wrong_selected_map.selected_map += 1;
    assert!(
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
            &package.output,
            wrong_selected_map,
            ForeverNavigationLimits::default(),
        )
        .is_err()
    );

    let misplaced = SyntheticNavigationPackage::create_with_bad_placement(true);
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &misplaced.output,
        misplaced.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A]),
        Err(ForeverNavigationError::TilePlacementMismatch {
            tile: WRITTEN_A,
            expected: (1, 1),
            actual: (2, 1),
        })
    ));
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);

    let wrong_bounds = SyntheticNavigationPackage::create_with_bad_bounds(true);
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &wrong_bounds.output,
        wrong_bounds.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A]),
        Err(ForeverNavigationError::TileBoundsMismatch { tile, .. }) if tile == WRITTEN_A
    ));
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);

    let wrong_global_origin = SyntheticNavigationPackage::create_with_bad_global_origin(true);
    assert!(matches!(
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
            &wrong_global_origin.output,
            wrong_global_origin.expected,
            ForeverNavigationLimits::default(),
        ),
        Err(ForeverNavigationError::NonCanonicalGlobalParams)
    ));
}

#[test]
fn ancestor_slot_count_may_be_below_coverage_and_preflights_native_growth() {
    let package = SyntheticNavigationPackage::create();
    let verified = forever_navigation::package_v23::verify(&package.output, package.expected)
        .expect("verify package with ancestor-owned global slots");
    let global_bytes = fs::read(
        package
            .output
            .join(forever_navigation::mmap::global_path(SELECTED_MAP)),
    )
    .unwrap();
    let global = forever_navigation::mmap::GlobalView::parse(
        &global_bytes,
        SELECTED_MAP,
        forever_navigation::mmap::Limits::default(),
    )
    .unwrap();
    assert_eq!(verified.coverage().tiles.len(), 4);
    assert_eq!(global.params().max_tiles, 2);

    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A, WRITTEN_C, WRITTEN_B]),
        Err(ForeverNavigationError::LoadedTileLimit {
            actual: 3,
            maximum: 2
        })
    ));
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);
}

#[test]
fn locale_map_and_phase_mismatches_are_rejected_explicitly() {
    let package = SyntheticNavigationPackage::create();
    let mut wrong_locale = package.expected;
    wrong_locale.locale = forever_casc::Locale::EsEs;
    assert!(matches!(
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
            &package.output,
            wrong_locale,
            ForeverNavigationLimits::default(),
        ),
        Err(ForeverNavigationError::InvalidExpected { field: "locale" })
    ));

    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        worker.query_forever_navigation_corridor(query(
            Position::xyz(5.0, 5.0, 0.0),
            Position::xyz(25.0, 25.0, 0.0),
        )),
        Err(ForeverNavigationError::TileNotPrepared(WRITTEN_A))
    ));
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP + 1, [WRITTEN_A]),
        Err(ForeverNavigationError::WrongMap { .. })
    ));

    let mut phased = query(Position::xyz(5.0, 5.0, 0.0), Position::xyz(25.0, 25.0, 0.0));
    phased.phase_shift = PhaseShift::from_phases([17]);
    assert!(matches!(
        worker.query_forever_navigation_corridor(phased),
        Err(ForeverNavigationError::UnsupportedPhase)
    ));

    let mut visible_map = query(Position::xyz(5.0, 5.0, 0.0), Position::xyz(25.0, 25.0, 0.0));
    visible_map
        .phase_shift
        .add_visible_map_id_like_cpp(SELECTED_MAP + 1, 1);
    assert!(matches!(
        worker.query_forever_navigation_corridor(visible_map),
        Err(ForeverNavigationError::UnsupportedPhase)
    ));
}

#[test]
fn public_batch_and_instance_limits_are_enforced_before_growth() {
    let package = SyntheticNavigationPackage::create();
    let defaults = ForeverNavigationLimits::default();
    let limits = ForeverNavigationLimits::new(
        defaults.max_loaded_tiles(),
        defaults.max_loaded_bytes(),
        1,
        defaults.max_batch_bytes(),
        defaults.max_file_bytes(),
        1,
    )
    .unwrap();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        limits,
    )
    .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A, WRITTEN_B]),
        Err(ForeverNavigationError::BatchTooLarge {
            actual: 2,
            maximum: 1
        })
    ));

    worker
        .prepare_forever_navigation_instance(SELECTED_MAP, SELECTED_MAP, 9)
        .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_instance(SELECTED_MAP, SELECTED_MAP, 10),
        Err(ForeverNavigationError::InstanceQueryLimit {
            actual: 2,
            maximum: 1
        })
    ));
    assert_eq!(
        worker.forever_navigation_stats().unwrap().instance_queries,
        1
    );
    worker
        .unload_forever_navigation_instance(SELECTED_MAP, SELECTED_MAP, 9)
        .unwrap();
    worker
        .prepare_forever_navigation_instance(SELECTED_MAP, SELECTED_MAP, 10)
        .unwrap();
    assert_eq!(
        worker.forever_navigation_stats().unwrap().instance_queries,
        1
    );
    drop(worker);

    let package = SyntheticNavigationPackage::create();
    let verified = forever_navigation::package_v23::verify(&package.output, package.expected)
        .expect("verify synthetic byte budgets");
    let coverage = verified.coverage();
    let tile_bytes = coverage
        .tiles
        .iter()
        .find(|tile| tile.tile == WRITTEN_A)
        .expect("written synthetic tile")
        .bytes;
    let just_under_tile_budget = usize::try_from(coverage.global_bytes + tile_bytes - 1).unwrap();
    let global_bytes = usize::try_from(coverage.global_bytes).unwrap();
    let defaults = ForeverNavigationLimits::default();
    let limits = ForeverNavigationLimits::new(
        defaults.max_loaded_tiles(),
        just_under_tile_budget,
        defaults.max_tiles_per_batch(),
        defaults.max_batch_bytes(),
        defaults.max_file_bytes(),
        defaults.max_instance_queries(),
    )
    .unwrap();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        limits,
    )
    .unwrap();
    assert!(global_bytes < just_under_tile_budget);
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A]),
        Err(ForeverNavigationError::LoadedByteLimit { .. })
    ));
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);
    drop(worker);

    let package = SyntheticNavigationPackage::create();
    let verified = forever_navigation::package_v23::verify(&package.output, package.expected)
        .expect("verify synthetic batch-byte budget");
    let coverage = verified.coverage();
    let tile_bytes = coverage
        .tiles
        .iter()
        .find(|tile| tile.tile == WRITTEN_A)
        .expect("written synthetic tile")
        .bytes;
    let just_under_batch_budget = usize::try_from(coverage.global_bytes + tile_bytes - 1).unwrap();
    let limits = ForeverNavigationLimits::new(
        defaults.max_loaded_tiles(),
        defaults.max_loaded_bytes(),
        defaults.max_tiles_per_batch(),
        just_under_batch_budget,
        defaults.max_file_bytes(),
        defaults.max_instance_queries(),
    )
    .unwrap();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        limits,
    )
    .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A]),
        Err(ForeverNavigationError::Package(_))
    ));
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);
}

#[test]
fn missing_global_artifact_fails_worker_startup() {
    let package = SyntheticNavigationPackage::create();
    let global = package
        .output
        .join(forever_navigation::mmap::global_path(SELECTED_MAP));
    fs::remove_file(global).unwrap();

    assert!(matches!(
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
            &package.output,
            package.expected,
            ForeverNavigationLimits::default(),
        ),
        Err(ForeverNavigationError::Package(_))
    ));
}

#[test]
fn changed_package_is_reverified_before_any_new_tile_is_admitted() {
    let package = SyntheticNavigationPackage::create();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    fs::write(package.tile_path(WRITTEN_A), b"changed after admission").unwrap();

    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A]),
        Err(ForeverNavigationError::Package(_))
    ));
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);
}

#[test]
fn later_valid_package_must_match_the_admitted_inventory_exactly() {
    let package = SyntheticNavigationPackage::create();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    let replacement = SyntheticNavigationPackage::create_with_metadata_calls(10);
    package.replace_output_with(&replacement).unwrap();

    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A]),
        Err(ForeverNavigationError::PackageChanged)
    ));
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);
}

#[cfg(feature = "test-fixtures")]
#[test]
fn limits_preflight_and_failure_rollback_keep_the_native_manager_bounded() {
    let package = SyntheticNavigationPackage::create();
    let defaults = ForeverNavigationLimits::default();
    let limits = ForeverNavigationLimits::new(
        1,
        defaults.max_loaded_bytes(),
        defaults.max_tiles_per_batch(),
        defaults.max_batch_bytes(),
        defaults.max_file_bytes(),
        defaults.max_instance_queries(),
    )
    .unwrap();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        limits,
    )
    .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A, WRITTEN_B]),
        Err(ForeverNavigationError::LoadedTileLimit {
            actual: 2,
            maximum: 1
        })
    ));
    assert_eq!(worker.forever_navigation_stats().unwrap().loaded_tiles, 0);

    let package = SyntheticNavigationPackage::create();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    worker
        .set_forever_navigation_test_failure_plan(ForeverNavigationTestFailurePlan {
            fail_after_native_adds: 1,
            fail_rollback: false,
        })
        .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A, WRITTEN_B]),
        Err(ForeverNavigationError::TestInjectedNativeFailure)
    ));
    let rolled_back = worker.forever_navigation_stats().unwrap();
    assert_eq!(rolled_back.loaded_tiles, 0);
    assert!(!rolled_back.poisoned);

    let package = SyntheticNavigationPackage::create();
    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation(
        &package.output,
        package.expected,
        ForeverNavigationLimits::default(),
    )
    .unwrap();
    worker
        .set_forever_navigation_test_failure_plan(ForeverNavigationTestFailurePlan {
            fail_after_native_adds: 1,
            fail_rollback: true,
        })
        .unwrap();
    assert!(matches!(
        worker.prepare_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A, WRITTEN_B]),
        Err(ForeverNavigationError::RollbackFailed { .. })
    ));
    let poisoned = worker.forever_navigation_stats().unwrap();
    assert!(poisoned.poisoned);
    assert_eq!(poisoned.loaded_tiles, 1);
    assert!(matches!(
        worker.unload_forever_navigation_tiles(SELECTED_MAP, [WRITTEN_A]),
        Err(ForeverNavigationError::Poisoned)
    ));
}
