//! Original interaction branch and model-bound cases.
use super::*;

#[test]
fn gameobject_interaction_distance_uses_cpp_type_branches() {
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_CHEST as u8),
            Some(725),
            0.5
        ),
        7.25
    );
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_AREADAMAGE as u8),
            None,
            0.5
        ),
        0.0
    );
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_QUESTGIVER as u8),
            None,
            0.5
        ),
        5.5555553
    );
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_BINDER as u8),
            None,
            0.5
        ),
        10.0
    );
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_CHAIR as u8),
            None,
            0.5
        ),
        3.0
    );
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_FISHING_NODE as u8),
            None,
            0.5
        ),
        100.0
    );
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_FISHING_HOLE as u8),
            None,
            0.5
        ),
        20.0 + 0.5
    );
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_DOOR as u8),
            None,
            0.5
        ),
        5.0
    );
    assert_eq!(
        gameobject_interaction_distance(
            Some(GAMEOBJECT_TYPE_GUILD_BANK as u8),
            None,
            0.5
        ),
        10.0
    );
    assert_eq!(
        gameobject_interaction_distance(None, None, 0.5),
        5.0
    );
}

#[test]
fn gameobject_display_box_interaction_matches_cpp_contains_branch() {
    let bounds_min = [-2.0, -1.0, -0.5];
    let bounds_max = [2.0, 1.0, 0.5];
    let go_position = Position::ZERO;

    assert!(gameobject_display_box_contains(
        go_position,
        Position::xyz(6.9, 0.0, 0.0),
        bounds_min,
        bounds_max,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
        5.0,
    ));
    assert!(!gameobject_display_box_contains(
        go_position,
        Position::xyz(7.1, 0.0, 0.0),
        bounds_min,
        bounds_max,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
        5.0,
    ));
}
