use super::*;

#[test]
fn default_character_power1_seeds_energy_classes_like_cpp() {
    assert_eq!(
        default_character_power1_like_cpp(4, 0),
        100,
        "C++ level-1 rogues enter with full base Energy, not zeroed mana"
    );
    assert_eq!(default_character_power1_like_cpp(5, 160), 160);
    assert_eq!(default_character_power1_like_cpp(1, 0), 0);
}

#[test]
fn restored_saved_health_preserves_dead_zero_like_cpp() {
    assert_eq!(restored_saved_health_like_cpp(Some(0), 110), 0);
}

#[test]
fn restored_saved_health_clamps_to_recomputed_max_like_cpp() {
    assert_eq!(restored_saved_health_like_cpp(Some(500), 110), 110);
    assert_eq!(restored_saved_health_like_cpp(Some(77), 110), 77);
}

#[test]
fn start_zones_are_valid() {
    // Rust table sanity only; this enumeration is not proof of C++ parity.
    for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
        let zone = start_zone(race);
        assert!(zone > 0, "Race {race} has invalid zone");
    }
}
