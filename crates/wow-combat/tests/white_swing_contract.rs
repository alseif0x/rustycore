use std::cell::{Cell, RefCell};

use wow_combat::{
    ArmorMitigation, RepresentedMeleeAttackerFactsLikeCpp as Attacker,
    RepresentedMeleeDamageTakenLikeCpp as Taken, RepresentedMeleeOutcomeLikeCpp as Outcome,
    RepresentedMeleeVictimFactsLikeCpp as Victim, calculate_white_swing,
    white_swing_damage_roll_bounds,
};

fn attacker() -> Attacker {
    Attacker {
        level: 80,
        melee_hit_chance_pct: 5.0,
        crit_damage_multiplier: 1.0,
        is_controlled_by_player: true,
        no_crushing_blows: true,
        ..Default::default()
    }
}

fn victim() -> Victim {
    Victim {
        level: 80,
        is_creature: true,
        faces_attacker: true,
        is_stand_state: true,
        ..Default::default()
    }
}

#[test]
fn weapon_bounds_preserve_clamp_order_truncation_and_float_casts() {
    for (range, expected) in [
        ([0.0, 0.0], (0, 0)),
        ([-4.0, -2.0], (0, 0)),
        ([9.9, 5.2], (5, 9)),
        ([f32::NAN, 3.9], (0, 3)),
        ([f32::INFINITY, 4.0], (4, u32::MAX)),
        ([f32::NEG_INFINITY, f32::NAN], (0, 0)),
    ] {
        assert_eq!(white_swing_damage_roll_bounds(range[0], range[1]), expected);
    }
}

#[test]
fn damage_draw_precedes_attack_table_draw_with_original_inclusive_bounds() {
    let events = RefCell::new(Vec::new());
    let result = calculate_white_swing(
        [9.9, 5.2],
        1.0,
        (0, 1.0),
        ArmorMitigation::NONE,
        (&attacker(), &victim()),
        Taken::NONE,
        false,
        |min, max| {
            assert!(events.borrow().is_empty());
            events.borrow_mut().push(("damage", min, max));
            7
        },
        |min, max| {
            assert_eq!(*events.borrow(), vec![("damage", 5, 9)]);
            events.borrow_mut().push(("table", min, max));
            9_999
        },
    );

    assert_eq!(result, (Outcome::Hit, (7, 0, 7)));
    assert_eq!(
        *events.borrow(),
        vec![("damage", 5, 9), ("table", 0, 9_999)]
    );
}

#[test]
fn done_and_autoattack_remain_float_until_rounding_before_taken() {
    for (range, done, autoattack, taken, expected_damage) in [
        ([5.0, 5.0], (1, 1.25), 0.5, Taken::NONE, 4),
        ([1.0, 1.0], (0, 1.5), 1.0, Taken::NONE, 2),
        ([1.0, 1.0], (-3, 1.0), -2.0, Taken::NONE, 4),
        ([1.0, 1.0], (0, 0.5), 1.0, Taken::NONE, 1),
        ([5.0, 5.0], (-10, 1.0), 1.0, Taken::NONE, 1),
        ([0.0, 0.0], (0, 1.0), 0.0, Taken::NONE, 1),
        ([5.0, 5.0], (1, 1.25), 0.5, Taken { flat: 1, pct: 1.5 }, 7),
        ([5.0, 5.0], (-10, 1.0), 1.0, Taken { flat: -2, pct: 1.0 }, 0),
    ] {
        let result = calculate_white_swing(
            range,
            autoattack,
            done,
            ArmorMitigation::NONE,
            (&attacker(), &victim()),
            taken,
            false,
            |min, max| {
                assert_eq!(min, max);
                min
            },
            |min, max| {
                assert_eq!((min, max), (0, 9_999));
                9_999
            },
        );
        assert_eq!(
            result,
            (Outcome::Hit, (expected_damage, 0, expected_damage))
        );
    }
}

#[test]
fn taken_runs_after_done_and_before_armor_reduction() {
    // Done/autoattack: (100 + 10) * 1.5 * 2 = 330.
    // Taken: (330 - 30) * 0.5 = 150. Level-zero armor 400 halves it to 75.
    // Applying taken after armor instead would give 67, so this detects order.
    let result = calculate_white_swing(
        [100.0, 100.0],
        2.0,
        (10, 1.5),
        ArmorMitigation {
            victim_armor: 400,
            ..ArmorMitigation::NONE
        },
        (&Attacker::default(), &Victim::default()),
        Taken {
            flat: -30,
            pct: 0.5,
        },
        false,
        |min, max| {
            assert_eq!((min, max), (100, 100));
            100
        },
        |min, max| {
            assert_eq!((min, max), (0, 9_999));
            9_999
        },
    );
    assert_eq!(result, (Outcome::Hit, (75, 0, 75)));
}

