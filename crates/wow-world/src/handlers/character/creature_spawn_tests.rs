use super::*;

#[test]
fn sql_creature_template_speed_defaults_match_cpp_check_creature_template() {
    assert_eq!(normalize_creature_template_speed_walk_like_cpp(0.0), 1.0);
    assert_eq!(normalize_creature_template_speed_run_like_cpp(0.0), 1.14286);
    assert_eq!(normalize_creature_template_speed_walk_like_cpp(0.75), 0.75);
    assert_eq!(normalize_creature_template_speed_run_like_cpp(2.0), 2.0);
}

#[test]
fn creature_spawn_difficulties_filter_matches_spawn_mode_like_cpp() {
    assert!(spawn_difficulties_contains_spawn_mode_like_cpp("0", 0));
    assert!(spawn_difficulties_contains_spawn_mode_like_cpp("0,1", 1));
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("1", 0));
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("", 0));
}

#[test]
fn creature_spawn_difficulties_invalid_token_maps_to_none_like_cpp() {
    assert!(
        spawn_difficulties_contains_spawn_mode_like_cpp("bad", 0),
        "C++ ObjectMgr::ParseSpawnDifficulties maps invalid tokens to DIFFICULTY_NONE"
    );
    assert!(!spawn_difficulties_contains_spawn_mode_like_cpp("bad", 1));
}
