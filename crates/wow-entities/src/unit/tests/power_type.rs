//! Raw target PowerType bytes applied to canonical entity power storage.
use super::*;
use crate::{Creature, Player, PlayerPowerIndexResolver, represented_power_type_from_u8_like_cpp};
use std::cell::RefCell;

struct Resolver {
    calls: RefCell<Vec<u8>>,
}

impl PlayerPowerIndexResolver for Resolver {
    fn power_index_by_class(&self, power: PowerType, class_id: u8) -> Option<usize> {
        assert_eq!(class_id, 4);
        self.calls.borrow_mut().push(power as u8);
        match power {
            PowerType::Mana => Some(0),
            PowerType::ComboPoints => Some(2),
            PowerType::BurningEmbers => Some(3),
            _ => None,
        }
    }
}

#[test]
fn power_raw_four_and_fourteen_apply_to_distinct_player_class_slots() {
    let mut player = Player::new(None, false);
    player.set_race_class_gender(1, 4, Gender::Male);
    let resolver = Resolver {
        calls: RefCell::new(Vec::new()),
    };
    player.configure_power_indices_for_class(&resolver);
    assert_eq!(*resolver.calls.borrow(), (0..26).collect::<Vec<u8>>());
    let combo = represented_power_type_from_u8_like_cpp(4).unwrap();
    let embers = represented_power_type_from_u8_like_cpp(14).unwrap();
    assert_eq!(combo, PowerType::ComboPoints);
    assert_eq!(embers, PowerType::BurningEmbers);
    player.unit_mut().set_max_power(combo, 5);
    player.unit_mut().set_power(combo, 3);
    player.unit_mut().set_max_power(embers, 100);
    player.unit_mut().set_power(embers, 17);
    assert_eq!(player.get_power_index(PowerType::ComboPoints), Some(2));
    assert_eq!(player.get_power_index(PowerType::BurningEmbers), Some(3));
    assert_eq!(player.get_power(PowerType::ComboPoints), 3);
    assert_eq!(player.get_power(PowerType::BurningEmbers), 17);
    assert_eq!(player.unit().data().power[2..4], [3, 17]);
}

#[test]
fn power_display_decoder_admits_all_twenty_six_kinds_and_rejects_sentinels() {
    for raw in 0..26 {
        let kind = represented_power_type_from_u8_like_cpp(raw).unwrap();
        assert_eq!(kind as u8, raw);
        let mut unit = Unit::new(false);
        unit.set_display_power(kind);
        assert_eq!(unit.data().display_power, raw);
    }
    for (raw, expected) in [
        (20, PowerType::RuneBlood),
        (21, PowerType::RuneFrost),
        (22, PowerType::RuneUnholy),
        (23, PowerType::AlternateQuest),
        (24, PowerType::AlternateEncounter),
        (25, PowerType::AlternateMount),
    ] {
        assert_eq!(represented_power_type_from_u8_like_cpp(raw), Some(expected));
    }
    for raw in [26, 127, 254, 255] {
        assert_eq!(represented_power_type_from_u8_like_cpp(raw), None);
    }
}

#[test]
fn power_creature_raw_decoder_preserves_combo_slot_and_invalid_mana_fallback() {
    let mut creature = Creature::new(false);
    creature.set_power_type(PowerType::Energy);
    let combo = PowerType::from_u8_like_cpp(4).unwrap();
    creature.unit_mut().set_max_power(combo, 5);
    creature.unit_mut().set_power(combo, 4);
    assert_eq!(creature.get_power_index(PowerType::ComboPoints), Some(2));
    assert_eq!(creature.unit().get_power(PowerType::ComboPoints), 4);
    creature
        .unit_mut()
        .set_display_power(PowerType::BurningEmbers);
    assert_eq!(creature.unit().data().display_power, 14);
    assert_eq!(creature.power_type(), PowerType::BurningEmbers);
    creature
        .unit_mut()
        .set_display_power(PowerType::ComboPoints);
    assert_eq!(creature.unit().data().display_power, 4);
    assert_eq!(creature.power_type(), PowerType::ComboPoints);
    creature.unit_mut().set_display_power(PowerType::None);
    assert_eq!(creature.power_type(), PowerType::Mana);
}
