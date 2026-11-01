// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exercise the extracted damage API without compiling a Session fixture.
//! The existing wow-world packet/outcome regression remains at the adapter.

use wow_combat::{RepresentedMeleeOutcomeLikeCpp as Outcome, melee_outcome_damage_like_cpp};

#[test]
fn represented_damage_and_original_damage_follow_the_existing_outcome_contract() {
    // Unit::CalculateMeleeDamage, Unit.cpp:1343-1440 at the pinned 3.4.3 source.
    // Immune is the existing represented adapter result, not a new claim that
    // the target's pre-table immunity return assigns OriginalDamage here.
    for (outcome, expected) in [
        (Outcome::Immune, (0, 0, 100)),
        (Outcome::Evade, (0, 0, 100)),
        (Outcome::Miss, (0, 0, 100)),
        (Outcome::Dodge, (0, 0, 100)),
        (Outcome::Parry, (0, 0, 100)),
        (Outcome::Hit, (100, 0, 100)),
        (Outcome::Crit, (200, 0, 200)),
        (Outcome::Crushing, (150, 0, 150)),
        (Outcome::Block, (70, 30, 100)),
        (Outcome::Glancing, (100, 0, 100)),
    ] {
        assert_eq!(
            melee_outcome_damage_like_cpp(outcome, 100, 80, 80, 1.0, 30.0),
            expected,
            "{outcome:?}"
        );
    }
}

#[test]
fn represented_rounding_and_glancing_cap_are_preserved() {
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Block, 7, 80, 80, 1.0, 30.0),
        (5, 2, 7)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Crushing, 101, 80, 80, 1.0, 30.0),
        (151, 0, 151)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Crit, 100, 80, 80, 2.0, 30.0),
        (400, 0, 400)
    );
    for victim_level in [83, 84, 90] {
        assert_eq!(
            melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, victim_level, 1.0, 30.0),
            (70, 0, 100)
        );
    }
}
