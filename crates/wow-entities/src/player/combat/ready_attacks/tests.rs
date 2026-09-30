use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use super::Player;
use crate::{
    AppliedAuraRef, CurrentSpellRef, CurrentSpellSlot, Item, PlayerInventoryItem,
    SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES_LIKE_CPP,
    SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
};
use wow_constants::{
    EQUIPMENT_SLOT_OFFHAND, InventoryType, ShapeShiftForm, UnitFlags, UnitState, WeaponAttackType,
};
use wow_core::ObjectGuid;

fn ready_player(with_offhand: bool) -> Player {
    let mut player = Player::new(Some(7), false);
    let unit = player.unit_mut();
    unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
    unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
    unit.set_base_attack_time_like_cpp(WeaponAttackType::OffAttack, 1_500);
    unit.set_base_attack_time_like_cpp(WeaponAttackType::RangedAttack, 2_500);
    unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
    unit.set_attack_timer(WeaponAttackType::OffAttack, 0);
    unit.set_attack_timer(WeaponAttackType::RangedAttack, 700);
    unit.set_weapon_damage(WeaponAttackType::BaseAttack, 1.0, 2.0);
    unit.set_weapon_damage(WeaponAttackType::OffAttack, 3.0, 4.0);
    unit.set_mod_autoattack_damage_pct_like_cpp(1.25);

    let mut stats = *player.effective_combat_stats_like_cpp();
    stats.weapon_damage[WeaponAttackType::BaseAttack as usize] = [11.0, 13.0];
    stats.weapon_damage[WeaponAttackType::OffAttack as usize] = [5.0, 6.0];
    player.replace_effective_combat_stats_like_cpp(stats);

    if with_offhand {
        let guid = ObjectGuid::create_item(1, 61_001);
        let mut item = Item::default();
        item.object_mut().create(guid);
        item.set_max_durability(100);
        item.set_durability(100);
        player.inventory_runtime_mut_like_cpp().inventory_items_mut().insert(
            EQUIPMENT_SLOT_OFFHAND,
            PlayerInventoryItem {
                guid,
                entry_id: 61_001,
                db_guid: 61_001,
                inventory_type: Some(InventoryType::WeaponOffhand as u8),
            },
        );
        player.inventory_runtime_mut_like_cpp().item_objects_mut().insert(guid, item);
    }
    player
}

fn install_interrupt_auras(player: &mut Player) -> (AppliedAuraRef, AppliedAuraRef) {
    let caster = ObjectGuid::create_player(1, 7);
    let interrupted = AppliedAuraRef::new(701, caster, 0, 0x1);
    let kept = AppliedAuraRef::new(702, caster, 0, 0x2);
    let auras = &mut player.unit_mut().subsystems_mut().auras;
    auras.register_applied_aura(
        interrupted,
        None,
        SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
        0,
    );
    auras.register_applied_aura(kept, None, 0x20, 0);
    (interrupted, kept)
}

#[test]
fn timers_advance_before_melee_charging_and_casting_gates() {
    for gate in ["not_melee", "charging", "casting"] {
        let mut player = ready_player(true);
        let unit = player.unit_mut();
        unit.set_attack_timer(WeaponAttackType::BaseAttack, 250);
        unit.set_attack_timer(WeaponAttackType::OffAttack, 500);
        unit.set_attack_timer(WeaponAttackType::RangedAttack, 300);
        match gate {
            "not_melee" => unit.clear_unit_state(UnitState::MELEE_ATTACKING.bits()),
            "charging" => unit.add_unit_state(UnitState::CHARGING.bits()),
            "casting" => {
                unit.set_current_cast_spell(
                    CurrentSpellSlot::Generic,
                    CurrentSpellRef::new(710, None, None).with_cast_time_ms(1_000),
                );
            }
            _ => unreachable!(),
        }

        assert_eq!(
            player.take_ready_melee_attacks::<()>(750, true, true, true, 200, |_, _, _| {
                panic!("readiness gate must skip the resolver")
            }),
            None,
        );
        for attack in [
            WeaponAttackType::BaseAttack,
            WeaponAttackType::OffAttack,
            WeaponAttackType::RangedAttack,
        ] {
            assert_eq!(player.unit().attack_timer(attack), 0);
        }
    }
}

