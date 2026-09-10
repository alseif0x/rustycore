//! Waypoint generator regressions.
//!
//! Separated from waypoint.rs under #685.

use super::*;

fn snapshot() -> WaypointUnitSnapshot {
    WaypointUnitSnapshot {
        owner_alive: true,
        owner_unit_state: 0,
        movement_prevented_by_casting: false,
        move_spline_finalized: true,
        owner_is_on_transport: false,
        owner_is_formation_leader: false,
        formation_leader_move_allowed: true,
        owner_orientation: 0.0,
        owner_position: Position::new(0.0, 0.0, 0.0, 0.0),
        ai_enabled: true,
    }
}

fn path() -> WaypointPath {
    let mut path = WaypointPath::new(
        77,
        vec![
            WaypointNode::new(10, 1.0, 0.0, 0.0),
            WaypointNode::new(20, 2.0, 0.0, 0.0).with_delay(500),
            WaypointNode::new(30, 3.0, 0.0, 0.0).with_orientation(1.25),
        ],
    );
    path.move_type = WaypointMoveType::Run;
    path
}

mod scenarios;
