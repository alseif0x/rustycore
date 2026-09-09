//! Movement scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn waypoint_path_store_loads_paths_nodes_and_normalizes_coords_like_cpp() {
    let (store, report) = WaypointPathStoreLikeCpp::from_rows_like_cpp(
        [
            WaypointPathRowLikeCpp {
                path_id: 10,
                move_type: 1,
                flags: 0x01,
            },
            WaypointPathRowLikeCpp {
                path_id: 11,
                move_type: 4,
                flags: 0,
            },
        ],
        [
            WaypointPathNodeRowLikeCpp {
                path_id: 10,
                node_id: 1,
                x: wow_core::Position::MAP_HALFSIZE_LIKE_CPP + 100.0,
                y: -(wow_core::Position::MAP_HALFSIZE_LIKE_CPP + 100.0),
                z: 25.0,
                orientation: Some(1.25),
                delay: 500,
            },
            WaypointPathNodeRowLikeCpp {
                path_id: 12,
                node_id: 1,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: None,
                delay: 0,
            },
        ],
    );

    assert_eq!(store.len(), 1);
    assert_eq!(report.path_rows, 2);
    assert_eq!(report.paths_loaded, 1);
    assert_eq!(report.skipped_invalid_move_type, 1);
    assert_eq!(report.node_rows, 2);
    assert_eq!(report.nodes_loaded, 1);
    assert_eq!(report.skipped_missing_path, 1);
    assert_eq!(report.backwards_too_short, 1);

    let path = store.get(10).expect("valid path retained");
    assert_eq!(path.move_type, wow_movement::WaypointMoveType::Run);
    assert!(path.follow_path_backwards_from_end_to_start);
    assert_eq!(path.nodes.len(), 1);
    let node = path.nodes[0];
    let limit = wow_core::Position::MAP_HALFSIZE_LIKE_CPP - 0.5;
    assert_eq!(node.id, 1);
    assert_eq!(node.position.x, limit);
    assert_eq!(node.position.y, -limit);
    assert_eq!(node.position.z, 25.0);
    assert_eq!(node.orientation, Some(1.25));
    assert_eq!(node.delay_ms, 500);
}
#[test]
fn waypoint_path_store_reports_empty_paths_and_clamped_delay_like_cpp() {
    let (store, report) = WaypointPathStoreLikeCpp::from_rows_like_cpp(
        [
            WaypointPathRowLikeCpp {
                path_id: 20,
                move_type: 0,
                flags: 0,
            },
            WaypointPathRowLikeCpp {
                path_id: 21,
                move_type: 3,
                flags: 0,
            },
        ],
        [WaypointPathNodeRowLikeCpp {
            path_id: 21,
            node_id: 7,
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: None,
            delay: u32::MAX,
        }],
    );

    assert_eq!(store.len(), 2);
    assert_eq!(report.empty_paths, 1);
    assert_eq!(report.clamped_delay, 1);
    assert_eq!(
        store.get(21).unwrap().move_type,
        wow_movement::WaypointMoveType::TakeOff
    );
    assert_eq!(store.get(21).unwrap().nodes[0].delay_ms, i32::MAX);
}
#[test]
fn game_event_active_set_insert_dedupe_order_remove_and_clear_like_cpp() {
    let mut active = GameEventActiveSetLikeCpp::new();

    assert!(active.add_active_event_like_cpp(7));
    assert!(active.add_active_event_like_cpp(2));
    assert!(!active.add_active_event_like_cpp(7));
    assert!(active.add_active_event_like_cpp(5));
    assert_eq!(
        active.active_event_ids_like_cpp().collect::<Vec<_>>(),
        vec![2, 5, 7]
    );

    assert!(active.remove_active_event_like_cpp(5));
    assert!(!active.remove_active_event_like_cpp(5));
    assert_eq!(
        active.active_event_ids_like_cpp().collect::<Vec<_>>(),
        vec![2, 7]
    );

    active.clear_active_events_like_cpp();
    assert_eq!(active.active_event_ids_like_cpp().count(), 0);
}
