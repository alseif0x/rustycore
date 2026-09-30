#[test]
fn armor_reduction_matches_calc_armor_reduced_damage_like_cpp() {
    use wow_combat::armor_reduced_damage_like_cpp as reduced;
    // C++ `Unit::CalcArmorReducedDamage` (`Unit.cpp:1623-1685`) with a level-80
    // attacker: `levelModifier = 80 + 4.5 * 21 = 174.5`. Rows are
    // `(damage, attacker level, victim level, armour, CR_ARMOR_PENETRATION %,
    //   MOD_TARGET_RESISTANCE sum, MOD_IGNORE_TARGET_RESIST %, BYPASS_ARMOR %, expected)`.
    let cases: &[(u32, u8, u8, i32, f32, i32, f32, f32, u32)] = &[
        // No armour.
        (1_000, 80, 80, 0, 0.0, 0, 0.0, 0.0, 1_000),
        // 5,000 armour: `ceil(1000 * (1 - 0.247127))`.
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, 0.0, 753),
        // A 25% CR_ARMOR_PENETRATION rating ignores a quarter of the armour.
        (1_000, 80, 80, 5_000, 25.0, 0, 0.0, 0.0, 803),
        // 100% of that rating removes the whole armour value.
        (1_000, 80, 80, 5_000, 100.0, 0, 0.0, 0.0, 1_000),
        // The reduction clamps at 75%.
        (1_000, 80, 80, 10_000_000, 0.0, 0, 0.0, 0.0, 250),
        // A negative MOD_TARGET_RESISTANCE sum cancels the armour.
        (1_000, 80, 80, 5_000, 0.0, -5_000, 0.0, 0.0, 1_000),
        // A victim below level 60 uses `maxArmorPen = 400 + 85 * level`.
        (1_000, 80, 10, 500, 0.0, 0, 0.0, 0.0, 969),
        // `std::floor(AddPct(armor, -amount))` per ignore-resist effect.
        (1_000, 80, 80, 5_000, 0.0, 0, 50.0, 0.0, 860),
        (1_000, 80, 80, 5_000, 0.0, 0, 100.0, 0.0, 1_000),
        (1_000, 80, 80, 5_000, 0.0, 0, -50.0, 0.0, 671),
        // `CalculatePct(armor, 100 - min(bypass, 100))` for
        // `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER`, applied before the
        // target-resistance sum (884 if applied after).
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, 50.0, 860),
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, 100.0, 1_000),
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, 150.0, 1_000),
        (1_000, 80, 80, 5_000, 0.0, -1_000, 0.0, 50.0, 911),
        (1_000, 80, 80, 5_000, 0.0, 0, 0.0, -50.0, 671),
    ];
    for &(damage, attacker, victim, armor, pen, target_resist, ignore, bypass, expected) in cases {
        assert_eq!(
            reduced(
                damage,
                attacker,
                victim,
                armor,
                pen,
                target_resist,
                ignore,
                bypass
            ),
            expected,
            "armour {armor}, pen {pen}, targetResist {target_resist}, ignore {ignore}, bypass {bypass}"
        );
    }
}
