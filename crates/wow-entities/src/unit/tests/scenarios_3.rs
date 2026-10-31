//! Unit values, visibility and health-revision state regression scenarios, part 3 of 3.
//!
//! Moved out of the unit.rs root under #636; every test is unchanged.

use super::*;
use crate::{Creature, Player};

#[test]
fn native_death_power_marks_supported_values_and_queues_only_changes() {
    for index in [0, 3] {
        for (before, requested, expected) in [(0, 0, 0), (37, 0, 0), (0, 200, 100), (37, -3, -3)] {
            let mut unit = Unit::new(true);
            unit.world_mut().object_mut().add_to_world();
            unit.set_power_index(PowerType::Mana, Some(index));
            unit.set_max_power(PowerType::Mana, 100);
            unit.set_power(PowerType::Mana, before);
            unit.clear_unit_data_changes();
            unit.world_mut().object_mut().clear_update_mask(false);
            unit.set_power_for_native_transition_like_cpp(PowerType::Mana, requested);
            assert_eq!(unit.get_power(PowerType::Mana), expected);
            let update = unit.values_update().unit_data.unwrap();
            for bit in 0..UNIT_DATA_BITS {
                assert_eq!(update.mask.is_set(bit), [116, 137 + index].contains(&bit));
            }
            assert_eq!(update.values.power[index], expected);
            assert_eq!(
                unit.world().object().is_object_updated(),
                before != expected,
                "only a changed clamped power value admits the object to updates"
            );
            unit.clear_unit_data_changes();
            unit.set_power(PowerType::Mana, expected);
            assert!(
                !unit.unit_data_changes_mask().is_any_set(),
                "ordinary no-op remains unmarked"
            );
            unit.set_power_suppressing_object_update_like_cpp(PowerType::Mana, 1);
            assert!(
                !unit.unit_data_changes_mask().is_any_set(),
                "regeneration suppression remains intact"
            );
        }
    }
    for index in [None, Some(MAX_POWERS_PER_CLASS)] {
        let mut unit = Unit::new(true);
        unit.world_mut().object_mut().add_to_world();
        unit.set_power_index(PowerType::Mana, index);
        unit.clear_unit_data_changes();
        unit.world_mut().object_mut().clear_update_mask(false);
        let before = unit.data().power;
        unit.set_power_for_native_transition_like_cpp(PowerType::Mana, 0);
        assert_eq!(unit.data().power, before);
        assert!(!unit.unit_data_changes_mask().is_any_set());
        assert!(!unit.world().object().is_object_updated());
    }
}

#[test]
fn death_presentation_reset_marks_exact_nested_fields_even_for_noop() {
    for changed in [false, true] {
        let mut unit = Unit::new(true);
        unit.world_mut().object_mut().add_to_world();
        unit.set_unit_flags_like_cpp(UnitFlags::PLAYER_CONTROLLED);
        if changed {
            unit.set_emote_state_like_cpp(10);
            unit.set_stand_state_like_cpp(UnitStandStateType::Sit);
        }
        unit.clear_unit_data_changes();
        unit.reset_death_presentation_like_cpp();
        assert_eq!(unit.emote_state_like_cpp(), 0);
        assert_eq!(unit.unit_flags_like_cpp(), UnitFlags::PLAYER_CONTROLLED);
        let update = unit.values_update().unit_data.unwrap();
        assert_eq!(update.values.emote_state, 0);
        assert_eq!(update.values.stand_state, 0);
        for bit in 0..UNIT_DATA_BITS {
            assert_eq!(update.mask.is_set(bit), [32, 56, 64, 72].contains(&bit));
        }
        assert!(unit.world().object().is_object_updated());
        unit.clear_unit_data_changes();
        unit.set_emote_state_like_cpp(0);
        unit.set_stand_state_like_cpp(UnitStandStateType::Stand);
        assert!(
            !unit.unit_data_changes_mask().is_any_set(),
            "ordinary setters retain their changed-value contract"
        );
    }
}