#[test]
fn only_generic_and_channeled_spells_pause_attack_timers() {
    for (slot, paused) in [
        (CurrentSpellSlot::Generic, true),
        (CurrentSpellSlot::Channeled, true),
        (CurrentSpellSlot::Autorepeat, false),
        (CurrentSpellSlot::Melee, false),
    ] {
        let mut player = ready_player(false);
        let unit = player.unit_mut();
        unit.set_attack_timer(WeaponAttackType::BaseAttack, 700);
        unit.set_attack_timer(WeaponAttackType::OffAttack, 900);
        unit.set_attack_timer(WeaponAttackType::RangedAttack, 1_100);
        unit.subsystems_mut().spells.set_current_spell(
            slot,
            CurrentSpellRef::new(711, None, None).with_delay_combat_timer_during_cast(true),
        );
        if slot == CurrentSpellSlot::Channeled {
            unit.subsystems_mut().spells.set_current_spell(
                CurrentSpellSlot::Generic,
                CurrentSpellRef::new(712, None, None),
            );
        }

        assert_eq!(
            player.take_ready_melee_attacks::<()>(300, true, true, true, 200, |_, _, _| {
                panic!("no timer is ready")
            }),
            None,
        );
        let expected = if paused { [700, 900, 1_100] } else { [400, 600, 800] };
        for (attack, timer) in [
            WeaponAttackType::BaseAttack,
            WeaponAttackType::OffAttack,
            WeaponAttackType::RangedAttack,
        ].into_iter().zip(expected) {
            assert_eq!(player.unit().attack_timer(attack), timer);
        }
    }
}

#[test]
fn casting_channel_without_actions_blocks_ready_attacks() {
    let mut player = ready_player(true);
    let channel = CurrentSpellRef::new(713, None, None).with_cast_time_ms(2_000);
    player.unit_mut().set_current_cast_spell(CurrentSpellSlot::Channeled, channel);

    assert_eq!(
        player.take_ready_melee_attacks::<()>(100, true, true, true, 200, |_, _, _| {
            panic!("channel without actions must skip the resolver")
        }),
        None,
    );
    assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 0);
    assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 0);
    assert_eq!(player.unit().current_spell(CurrentSpellSlot::Channeled), Some(channel));
}

#[test]
fn channel_actions_and_timer_pause_are_independent() {
    for paused in [false, true] {
        let mut player = ready_player(false);
        let channel = CurrentSpellRef::new(714, None, None)
            .with_cast_time_ms(2_000)
            .with_allow_actions_during_channel(true)
            .with_delay_combat_timer_during_cast(paused);
        player.unit_mut().set_current_cast_spell(CurrentSpellSlot::Channeled, channel);
        player.unit_mut().set_attack_timer(WeaponAttackType::BaseAttack, 100);

        let first = player.take_ready_melee_attacks(150, true, true, true, 200, |attack, _, _| attack);
        if paused {
            assert_eq!(first, None);
            assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 100);
            player.unit_mut().set_attack_timer(WeaponAttackType::BaseAttack, 0);
            assert_eq!(
                player.take_ready_melee_attacks(150, true, true, true, 200, |attack, _, _| attack),
                Some((vec![WeaponAttackType::BaseAttack], Some(None))),
            );
        } else {
            assert_eq!(first, Some((vec![WeaponAttackType::BaseAttack], Some(None))));
        }
        assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 2_000);
        assert_eq!(player.unit().current_spell(CurrentSpellSlot::Channeled), Some(channel));
        assert!(player.unit().has_unit_state(UnitState::CASTING.bits()));
    }
}

#[test]
fn range_error_precedes_facing_and_both_attacks_use_short_retry() {
    for (in_range, facing, error) in [(false, false, 0), (false, true, 0), (true, false, 1)] {
        let mut player = ready_player(true);
        let (interrupted, _) = install_interrupt_auras(&mut player);
        assert_eq!(
            player.take_ready_melee_attacks::<()>(0, in_range, facing, true, 200, |_, _, _| {
                panic!("range/facing error must skip the resolver")
            }),
            Some((vec![], Some(Some(error)))),
        );
        assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 100);
        assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 100);
        assert!(player.unit().subsystems().auras.has_applied(interrupted));
    }
}

