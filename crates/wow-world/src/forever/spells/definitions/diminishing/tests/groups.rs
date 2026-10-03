use super::*;
use DiminishingGroup::*;

#[test]
fn exact_single_family_flag_bits_without_mechanic_heuristics() {
    // Independent source-case ledger: family, word, bit, expected group.
    let cases = [
        (3, 0, 0x40, Root),
        (3, 2, 0x200, Root),
        (3, 0, 0x800000, Incapacitate),
        (3, 0, 0x1000000, Incapacitate),
        (3, 2, 0x40, Incapacitate),
        (3, 2, 0x800000, Incapacitate),
        (4, 1, 0x8000, Stun),
        (4, 2, 0x1000, Stun),
        (4, 0, 0x40000, Disorient),
        (5, 0, 0x80000, Incapacitate),
        (5, 1, 0x8000000, Incapacitate),
        (5, 1, 0x400, Disorient),
        (5, 1, 8, Disorient),
        (5, 1, 0x1000, Stun),
        (5, 0, 0x1000, Stun),
        (57, 0, 0x8000000, AoeKnockback),
        (57, 0, 0x2000000, Disorient),
        (57, 1, 4, Stun),
        (7, 1, 0x80, Stun),
        (7, 0, 0x2000, Stun),
        (7, 1, 1, Incapacitate),
        (7, 1, 0x20, Disorient),
        (7, 1, 0x1000000, AoeKnockback),
        (7, 0, 0x200, Root),
        (7, 2, 4, Root),
        (8, 0, 0x800000, Stun),
        (8, 0, 0x400, Stun),
        (8, 0, 0x200000, Stun),
        (8, 0, 8, Incapacitate),
        (8, 0, 0x80, Incapacitate),
        (8, 0, 0x1000000, Disorient),
        (8, 1, 0x20000000, Silence),
        (9, 0, 8, Incapacitate),
        (9, 1, 0x1000, Incapacitate),
        (9, 2, 0x40, Disorient),
        (9, 2, 0x8000, Disorient),
        (10, 0, 4, Incapacitate),
        (10, 0, 0x4000, Silence),
        (10, 0, 0x800, Stun),
        (11, 1, 0x8000, Incapacitate),
        (11, 1, 0x2000, AoeKnockback),
        (11, 2, 0x4000, Root),
        (11, 3, 0x2000000, Stun),
        (15, 0, 0x200, Silence),
        (15, 2, 0x100000, Stun),
        (6, 0, 0x10000, Disorient),
        (53, 1, 0x800000, Stun),
        (53, 1, 0x200, Stun),
        (53, 2, 0x800000, Incapacitate),
    ];
    for family in [
        0,
        1,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        15,
        53,
        57,
        107,
        255,
        u32::MAX,
    ] {
        for word in 0..4 {
            for shift in 0..32 {
                let bit = 1u32 << shift;
                let mut flags = [0; 4];
                flags[word] = bit;
                let mut d = definition(family, flags);
                d.fields.mechanic = 12;
                let expected = cases
                    .iter()
                    .find(|row| row.0 == family && row.1 == word && row.2 == bit)
                    .map_or(None, |row| row.3);
                assert_eq!(
                    super::super::groups::compute(&d, 900_001, &mut || Ok(0)).unwrap(),
                    expected,
                    "{family}/{word}/{shift}"
                );
            }
        }
    }
}

#[test]
fn global_ids_precede_families_but_positive_and_active_taunt_precede_ids() {
    for (id, expected) in [
        (20549, Stun),
        (24394, Stun),
        (118345, Stun),
        (118905, Stun),
        (107079, Incapacitate),
        (155145, Silence),
        (108199, AoeKnockback),
        (191244, AoeKnockback),
    ] {
        let mut d = definition(3, [0x40, 0, 0, 0]);
        assert_eq!(
            super::super::groups::compute(&d, id, &mut no_visual).unwrap(),
            expected
        );
        d.effects = vec![SpellEffectValues {
            effect: 6,
            aura: 11,
            ..Default::default()
        }];
        assert_eq!(
            super::super::groups::compute(&d, id, &mut no_visual).unwrap(),
            Taunt
        );
        d.negative_effects = [false; 32];
        assert_eq!(
            super::super::groups::compute(&d, id, &mut no_visual).unwrap(),
            None
        );
    }
    let mut d = definition(0, [0; 4]);
    for kind in [0, 3] {
        d.effects = vec![SpellEffectValues {
            effect: kind,
            aura: 11,
            ..Default::default()
        }];
        assert_eq!(
            super::super::groups::compute(&d, 900_001, &mut no_visual).unwrap(),
            None
        );
    }
}

#[test]
fn family_ids_and_ordered_collision_precedence_are_retained() {
    for (family, id, expected) in [
        (0, 48400, None),
        (0, 47481, Stun),
        (0, 66689, None),
        (0, 64155, None),
        (0, 51750, None),
        (0, 48179, None),
        (5, 170995, LimitOnly),
        (7, 163505, Stun),
        (7, 81261, Silence),
        (7, 118283, AoeKnockback),
        (9, 53148, Root),
        (9, 200108, Root),
        (9, 212638, Root),
        (9, 117526, Stun),
        (9, 202933, Silence),
        (10, 105421, Disorient),
        (15, 96294, Root),
        (15, 207167, Disorient),
        (15, 91800, Stun),
        (15, 91797, Stun),
        (15, 207171, Stun),
        (6, 226943, Stun),
        (6, 204263, AoeKnockback),
        (53, 116706, Root),
        (53, 202274, Incapacitate),
        (53, 198909, Disorient),
        (107, 179057, Stun),
        (107, 211881, Stun),
        (107, 200166, Stun),
        (107, 205630, Stun),
        (107, 217832, Incapacitate),
        (107, 221527, Incapacitate),
    ] {
        assert_eq!(
            super::super::groups::compute(&definition(family, [0; 4]), id, &mut no_visual).unwrap(),
            expected
        );
    }
    for (family, id, flags, expected) in [
        (3, 900_001, [0x800040, 0, 0, 0], Root),
        (5, 170995, [0x80000, 0x1000, 0, 0], Incapacitate),
        (7, 81261, [0, 0x80, 0, 0], Stun),
        (10, 105421, [4, 0, 0, 0], Incapacitate),
        (53, 900_001, [0, 0x800000, 8, 0], None),
    ] {
        assert_eq!(
            super::super::groups::compute(&definition(family, flags), id, &mut no_visual).unwrap(),
            expected
        );
    }
}