#[test]
fn mainhand_and_offhand_select_distinct_critical_and_expertise_tables() {
    let player_victim = Victim {
        is_creature: false,
        is_player: true,
        ..victim()
    };
    let critical_attacker = Attacker {
        crit_pct: [100.0, 0.0],
        crit_damage_multiplier: 1.25,
        ..attacker()
    };
    let expert_attacker = Attacker {
        expertise_reduction_pct: [0.0, 100.0],
        ..attacker()
    };
    let dodging_victim = Victim {
        dodge_pct: 100.0,
        ..player_victim
    };

    for (attacker, victim, offhand, expected) in [
        (
            critical_attacker,
            player_victim,
            false,
            (Outcome::Crit, (250, 0, 250)),
        ),
        (
            critical_attacker,
            player_victim,
            true,
            (Outcome::Hit, (100, 0, 100)),
        ),
        (
            expert_attacker,
            dodging_victim,
            false,
            (Outcome::Dodge, (0, 0, 100)),
        ),
        (
            expert_attacker,
            dodging_victim,
            true,
            (Outcome::Hit, (100, 0, 100)),
        ),
    ] {
        assert_eq!(
            calculate_white_swing(
                [100.0, 100.0],
                1.0,
                (0, 1.0),
                ArmorMitigation::NONE,
                (&attacker, &victim),
                Taken::NONE,
                offhand,
                |_, _| 100,
                |_, _| 0,
            ),
            expected,
        );
    }
}

#[test]
fn immunity_and_evade_still_draw_twice_even_with_equal_weapon_bounds() {
    for (immune, evading, expected_outcome) in [
        (true, false, Outcome::Immune),
        (false, true, Outcome::Evade),
        (true, true, Outcome::Immune),
    ] {
        let victim = Victim {
            is_immune_to_damage: immune,
            is_evading_attacks: evading,
            ..victim()
        };
        let calls = Cell::new(0);
        let result = calculate_white_swing(
            [20.0, 20.0],
            1.0,
            (0, 1.0),
            ArmorMitigation::NONE,
            (&attacker(), &victim),
            Taken::NONE,
            false,
            |min, max| {
                assert_eq!(calls.get(), 0);
                assert_eq!((min, max), (20, 20));
                calls.set(1);
                20
            },
            |min, max| {
                assert_eq!(calls.get(), 1);
                assert_eq!((min, max), (0, 9_999));
                calls.set(2);
                9_999
            },
        );
        assert_eq!(calls.get(), 2);
        assert_eq!(result, (expected_outcome, (0, 0, 20)));
    }
}

#[test]
fn block_and_critical_damage_keep_the_existing_original_damage_convention() {
    for (attacker, victim, expected) in [
        (
            attacker(),
            Victim {
                block_pct: 100.0,
                ..victim()
            },
            (Outcome::Block, (70, 30, 100)),
        ),
        (
            Attacker {
                crit_pct: [100.0; 2],
                crit_damage_multiplier: 1.75,
                ..attacker()
            },
            victim(),
            (Outcome::Crit, (350, 0, 350)),
        ),
    ] {
        assert_eq!(
            calculate_white_swing(
                [100.0, 100.0],
                1.0,
                (0, 1.0),
                ArmorMitigation::NONE,
                (&attacker, &victim),
                Taken::NONE,
                false,
                |_, _| 100,
                |_, _| 0,
            ),
            expected,
        );
    }
}

#[test]
fn attack_table_roll_conversion_retains_default_on_i32_overflow() {
    let victim = Victim {
        block_pct: 100.0,
        ..victim()
    };
    let result = calculate_white_swing(
        [7.0, 7.0],
        1.0,
        (0, 1.0),
        ArmorMitigation::NONE,
        (&attacker(), &victim),
        Taken::NONE,
        false,
        |_, _| 7,
        |_, _| u32::MAX,
    );
    assert_eq!(result, (Outcome::Block, (5, 2, 7)));
}
