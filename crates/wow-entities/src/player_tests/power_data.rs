use super::*;
use wow_constants::{Gender, PowerType};
use wow_data::player::power::{ClassPowerIndexRecord, Db2PlayerPowerIndexResolver};

const CLASS_PALADIN: u8 = 2;

#[test]
fn db2_player_power_index_resolver_feeds_player_and_entity_ignores_out_of_range_indices() {
    let resolver = Db2PlayerPowerIndexResolver::from_records([
        ClassPowerIndexRecord::new(CLASS_PALADIN, PowerType::Mana, 0),
        ClassPowerIndexRecord::new(CLASS_PALADIN, PowerType::ComboPoints, 9),
        ClassPowerIndexRecord::new(
            CLASS_PALADIN,
            PowerType::AlternateMount,
            MAX_POWERS_PER_CLASS,
        ),
    ]);
    let mut player = Player::new(None, false);
    player.set_race_class_gender(1, CLASS_PALADIN, Gender::Male);
    player.clear_data_changes();

    player.configure_power_indices_for_class(&resolver);

    assert_eq!(player.get_power_index(PowerType::Mana), Some(0));
    assert_eq!(player.get_power_index(PowerType::ComboPoints), Some(9));
    assert_eq!(player.get_power_index(PowerType::AlternateMount), None);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
}
