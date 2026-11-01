// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: public production-library composition, synthetic assets.
#![cfg(all(feature = "forever-terrain", target_os = "linux"))]
#[path = "../../wow-map/tests/support/forever.rs"]
mod fixture;
use fixture::{Fixture, ground};
use wow_entities::WorldObjectHeightQuery;
use wow_map::terrain::forever::{
    Error, ForeverTerrain, Ground, Limits, MapParent, Options, Profile, QueryBudget, QueryLimits,
    SourceLimits, StoreLimits, TileKey,
};
use wow_world::terrain::LiveForeverTerrain;

fn live(f: &Fixture) -> LiveForeverTerrain {
    LiveForeverTerrain::new(
        ForeverTerrain::open(
            &f.root,
            Profile::declared_build(70205).unwrap(),
            &[MapParent {
                map_id: 1,
                parent: None,
                cosmetic_parent: None,
            }],
            Options::default(),
            Limits::default(),
            StoreLimits::default(),
            SourceLimits::default(),
        )
        .unwrap(),
    )
}
fn budget() -> QueryBudget {
    QueryBudget::new(QueryLimits::default()).unwrap()
}
#[test]
fn production_facades_share_one_geometry_owner_and_consumer_lifetime() {
    let f = Fixture::new();
    f.plane();
    let first = live(&f);
    let second = first.clone();
    let bytes = ground(70205, -2., 1, false);
    let key = TileKey::for_position(1, -1., -1.).unwrap();
    let a = first.prepare(key, Ground::Complete(&bytes)).unwrap();
    let b = second.prepare(key, Ground::Complete(&bytes)).unwrap();
    let before = first.stats().unwrap();
    assert_eq!(before.tiles, 1);
    assert_eq!(before.models, 1);
    assert_eq!(before.leases, 2);
    assert_eq!(second.grid_height(1, -1., -1.), Ok(-2.));
    assert_eq!(
        second.static_height(
            1,
            [-1., -1., 5.],
            WorldObjectHeightQuery::default(),
            &mut budget()
        ),
        Ok(0.)
    );
    assert_eq!(
        second.static_height(
            1,
            [-1., -1., 5.],
            WorldObjectHeightQuery {
                vmap: false,
                distance_to_search: 50.
            },
            &mut budget()
        ),
        Ok(-2.)
    );
    let hit = second
        .static_object_hit(1, [-1., -1., 5.], [-1., -1., -5.], 0., &mut budget())
        .unwrap();
    assert!(hit.hit());
    assert_eq!(hit.position(), [-1., -1., 0.]);
    assert_eq!(first.stats().unwrap(), before);
    drop(a);
    assert_eq!(first.cleanup_unreferenced(), Ok(0));
    drop(b);
    assert_eq!(second.cleanup_unreferenced(), Ok(1));
    assert_eq!(
        first.static_height(
            1,
            [-1., -1., 5.],
            WorldObjectHeightQuery::default(),
            &mut budget()
        ),
        Err(Error::Unprepared)
    );
}
#[test]
fn production_facade_propagates_errors_and_source_checkpoint() {
    let f = Fixture::new();
    f.plane();
    let live = live(&f);
    let key = TileKey::for_position(1, -1., -1.).unwrap();
    assert_eq!(
        live.prepare(key, Ground::Complete(b"partial")).unwrap_err(),
        Error::GroundFormat
    );
    assert_eq!(live.stats().unwrap().tiles, 0);
    let _lease = live.prepare(key, Ground::Absent).unwrap();
    std::fs::write(f.root.join("plane.vmo"), b"changed synthetic bytes").unwrap();
    assert!(live.verify_source().is_err());
    assert_eq!(live.grid_height(1, -1., -1.), Err(Error::InputChanged));
}

#[test]
fn production_los_forwards_resolved_entity_endpoints_once() {
    use wow_constants::{TypeId, TypeMask};
    use wow_core::Position;
    use wow_entities::{LineOfSightOptions, LineOfSightQuery, WorldObject};
    let f = Fixture::new();
    f.plane();
    let live = live(&f);
    let _lease = live
        .prepare(TileKey::for_position(1, -1., -1.).unwrap(), Ground::Absent)
        .unwrap();
    let mut source = WorldObject::new(true, TypeId::Object, TypeMask::OBJECT);
    source.relocate(Position::new(-1., -1., 5., 0.));
    let query = LineOfSightQuery::to_position_like_cpp(
        &source,
        Position::new(-1., -1., -5., 0.),
        LineOfSightOptions {
            check_dynamic: false,
        },
    );
    assert_eq!(
        live.static_line_of_sight(1, query, false, &mut budget()),
        Ok(false)
    );
    let query = LineOfSightQuery::to_position_like_cpp(
        &source,
        Position::new(1., 1., -5., 0.),
        LineOfSightOptions {
            check_dynamic: false,
        },
    );
    assert_eq!(
        live.static_line_of_sight(1, query, false, &mut budget()),
        Err(Error::Unprepared)
    );
}