#[test]
fn mainhand_precedes_offhand_and_supplies_effective_weapon_snapshots() {
    let mut player = ready_player(true);
    let first = player.take_ready_melee_attacks(0, true, true, true, 200, |attack, range, multiplier| {
        (attack, range, multiplier)
    });
    assert_eq!(
        first,
        Some((vec![(WeaponAttackType::BaseAttack, [11.0, 13.0], 1.25)], Some(None))),
    );
    assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 2_000);
    assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 200);

    let second = player.take_ready_melee_attacks(200, true, true, true, 200, |attack, range, multiplier| {
        (attack, range, multiplier)
    });
    assert_eq!(
        second,
        Some((vec![(WeaponAttackType::OffAttack, [5.0, 6.0], 1.25)], None)),
    );
    assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 1_800);
    assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 1_500);
}

#[test]
fn offhand_only_never_updates_the_mainhand_error() {
    for (in_range, facing) in [(true, true), (false, false), (true, false)] {
        let mut player = ready_player(true);
        player.unit_mut().set_attack_timer(WeaponAttackType::BaseAttack, 500);
        player.set_attack_swing_error_like_cpp(Some(1));
        let outcome = player.take_ready_melee_attacks(0, in_range, facing, true, 200, |attack, _, _| attack);
        let expected = if in_range && facing { vec![WeaponAttackType::OffAttack] } else { vec![] };
        assert_eq!(outcome, Some((expected, None)));
        assert_eq!(player.attack_swing_error_like_cpp(), Some(1));
        assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 500);
        assert_eq!(
            player.unit().attack_timer(WeaponAttackType::OffAttack),
            if in_range && facing { 1_500 } else { 100 },
        );
    }
}

#[test]
fn offhand_requires_equipment_and_nonferal_form() {
    for admission in ["missing", "broken", "feral"] {
        let mut player = ready_player(admission != "missing");
        player.unit_mut().set_can_dual_wield_like_cpp(true);
        player.unit_mut().set_attack_timer(WeaponAttackType::BaseAttack, 500);
        match admission {
            "broken" => {
                player.inventory_runtime_mut_like_cpp().item_objects_mut()
                    .get_mut(&ObjectGuid::create_item(1, 61_001)).expect("offhand item")
                    .set_durability(0);
            }
            "feral" => player.unit_mut().set_shapeshift_form_like_cpp(ShapeShiftForm::CatForm),
            _ => {}
        }
        assert_eq!(
            player.take_ready_melee_attacks::<()>(0, true, true, true, 200, |_, _, _| {
                panic!("offhand admission must skip the resolver")
            }),
            None,
        );
        assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 0);
    }

    // The mainhand sibling delay still applies with an equipped weapon in feral form.
    let mut player = ready_player(true);
    player.unit_mut().set_shapeshift_form_like_cpp(ShapeShiftForm::CatForm);
    assert_eq!(
        player.take_ready_melee_attacks(0, true, true, true, 200, |attack, _, _| attack),
        Some((vec![WeaponAttackType::BaseAttack], Some(None))),
    );
    assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 200);
}

#[test]
fn attack_admission_blocks_still_reset_and_separate_timers() {
    for gate in ["los", "pacified", "controlled", "disable_aura"] {
        for offhand_only in [false, true] {
            let mut player = ready_player(true);
            if offhand_only {
                player.unit_mut().set_attack_timer(WeaponAttackType::BaseAttack, 50);
            }
            let (interrupted, kept) = install_interrupt_auras(&mut player);
            let melee = CurrentSpellRef::new(715, None, None);
            player.unit_mut().set_current_cast_spell(CurrentSpellSlot::Melee, melee);
            match gate {
                "pacified" => player.unit_mut().set_unit_flags_like_cpp(UnitFlags::PACIFIED),
                "controlled" => player.unit_mut().add_unit_state(UnitState::STUNNED.bits()),
                "disable_aura" => {
                    player.unit_mut().subsystems_mut().auras.register_applied_aura_type_like_cpp(
                        AppliedAuraRef::new(703, ObjectGuid::create_player(1, 7), 0, 0x1),
                        SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES_LIKE_CPP,
                    );
                }
                _ => {}
            }

            assert_eq!(
                player.take_ready_melee_attacks::<()>(0, true, true, gate != "los", 200, |_, _, _| {
                    panic!("attack admission must skip the resolver")
                }),
                Some((vec![], if offhand_only { None } else { Some(None) })),
            );
            assert_eq!(
                player.unit().attack_timer(WeaponAttackType::BaseAttack),
                if offhand_only { 200 } else { 2_000 },
            );
            assert_eq!(
                player.unit().attack_timer(WeaponAttackType::OffAttack),
                if offhand_only { 1_500 } else { 200 },
            );
            assert_eq!(player.unit().current_spell(CurrentSpellSlot::Melee), Some(melee));
            assert!(player.unit().subsystems().auras.has_applied(interrupted));
            assert!(player.unit().subsystems().auras.has_applied(kept));
        }
    }
}

