//! World-object visibility and location model regression scenarios.
//!
//! Separated from the world_object.rs root under #648.

use std::cell::{Cell, RefCell};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
struct LosSnapshot {
    from: LineOfSightEndpoint,
    to: LineOfSightEndpoint,
    has_target: bool,
}

#[derive(Debug, Clone)]
struct TestEnvironment {
    map_id: u32,
    instance_id: u32,
    visibility_range: f32,
    visibility_override: Option<f32>,
    creature_sight_distance: Option<f32>,
    cinematic: bool,
    los: bool,
    los_calls: Cell<usize>,
    last_los: RefCell<Option<LosSnapshot>>,
    height: f32,
    floor: f32,
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self {
            map_id: 571,
            instance_id: 1,
            visibility_range: DEFAULT_VISIBILITY_DISTANCE,
            visibility_override: None,
            creature_sight_distance: None,
            cinematic: false,
            los: true,
            los_calls: Cell::new(0),
            last_los: RefCell::new(None),
            height: INVALID_HEIGHT,
            floor: INVALID_HEIGHT,
        }
    }
}

impl WorldObjectEnvironment for TestEnvironment {
    fn map_id(&self) -> u32 {
        self.map_id
    }

    fn instance_id(&self) -> u32 {
        self.instance_id
    }

    fn visibility_range(&self) -> f32 {
        self.visibility_range
    }

    fn visibility_override(&self, _object: &WorldObject) -> Option<f32> {
        self.visibility_override
    }

    fn creature_sight_distance(&self, _object: &WorldObject) -> Option<f32> {
        self.creature_sight_distance
    }

    fn player_on_cinematic(&self, _object: &WorldObject) -> bool {
        self.cinematic
    }

    fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool {
        self.los_calls.set(self.los_calls.get() + 1);
        *self.last_los.borrow_mut() = Some(LosSnapshot {
            from: query.from,
            to: query.to,
            has_target: query.target.is_some(),
        });
        self.los
    }

    fn map_height(
        &self,
        _object: &WorldObject,
        _x: f32,
        _y: f32,
        z: f32,
        _query: WorldObjectHeightQuery,
    ) -> f32 {
        if (z - (10.0 + Z_OFFSET_FIND_HEIGHT)).abs() < 0.001 {
            self.height
        } else {
            INVALID_HEIGHT
        }
    }

    fn floor_z(&self, _object: &WorldObject, position: Position, _max_search_dist: f32) -> f32 {
        if (position.z - (10.0 + Z_OFFSET_FIND_HEIGHT)).abs() < 0.001 {
            self.floor
        } else {
            INVALID_HEIGHT
        }
    }
}

fn assert_position_close(actual: Position, expected: Position) {
    assert!(
        (actual.x - expected.x).abs() < 0.0001,
        "x: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.y - expected.y).abs() < 0.0001,
        "y: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.z - expected.z).abs() < 0.0001,
        "z: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.orientation - expected.orientation).abs() < 0.0001,
        "orientation: {actual:?} != {expected:?}"
    );
}

fn prepare_in_world(object: &mut WorldObject, position: Position) {
    object.set_map(571, 1).unwrap();
    object.object_mut().add_to_world();
    object.relocate(position);
}

fn last_los(environment: &TestEnvironment) -> LosSnapshot {
    environment
        .last_los
        .borrow()
        .expect("LOS query should be captured")
}

mod scenarios_1;
