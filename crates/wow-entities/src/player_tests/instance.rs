//! Instance scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn pending_bind_expires_once_at_sixty_seconds() {
    let mut player = Player::new(None, false);
    let pending = RepresentedPendingBind {
        map_id: 631,
        instance_id: 9001,
        completed_mask: 1,
        time_until_lock_ms: 60_000,
    };
    player.set_pending_bind_like_cpp(pending);
    assert!(player.advance_pending_bind_like_cpp(59_999).is_none());
    assert_eq!(
        player.pending_bind_like_cpp().unwrap().time_until_lock_ms,
        1
    );
    assert_eq!(
        player.advance_pending_bind_like_cpp(1).unwrap().instance_id,
        9001
    );
    assert!(player.advance_pending_bind_like_cpp(60_000).is_none());
    player.set_pending_bind_like_cpp(pending);
    player.take_pending_bind_like_cpp();
    assert!(player.advance_pending_bind_like_cpp(60_001).is_none());
}
#[test]
fn pending_bind_is_player_owned_and_replacement_starts_empty_like_cpp() {
    let pending_bind = RepresentedPendingBind {
        map_id: 631,
        instance_id: 9001,
        completed_mask: 0xA5,
        time_until_lock_ms: 60_000,
    };
    let mut first = Player::new(None, false);
    let second = Player::new(None, false);

    first.set_pending_bind_like_cpp(pending_bind);

    assert_eq!(first.pending_bind_like_cpp(), Some(pending_bind));
    assert!(first.has_pending_bind_like_cpp());
    assert_eq!(second.pending_bind_like_cpp(), None);
    assert_eq!(first.take_pending_bind_like_cpp(), Some(pending_bind));
    assert!(!first.has_pending_bind_like_cpp());

    let replacement = Player::new(None, false);
    assert_eq!(replacement.pending_bind_like_cpp(), None);
}

#[test]
fn player_power_index_resolver_configures_runtime_mapping_without_update_masks() {
    let mut player = Player::new(None, false);
    player.set_race_class_gender(1, CLASS_PALADIN, Gender::Male);
    player.set_power_index(PowerType::Focus, Some(4));
    player.clear_data_changes();

    player.configure_power_indices_for_class(&StubPowerResolver);

    assert_eq!(player.get_power_index(PowerType::Mana), Some(0));
    assert_eq!(player.get_power_index(PowerType::Energy), Some(3));
    assert_eq!(player.get_power_index(PowerType::ComboPoints), Some(9));
    assert_eq!(player.get_power_index(PowerType::Focus), None);
    assert_eq!(player.get_power_index(PowerType::AlternateMount), None);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(!player.player_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
}