#[test]
fn mainhand_finishes_pending_melee_spell_and_interrupts_auras_without_resolving_swing() {
    let mut player = ready_player(true);
    let (interrupted, kept) = install_interrupt_auras(&mut player);
    player.unit_mut().set_current_cast_spell(
        CurrentSpellSlot::Melee,
        CurrentSpellRef::new(716, None, None),
    );
    assert_eq!(
        player.take_ready_melee_attacks::<()>(0, true, true, true, 200, |_, _, _| {
            panic!("pending mainhand melee spell replaces the normal swing")
        }),
        Some((vec![], Some(None))),
    );
    assert_eq!(player.unit().current_spell(CurrentSpellSlot::Melee), None);
    assert!(!player.unit().subsystems().auras.has_applied(interrupted));
    assert!(player.unit().subsystems().auras.has_applied(kept));
    assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 2_000);
    assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 200);
}

#[test]
fn offhand_interrupts_auras_and_preserves_pending_melee_spell() {
    let mut player = ready_player(true);
    player.unit_mut().set_attack_timer(WeaponAttackType::BaseAttack, 500);
    let (interrupted, kept) = install_interrupt_auras(&mut player);
    let melee = CurrentSpellRef::new(717, None, None);
    player.unit_mut().set_current_cast_spell(CurrentSpellSlot::Melee, melee);
    assert_eq!(
        player.take_ready_melee_attacks(0, true, true, true, 200, |attack, _, _| attack),
        Some((vec![WeaponAttackType::OffAttack], None)),
    );
    assert_eq!(player.unit().current_spell(CurrentSpellSlot::Melee), Some(melee));
    assert!(!player.unit().subsystems().auras.has_applied(interrupted));
    assert!(player.unit().subsystems().auras.has_applied(kept));
}

#[test]
fn callback_runs_after_aura_interrupt_and_before_attack_timer_reset() {
    for attack in [WeaponAttackType::BaseAttack, WeaponAttackType::OffAttack] {
        let mut player = ready_player(true);
        if attack == WeaponAttackType::OffAttack {
            player.unit_mut().set_attack_timer(WeaponAttackType::BaseAttack, 50);
        }
        let (interrupted, kept) = install_interrupt_auras(&mut player);
        let observed_attack = Cell::new(None);
        let result = catch_unwind(AssertUnwindSafe(|| {
            player.take_ready_melee_attacks::<()>(0, true, true, true, 200, |kind, _, _| {
                observed_attack.set(Some(kind));
                panic!("stop at the original damage/RNG boundary")
            })
        }));
        assert!(result.is_err());
        assert_eq!(observed_attack.get(), Some(attack));
        assert!(!player.unit().subsystems().auras.has_applied(interrupted));
        assert!(player.unit().subsystems().auras.has_applied(kept));
        assert_eq!(player.unit().attack_timer(attack), 0);
        let sibling = if attack == WeaponAttackType::BaseAttack {
            WeaponAttackType::OffAttack
        } else {
            WeaponAttackType::BaseAttack
        };
        assert_eq!(player.unit().attack_timer(sibling), 200);
    }
}

#[test]
fn processed_attacks_reset_with_existing_speed_multipliers() {
    let mut player = ready_player(true);
    player.unit_mut().apply_attack_time_multipliers_like_cpp([0.75, 0.5, 1.0]);
    assert_eq!(
        player.take_ready_melee_attacks(0, true, true, true, 200, |attack, _, _| attack),
        Some((vec![WeaponAttackType::BaseAttack], Some(None))),
    );
    assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 1_500);
    assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 200);
    assert_eq!(
        player.take_ready_melee_attacks(200, true, true, true, 200, |attack, _, _| attack),
        Some((vec![WeaponAttackType::OffAttack], None)),
    );
    assert_eq!(player.unit().attack_timer(WeaponAttackType::BaseAttack), 1_300);
    assert_eq!(player.unit().attack_timer(WeaponAttackType::OffAttack), 750);
}