#[test]
fn remove_unit_flags_marks_absent_flags_and_preserves_other_bits_like_cpp() {
    for present in [false, true] {
        let mut unit = Unit::new(true);
        let remaining = UnitFlags::PLAYER_CONTROLLED | UnitFlags::IN_COMBAT;
        unit.set_unit_flags_like_cpp(
            remaining
                | if present {
                    UnitFlags::SKINNABLE
                } else {
                    UnitFlags::empty()
                },
        );
        unit.clear_unit_data_changes();
        unit.remove_unit_flags_like_cpp(UnitFlags::SKINNABLE);
        assert_eq!(unit.unit_flags_like_cpp(), remaining);
        let delta = unit
            .values_update()
            .unit_data
            .expect("RemoveUnitFlag dirties Flags even for a no-op");
        assert!(delta.mask.is_set(UNIT_DATA_STAND_STATE_PARENT_BIT));
        assert!(delta.mask.is_set(UNIT_DATA_FLAGS_BIT));
        assert_eq!(delta.values.flags, remaining.bits());
        unit.clear_unit_data_changes();
        unit.set_unit_flags_like_cpp(remaining);
        assert!(
            !unit.unit_data_changes_mask().is_any_set(),
            "the ordinary setter keeps its existing contract"
        );
    }
}

#[test]
fn values_update_sets_unit_object_type_bit() {
    let mut unit = Unit::new(true);

    unit.set_level(12);
    let update = unit.values_update();

    assert!(update.has_data());
    assert_eq!(update.changed_object_type_mask, 1 << TYPEID_UNIT);
    let unit_data = update.unit_data.unwrap();
    assert_eq!(unit_data.values.level, 12);
    assert!(unit_data.mask.is_set(UNIT_DATA_LEVEL_BIT));
}

#[test]
fn emote_state_marks_cpp_nested_unit_data_bits() {
    let mut unit = Unit::new(true);
    unit.clear_unit_data_changes();

    unit.set_emote_state_like_cpp(10);

    assert_eq!(unit.emote_state_like_cpp(), 10);
    assert_eq!(
        unit.values_update().unit_data.unwrap().values.emote_state,
        10
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_MODS_PARENT_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_EMOTE_STATE_BIT)
    );
}

#[test]
fn pvp_flags_match_cpp_unit_pvp_state_helpers() {
    let mut unit = Unit::new(true);

    unit.set_pvp_flag_like_cpp(UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP);
    assert!(unit.is_pvp_like_cpp());
    assert!(unit.is_ffa_pvp_like_cpp());
    assert!(!unit.is_in_sanctuary_like_cpp());
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_PVP_FLAGS_BIT)
    );

    unit.remove_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
    assert!(unit.is_pvp_like_cpp());
    assert!(!unit.is_ffa_pvp_like_cpp());

    unit.replace_all_pvp_flags_like_cpp(UnitPvpFlags::SANCTUARY | UnitPvpFlags::UNK1);
    assert!(!unit.is_pvp_like_cpp());
    assert!(unit.is_in_sanctuary_like_cpp());
    assert!(unit.has_pvp_flag_like_cpp(UnitPvpFlags::UNK1));
}

