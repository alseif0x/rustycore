// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete-reader, ownership, query and failure acceptance.
use super::*;
#[path = "../../../tests/support/forever.rs"]
mod fixture;
use fixture::{Fixture, ground};
mod numeric;

fn topology() -> [MapParent; 1] {
    [MapParent {
        map_id: 1,
        parent: None,
        cosmetic_parent: None,
    }]
}
fn open(f: &Fixture, options: Options, limits: Limits) -> Arc<ForeverTerrain> {
    ForeverTerrain::open(
        &f.root,
        Profile::declared_build(70205).unwrap(),
        &topology(),
        options,
        limits,
        StoreLimits::default(),
        SourceLimits::default(),
    )
    .unwrap()
}
fn disabled() -> Options {
    Options {
        enable_height: false,
        enable_line_of_sight: false,
        ..Options::default()
    }
}
fn key() -> TileKey {
    TileKey::new(1, 32, 32).unwrap()
}
fn budget() -> QueryBudget {
    QueryBudget::new(QueryLimits::default()).unwrap()
}
fn height(t: &ForeverTerrain, z: f32) -> Result<f32, Error> {
    t.static_height(1, [-1., -1., z], true, 50., &mut budget())
}

#[test]
fn profile_and_grid_admission() {
    assert!(Profile::declared_build(70170).is_ok());
    assert_eq!(Profile::declared_build(51943), Err(Error::Profile));
    assert!(TileKey::new(1, 64, 0).is_err());
    assert_eq!(TileKey::for_position(1, -1., -1.).unwrap().x(), 32);
    assert_eq!(TileKey::for_position(1, 1., 1.).unwrap().x(), 31);
    assert!(TileKey::for_position(1, f32::NAN, 0.).is_err());
    assert!(TileKey::for_position(1, 17067., 0.).is_err());
    assert!(TileKey::for_position(1, -17067., 0.).is_err());
}

#[test]
fn declared_70170_and_70205_each_require_their_own_complete_maps_tag() {
    for build in [70170, 70205] {
        let f = Fixture::new();
        let owner = ForeverTerrain::open(
            &f.root,
            Profile::declared_build(build).unwrap(),
            &topology(),
            disabled(),
            Limits::default(),
            StoreLimits::default(),
            SourceLimits::default(),
        )
        .unwrap();
        let bytes = ground(build, 7., 1, false);
        let _lease = owner.prepare(key(), Ground::Complete(&bytes)).unwrap();
        assert_eq!(owner.profile().build(), build);
        assert_eq!(height(&owner, 8.), Ok(7.));
    }
}

