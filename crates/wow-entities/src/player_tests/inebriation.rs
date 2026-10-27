use super::*;

const DRUNK_DETECTION: usize = 6;

#[test]
fn set_drunk_value_uses_native_thresholds_and_clamps() {
    let mut player = Player::new(None, false);
    for (value, state) in [(0, 0), (1, 1), (49, 1), (50, 2), (89, 2), (90, 3), (100, 3)] {
        assert_eq!(Player::drunken_state_by_value_like_cpp(value), state);
    }

    assert!(player.set_drunk_value_like_cpp(150, 0, false));
    assert_eq!(player.inebriation_like_cpp(), 100);
}

#[test]
fn set_drunk_value_resets_timer_only_when_not_sobering() {
    let mut player = Player::new(None, false);
    player.drunk_timer = 900;
    assert!(!player.set_drunk_value_like_cpp(0, 0, false));
    assert_eq!(player.drunk_timer, 0);

    player.drunk_timer = 700;
    player.set_drunk_value_like_cpp(70, 0, false);
    assert_eq!(player.drunk_timer, 0);

    player.drunk_timer = 500;
    player.set_drunk_value_like_cpp(20, 0, false);
    assert_eq!(player.drunk_timer, 500);
}

#[test]
fn fake_inebriation_overrides_value_and_zero_sum_preserves_detection() {
    let mut player = Player::new(None, false);
    player.set_drunk_value_like_cpp(0, 65, true);
    assert!(player.unit().invisibility_detect_flagged_like_cpp(DRUNK_DETECTION));
    assert_eq!(player.unit().invisibility_detect_value_like_cpp(DRUNK_DETECTION), 65);

    player.set_drunk_value_like_cpp(0, 0, true);
    assert!(player.unit().invisibility_detect_flagged_like_cpp(DRUNK_DETECTION));
    assert_eq!(player.unit().invisibility_detect_value_like_cpp(DRUNK_DETECTION), 65);
}

#[test]
fn clearing_drunk_detection_preserves_stored_value_and_updates_only_native_visibility() {
    let mut player = Player::new(None, false);
    player.unit_mut().set_invisibility_detect_like_cpp(DRUNK_DETECTION, 23);
    player.drunk_timer = 300;
    let old_health_revision = player.unit().health_state_revision_like_cpp();
    let old_auras = player.unit().subsystems().auras.runtime_applications_like_cpp().len();
    player.set_drunk_value_like_cpp(0, 0, false);
    assert!(!player.unit().invisibility_detect_flagged_like_cpp(DRUNK_DETECTION));
    assert_eq!(player.unit().invisibility_detect_value_like_cpp(DRUNK_DETECTION), 23);
    assert_eq!(player.unit().health_state_revision_like_cpp(), old_health_revision);
    assert_eq!(player.unit().subsystems().auras.runtime_applications_like_cpp().len(), old_auras);
    assert_eq!(player.drunk_timer, 0);
    assert!(!player.unit().world().object().is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED));

    player.unit_mut().world_mut().object_mut().add_to_world();
    player.set_drunk_value_like_cpp(1, 0, false);
    assert!(player.unit().world().object().is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED));
}

#[test]
fn player_update_fields_queue_only_changed_in_world_values() {
    let mut player = Player::new(None, false);
    player.set_inebriation_like_cpp(1);
    assert!(player
        .player_data_changes_mask()
        .is_set(crate::PLAYER_DATA_INEBRIATION_BIT));
    assert!(!player.unit().world().object().is_object_updated());

    player.unit_mut().world_mut().object_mut().add_to_world();
    player.clear_data_changes();
    player.set_inebriation_like_cpp(50);
    assert!(player.unit().world().object().is_object_updated());

    player.clear_data_changes();
    player.set_inebriation_like_cpp(50);
    assert!(!player.unit().world().object().is_object_updated());
    assert!(!player.player_data_changes_mask().is_any_set());
}

#[test]
fn self_res_spell_dynamic_field_uses_the_native_player_update_queue() {
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.clear_data_changes();

    player.add_self_res_spell(21169);

    assert!(player.unit().world().object().is_object_updated());
    assert!(player
        .active_player_data_changes_mask()
        .is_set(crate::ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT));
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        Some(vec![1])
    );
}