#[test]
fn native_pvp_flag_helpers_mark_nested_bits_but_admit_only_changes() {
    let mut unit = Unit::new(true);
    let initial_flags = UnitPvpFlags::PVP
        | UnitPvpFlags::FFA_PVP
        | UnitPvpFlags::UNK1
        | UnitPvpFlags::from_bits_retain(0x80);
    unit.replace_all_pvp_flags_like_cpp(initial_flags);
    unit.world_mut().object_mut().add_to_world();
    unit.clear_unit_data_changes();
    unit.world_mut().object_mut().clear_update_mask(false);

    for operation in [0, 1, 2] {
        match operation {
            0 => unit.replace_all_pvp_flags_like_cpp(initial_flags),
            1 => unit.set_pvp_flag_like_cpp(UnitPvpFlags::PVP),
            _ => unit.remove_pvp_flag_like_cpp(UnitPvpFlags::SANCTUARY),
        }
        for bit in 0..UNIT_DATA_BITS {
            assert_eq!(
                unit.unit_data_changes_mask().is_set(bit),
                [64, 78].contains(&bit),
                "native PvP helper {operation} marks only nested bits 64 and 78"
            );
        }
        assert!(!unit.world().object().is_object_updated());
        unit.clear_unit_data_changes();
    }

    unit.set_pvp_flag_like_cpp(UnitPvpFlags::SANCTUARY);
    assert!(unit.world().object().is_object_updated());
    assert!(unit.unit_data_changes_mask().is_set(64));
    assert!(unit.unit_data_changes_mask().is_set(78));
    assert_eq!(
        unit.pvp_flags_like_cpp(),
        initial_flags | UnitPvpFlags::SANCTUARY
    );

    unit.world_mut().object_mut().clear_update_mask(false);
    unit.clear_unit_data_changes();
    unit.set_level(12);
    unit.replace_all_pvp_flags_like_cpp(unit.pvp_flags_like_cpp());
    assert!(unit.world().object().is_object_updated());
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_LEVEL_BIT));
    assert!(unit.unit_data_changes_mask().is_set(64));
    assert!(unit.unit_data_changes_mask().is_set(78));
}

#[test]
fn area_sanctuary_preserves_other_flags_and_queues_only_changes() {
    let other = UnitPvpFlags::PVP
        | UnitPvpFlags::FFA_PVP
        | UnitPvpFlags::UNK1
        | UnitPvpFlags::from_bits_retain(0x80);
    for before in [false, true] {
        for sanctuary in [false, true] {
            let mut unit = Unit::new(true);
            unit.world_mut().object_mut().add_to_world();
            unit.replace_all_pvp_flags_like_cpp(
                other
                    | if before {
                        UnitPvpFlags::SANCTUARY
                    } else {
                        UnitPvpFlags::empty()
                    },
            );
            unit.clear_unit_data_changes();
            unit.world_mut().object_mut().clear_update_mask(false);
            unit.set_sanctuary_from_area_like_cpp(sanctuary);
            let expected = other
                | if sanctuary {
                    UnitPvpFlags::SANCTUARY
                } else {
                    UnitPvpFlags::empty()
                };
            assert_eq!(unit.pvp_flags_like_cpp(), expected);
            let update = unit.values_update().unit_data.unwrap();
            assert_eq!(update.values.pvp_flags, expected.bits());
            for bit in 0..UNIT_DATA_BITS {
                assert_eq!(update.mask.is_set(bit), [64, 78].contains(&bit));
            }
            assert_eq!(
                unit.world().object().is_object_updated(),
                before != sanctuary,
                "only a changed sanctuary flag admits the object to updates"
            );
            unit.clear_unit_data_changes();
            unit.replace_all_pvp_flags_like_cpp(expected);
            for bit in 0..UNIT_DATA_BITS {
                assert_eq!(
                    unit.unit_data_changes_mask().is_set(bit),
                    [64, 78].contains(&bit),
                    "native no-op setter retains only the nested PvP field bits"
                );
            }
        }
    }
}

#[test]
fn forced_native_field_noops_preserve_pending_bits_and_object_queue_state() {
    let mut unit = Unit::new(true);
    unit.set_level(12);
    unit.set_power_index(PowerType::Mana, Some(0));
    unit.set_max_power(PowerType::Mana, 100);
    unit.set_power(PowerType::Mana, 25);
    unit.replace_all_pvp_flags_like_cpp(UnitPvpFlags::PVP | UnitPvpFlags::SANCTUARY);
    unit.world_mut().object_mut().add_to_world();
    assert!(!unit.world().object().is_object_updated());

    unit.set_power_for_native_transition_like_cpp(PowerType::Mana, 25);
    unit.set_sanctuary_from_area_like_cpp(true);

    assert!(!unit.world().object().is_object_updated());
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_LEVEL_BIT));
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_PVP_FLAGS_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_POWER_FIRST_BIT)
    );

    unit.set_level(13);
    assert!(unit.world().object().is_object_updated());
    unit.set_power_for_native_transition_like_cpp(PowerType::Mana, 25);
    unit.set_sanctuary_from_area_like_cpp(true);

    assert!(unit.world().object().is_object_updated());
    assert!(unit.unit_data_changes_mask().is_set(UNIT_DATA_LEVEL_BIT));
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_PVP_FLAGS_BIT)
    );
    assert!(
        unit.unit_data_changes_mask()
            .is_set(UNIT_DATA_POWER_FIRST_BIT)
    );
}

