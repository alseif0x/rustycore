//! Instance scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