#[test]
fn failed_new_tile_keeps_existing_ground_and_lease_usable() {
    let f = Fixture::new();
    let owner = open(&f, disabled(), Limits::default());
    let bytes = ground(70205, 7., 1, false);
    let _lease = owner.prepare(key(), Ground::Complete(&bytes)).unwrap();
    let before = owner.stats().unwrap();
    assert_eq!(
        owner
            .prepare(
                TileKey::new(1, 31, 31).unwrap(),
                Ground::Complete(&bytes[..bytes.len() - 1])
            )
            .unwrap_err(),
        Error::GroundFormat
    );
    assert_eq!(owner.stats().unwrap(), before);
    assert_eq!(height(&owner, 8.), Ok(7.));
}
#[test]
fn all_ground_encodings_use_actual_grid_reader() {
    for encoding in [0, 1, 2, 4] {
        let f = Fixture::new();
        let owner = open(&f, disabled(), Limits::default());
        let bytes = ground(70205, 7., encoding, false);
        let _lease = owner.prepare(key(), Ground::Complete(&bytes)).unwrap();
        assert_eq!(owner.grid_height(1, -1., -1.), Ok(7.));
        assert_eq!(height(&owner, 8.), Ok(7.));
        assert_eq!(height(&owner, 6.), Ok(NO_HEIGHT));
        assert_eq!(owner.stats().unwrap().source_reads, 0);
        assert!(owner.stats().unwrap().admitted_peak_bytes <= Limits::default().max_terrain_bytes);
    }
}
#[test]
fn holes_and_explicit_absence_have_distinct_native_sentinels() {
    let f = Fixture::new();
    let owner = open(&f, disabled(), Limits::default());
    let bytes = ground(70205, 7., 0, true);
    let _a = owner.prepare(key(), Ground::Complete(&bytes)).unwrap();
    let _b = owner
        .prepare(TileKey::new(1, 31, 31).unwrap(), Ground::Absent)
        .unwrap();
    assert_eq!(
        owner.grid_height(1, -1., -1.),
        Ok(crate::grid_map::INVALID_HEIGHT)
    );
    assert_eq!(owner.grid_height(1, 1., 1.), Ok(NO_HEIGHT));
}
#[test]
fn complete_container_build_and_tail_are_required() {
    let f = Fixture::new();
    let owner = open(&f, disabled(), Limits::default());
    let valid = ground(70205, 7., 1, false);
    let mut trailing = valid.clone();
    trailing.push(0);
    for bytes in [
        ground(70170, 7., 1, false),
        valid[..valid.len() - 1].to_vec(),
        trailing,
        ground(70205, f32::NAN, 1, false),
    ] {
        assert_eq!(
            owner.prepare(key(), Ground::Complete(&bytes)).unwrap_err(),
            Error::GroundFormat
        );
        assert_eq!(owner.stats().unwrap().tiles, 0);
    }
    assert!(owner.prepare(key(), Ground::Complete(&valid)).is_ok());
}
#[test]
fn leases_share_geometry_and_defer_unload_until_cleanup() {
    let f = Fixture::new();
    f.plane();
    let owner = open(&f, Options::default(), Limits::default());
    let bytes = ground(70205, -2., 1, false);
    let a = owner.prepare(key(), Ground::Complete(&bytes)).unwrap();
    let reads = owner.stats().unwrap().source_reads;
    let b = owner.prepare(key(), Ground::Complete(&bytes)).unwrap();
    assert_eq!(owner.stats().unwrap().source_reads, reads);
    assert_eq!(owner.stats().unwrap().leases, 2);
    assert_eq!(owner.stats().unwrap().models, 1);
    assert_eq!(owner.cleanup_unreferenced(), Ok(0));
    drop(a);
    assert_eq!(owner.cleanup_unreferenced(), Ok(0));
    drop(b);
    assert_eq!(height(&owner, 5.), Ok(0.));
    assert_eq!(owner.cleanup_unreferenced(), Ok(1));
    assert_eq!(owner.stats().unwrap().models, 0);
    assert_eq!(height(&owner, 5.), Err(Error::Unprepared));
    let _again = owner.prepare(key(), Ground::Complete(&bytes)).unwrap();
    assert_eq!(height(&owner, 5.), Ok(0.));
}
#[test]
fn composed_collision_queries_do_no_source_reads() {
    let f = Fixture::new();
    f.plane();
    let owner = open(&f, Options::default(), Limits::default());
    let bytes = ground(70205, -2., 1, false);
    let _lease = owner.prepare(key(), Ground::Complete(&bytes)).unwrap();
    let before = owner.stats().unwrap();
    let a = [-1., -1., 5.];
    let b = [-1., -1., -5.];
    assert_eq!(
        owner.line_of_sight(1, a, b, false, &mut budget()),
        Ok(false)
    );
    assert_eq!(height(&owner, 5.), Ok(0.));
    assert_eq!(
        owner.static_height(1, a, false, 50., &mut budget()),
        Ok(-2.)
    );
    assert_eq!(owner.static_height(1, a, true, 1., &mut budget()), Ok(-2.));
    let hit = owner.object_hit(1, a, b, -1., &mut budget()).unwrap();
    assert!(hit.hit());
    assert_eq!(hit.position(), [-1., -1., 1.]);
    assert_eq!(owner.stats().unwrap(), before);
}
#[test]
fn missing_coverage_including_same_point_is_an_error() {
    let f = Fixture::new();
    f.plane();
    let owner = open(&f, Options::default(), Limits::default());
    let a = [-1., -1., 5.];
    let b = [1., 1., 5.];
    assert_eq!(
        owner.line_of_sight(1, a, a, false, &mut budget()),
        Err(Error::Unprepared)
    );
    let _lease = owner.prepare(key(), Ground::Absent).unwrap();
    assert_eq!(
        owner.line_of_sight(1, a, b, false, &mut budget()),
        Err(Error::Unprepared)
    );
    assert!(matches!(
        owner.object_hit(1, a, b, 0., &mut budget()),
        Err(Error::Unprepared)
    ));
}
#[test]
fn disabled_options_still_check_profile_map_positions_and_poison() {
    let f = Fixture::new();
    let owner = open(&f, disabled(), Limits::default());
    assert_eq!(
        owner.line_of_sight(1, [0.; 3], [0.; 3], false, &mut budget()),
        Ok(true)
    );
    assert_eq!(
        owner.line_of_sight(9, [0.; 3], [0.; 3], false, &mut budget()),
        Err(Error::UnknownMap)
    );
    assert_eq!(
        owner.line_of_sight(1, [f32::NAN; 3], [0.; 3], false, &mut budget()),
        Err(Error::NonFinite)
    );
    assert_eq!(height(&owner, 5.), Err(Error::Unprepared));
    let _a = owner.prepare(key(), Ground::Absent).unwrap();
    assert_eq!(
        owner.prepare(key(), Ground::Complete(&[])).unwrap_err(),
        Error::InputChanged
    );
    assert_eq!(
        owner.line_of_sight(1, [0.; 3], [0.; 3], false, &mut budget()),
        Err(Error::InputChanged)
    );
    assert_eq!(owner.cleanup_unreferenced(), Err(Error::InputChanged));
}
#[test]
fn different_cached_ground_bytes_poison_the_owner() {
    let f = Fixture::new();
    let owner = open(&f, disabled(), Limits::default());
    let a = ground(70205, 7., 1, false);
    let b = ground(70205, 8., 1, false);
    let _lease = owner.prepare(key(), Ground::Complete(&a)).unwrap();
    assert_eq!(
        owner.prepare(key(), Ground::Complete(&b)).unwrap_err(),
        Error::InputChanged
    );
    assert_eq!(height(&owner, 9.), Err(Error::InputChanged));
    assert!(owner.stats().unwrap().input_changed);
}
#[test]
fn collision_mutation_stays_poisoned_after_checkpoint() {
    let f = Fixture::new();
    f.plane();
    let owner = open(&f, Options::default(), Limits::default());
    let _lease = owner.prepare(key(), Ground::Absent).unwrap();
    std::fs::write(f.root.join("plane.vmo"), b"changed synthetic source").unwrap();
    assert!(owner.verify_source().is_err());
    assert_eq!(height(&owner, 5.), Err(Error::InputChanged));
    assert!(owner.stats().unwrap().input_changed);
}
#[test]
fn lease_and_registry_limits_preserve_previous_cache() {
    for limits in [
        Limits {
            max_leases: 1,
            ..Limits::default()
        },
        Limits {
            max_tiles: 1,
            ..Limits::default()
        },
    ] {
        let f = Fixture::new();
        let owner = open(&f, disabled(), limits);
        let _a = owner.prepare(key(), Ground::Absent).unwrap();
        assert_eq!(
            owner
                .prepare(TileKey::new(1, 31, 31).unwrap(), Ground::Absent)
                .unwrap_err(),
            Error::Limits
        );
        assert_eq!(owner.stats().unwrap().tiles, 1);
        assert_eq!(owner.stats().unwrap().leases, 1);
        assert_eq!(height(&owner, 5.), Ok(NO_HEIGHT));
    }
}
#[test]
fn memory_admission_accounts_registry_overlap_and_parse_allowance() {
    let f = Fixture::new();
    let owner = open(
        &f,
        disabled(),
        Limits {
            max_terrain_bytes: size_of::<Tile>() * 2,
            ..Limits::default()
        },
    );
    let _a = owner.prepare(key(), Ground::Absent).unwrap();
    assert_eq!(
        owner
            .prepare(TileKey::new(1, 31, 31).unwrap(), Ground::Absent)
            .unwrap_err(),
        Error::Limits
    );
    let bytes = ground(70205, 7., 1, false);
    let small = open(
        &f,
        disabled(),
        Limits {
            max_terrain_bytes: GRID_PARSE_ALLOWANCE,
            ..Limits::default()
        },
    );
    assert_eq!(
        small.prepare(key(), Ground::Complete(&bytes)).unwrap_err(),
        Error::Limits
    );
    assert_eq!(small.stats().unwrap().tiles, 0);
}
#[test]
fn missing_collision_input_cannot_commit_ground_or_consumer() {
    let f = Fixture::new();
    let owner = open(&f, Options::default(), Limits::default());
    let bytes = ground(70205, 7., 1, false);
    assert!(owner.prepare(key(), Ground::Complete(&bytes)).is_err());
    assert_eq!(owner.stats().unwrap().tiles, 0);
    assert_eq!(owner.stats().unwrap().leases, 0);
    assert_eq!(owner.stats().unwrap().models, 0);
}
#[test]
fn shared_budget_exhaustion_is_not_clear_or_no_height() {
    let f = Fixture::new();
    f.plane();
    let owner = open(&f, Options::default(), Limits::default());
    let _lease = owner.prepare(key(), Ground::Absent).unwrap();
    let mut exhausted = QueryBudget::new(QueryLimits {
        max_node_visits: 0,
        ..QueryLimits::default()
    })
    .unwrap();
    assert!(
        owner
            .static_height(1, [-1., -1., 5.], true, 50., &mut exhausted)
            .is_err()
    );
    assert!(
        owner
            .line_of_sight(1, [-1., -1., 5.], [-1., -1., -5.], false, &mut exhausted)
            .is_err()
    );
}
#[test]
fn leases_can_cross_threads_and_active_tiles_cannot_be_evicted() {
    let f = Fixture::new();
    let owner = open(&f, disabled(), Limits::default());
    let lease = owner.prepare(key(), Ground::Absent).unwrap();
    let shared = Arc::clone(&owner);
    std::thread::spawn(move || {
        assert_eq!(shared.cleanup_unreferenced(), Ok(0));
        drop(lease);
    })
    .join()
    .unwrap();
    assert_eq!(owner.cleanup_unreferenced(), Ok(1));
}

#[test]
fn cleanup_compacts_mixed_live_tiles_without_breaking_sorted_lookup() {
    let f = Fixture::new();
    let owner = open(&f, disabled(), Limits::default());
    let mut leases = (0..64)
        .map(|x| {
            Some(
                owner
                    .prepare(TileKey::new(1, x, 32).unwrap(), Ground::Absent)
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();
    for i in (0..64).step_by(2) {
        leases[i].take();
    }
    assert_eq!(owner.cleanup_unreferenced(), Ok(32));
    for i in (1..64).step_by(2) {
        let temporary = owner
            .prepare(TileKey::new(1, i, 32).unwrap(), Ground::Absent)
            .unwrap();
        assert_eq!(owner.stats().unwrap().tiles, 32);
        assert_eq!(owner.stats().unwrap().leases, 33);
        drop(temporary);
    }
    drop(leases);
    assert_eq!(owner.cleanup_unreferenced(), Ok(32));
    assert_eq!(owner.stats().unwrap().leases, 0);
}