#[test]
fn health_state_revision_advances_only_for_real_health_or_death_changes_like_cpp() {
    let mut unit = Unit::new(true);
    unit.set_max_health(100);
    assert_eq!(unit.health_state_revision_like_cpp(), 1);

    unit.set_health(100);
    assert_eq!(unit.health_state_revision_like_cpp(), 2);
    unit.set_health(100);
    unit.set_death_state(DeathState::Alive);
    assert_eq!(unit.health_state_revision_like_cpp(), 2);

    unit.set_death_state(DeathState::JustDied);
    assert_eq!(unit.health_state_revision_like_cpp(), 3);
    unit.set_health(50);
    assert_eq!(unit.data().health, 0);
    assert_eq!(unit.health_state_revision_like_cpp(), 4);
    unit.set_health(50);
    assert_eq!(unit.health_state_revision_like_cpp(), 4);
}

#[test]
fn snapshot_preservation_copies_exact_authoritative_health_timeline_like_cpp() {
    let mut canonical = Unit::new(true);
    canonical.set_max_health(100);
    canonical.set_health(100);
    canonical.set_health(0);
    canonical.set_death_state(DeathState::Corpse);

    let mut snapshot = Unit::new(true);
    snapshot.set_max_health(200);
    snapshot.set_health(150);
    snapshot.preserve_authoritative_health_state_for_snapshot_like_cpp(&canonical);

    assert_eq!(snapshot.data().max_health, 100);
    assert_eq!(snapshot.data().health, 0);
    assert_eq!(snapshot.death_state(), DeathState::Corpse);
    assert_eq!(
        snapshot.health_state_revision_like_cpp(),
        canonical.health_state_revision_like_cpp()
    );
    assert!(snapshot.shares_health_state_revision_authority_like_cpp(
        &canonical.health_state_revision_authority_like_cpp()
    ));
}

#[test]
fn lifecycle_stat_player_dead_health_is_one_without_flipping_death_state() {
    let mut player = Player::new(Some(7), false);
    let unit = player.unit_mut();
    unit.set_max_health(20);
    unit.set_health(20);
    unit.set_death_state(DeathState::Dead);

    unit.set_max_health(10);
    assert_eq!(unit.data().health, 1);
    unit.set_health(0);
    assert_eq!(unit.data().health, 1);
    assert_eq!(unit.death_state(), DeathState::Dead);

    unit.set_death_state(DeathState::Alive);
    unit.set_health(0);
    assert_eq!(unit.data().health, 0);
    assert_eq!(unit.death_state(), DeathState::Alive);

    for death_state in [DeathState::JustDied, DeathState::Corpse] {
        unit.set_death_state(death_state);
        unit.set_health(10);
        assert_eq!(unit.data().health, 0);
        assert_eq!(unit.death_state(), death_state);
    }
}

#[test]
fn lifecycle_stat_creature_dead_health_does_not_use_player_minimum() {
    let mut creature = Creature::new(false);
    let unit = creature.unit_mut();
    unit.set_max_health(20);
    unit.set_health(20);
    unit.set_death_state(DeathState::Dead);

    unit.set_max_health(10);
    assert_eq!(unit.data().health, 10);
    unit.set_health(0);
    assert_eq!(unit.data().health, 0);
    assert_eq!(unit.death_state(), DeathState::Dead);
}
