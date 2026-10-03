use super::super::learning_fixtures as f;
use super::*;
use std::sync::Arc;

fn family(name: u32, flags: [u32; 4], dispel: u32) -> Definition {
    let mut d = f::definition(vec![]);
    d.fields.spell_family_name = name;
    d.fields.spell_family_flags = flags;
    d.fields.dispel = dispel;
    d
}

#[test]
fn standing_food_drink_uses_real_auras_and_never_falls_through_to_scrolls() {
    for (auras, expected) in [
        (vec![84], SpellSpecific::Food),
        (vec![20], SpellSpecific::Food),
        (vec![85], SpellSpecific::Drink),
        (vec![21], SpellSpecific::Drink),
        (vec![84, 21], SpellSpecific::FoodAndDrink),
    ] {
        let mut d = f::definition(auras.into_iter().map(|a| f::effect(6, a, 0, 0.0)).collect());
        d.fields.aura_interrupt_flags[0] = 0x40000;
        assert_eq!(classify(&d, 8118, 8118), Ok(expected));
    }
    let mut d = f::definition(vec![f::effect(3, 84, 0, 0.0), f::effect(6, 0, 0, 0.0)]);
    d.fields.aura_interrupt_flags[0] = 0x40000;
    assert_eq!(classify(&d, 8118, 8118), Ok(SpellSpecific::Normal));
    d.fields.aura_interrupt_flags = [0, 0x40000];
    for first in [8118, 8099, 8112, 8096, 8115, 8091] {
        assert_eq!(classify(&d, 999, first), Ok(SpellSpecific::Scroll));
    }
    assert_eq!(classify(&d, 8118, 999), Ok(SpellSpecific::Normal));
    d.fields.spell_family_name = 4;
    assert_eq!(classify(&d, 8118, 8118), Ok(SpellSpecific::Normal));
}

#[test]
fn family_rules_keep_each_source_bit_and_precedence() {
    use SpellSpecific::*;
    for (name, word, bits, expected) in [
        (3, 0, 0x12040000, MageArmor),
        (3, 0, 0x400, MageArcaneBrilliance),
        (5, 1, 0x20000020, WarlockArmor),
        (5, 2, 0x10, WarlockArmor),
        (6, 0, 0x20, PriestDivineSpirit),
        (9, 0, 0x200000, Aspect),
        (9, 2, 0x1010, Aspect),
        (10, 1, 0xA2000800, Seal),
        (10, 0, 0x2190, Hand),
        (11, 1, 0x420, ElementalShield),
        (11, 0, 0x400, ElementalShield),
    ] {
        for bit in 0..32 {
            if bits & (1 << bit) == 0 {
                continue;
            }
            let mut flags = [0; 4];
            flags[word] = 1 << bit;
            assert_eq!(classify(&family(name, flags, 0), 999, 999), Ok(expected));
        }
    }
    assert_eq!(
        classify(&family(3, [0x12040400, 0, 0, 0], 0), 999, 999),
        Ok(MageArmor)
    );
    assert_eq!(
        classify(&family(5, [0, 0x20, 0, 0], 2), 999, 999),
        Ok(Curse)
    );
    for id in [603, 980, 80240] {
        assert_eq!(classify(&family(5, [0, 0x20, 0, 0], 2), id, id), Ok(Bane));
    }
    assert_eq!(
        classify(&family(9, [0x200000, 0, 0x1010, 0], 4), 999, 999),
        Ok(Sting)
    );
    for id in [465, 32223, 183435, 317920] {
        assert_eq!(classify(&family(10, [0; 4], 0), id, id), Ok(Aura));
        assert_eq!(
            classify(&family(10, [0x10, 0x800, 0, 0], 0), id, id),
            Ok(Seal)
        );
        assert_eq!(classify(&family(10, [0x10, 0, 0, 0], 0), id, id), Ok(Hand));
    }
    assert_eq!(
        classify(&family(4, [0; 4], 0), 12292, 12292),
        Ok(WarriorEnrage)
    );
    assert_eq!(
        classify(&family(11, [0; 4], 0), 23552, 23552),
        Ok(ElementalShield)
    );
    for id in [48266, 48263, 48265] {
        assert_eq!(classify(&family(15, [0; 4], 0), id, id), Ok(Presence));
        assert_eq!(classify(&family(0, [0; 4], 0), id, id), Ok(Normal));
    }
}

#[test]
fn polymorph_queries_only_physical_effect_zero_and_reports_undefined_missing_slot() {
    let mut d = family(3, [0x1000000, 0, 0, 0], 0);
    assert_eq!(
        classify(&d, 1, 1),
        Err(SpellSpecificError::MissingPolymorphEffect)
    );
    d.effects = vec![f::effect(3, 5, 0, 0.0), f::effect(6, 5, 0, 0.0)];
    assert_eq!(classify(&d, 1, 1), Ok(SpellSpecific::Normal));
    d.effects[0].effect = 6;
    assert_eq!(classify(&d, 1, 1), Ok(SpellSpecific::MagePolymorph));
    d.effects.clear();
    d.fields.spell_family_flags[0] |= 0x400;
    assert_eq!(classify(&d, 1, 1), Ok(SpellSpecific::MageArcaneBrilliance));
}

#[test]
fn fallback_charm_and_tracking_keep_apply_aura_only_and_early_return_order() {
    for aura in [6, 378, 2, 177] {
        let d = f::definition(vec![f::effect(6, aura, 0, 0.0)]);
        assert_eq!(classify(&d, 1, 1), Ok(SpellSpecific::Charm));
    }
    for aura in [44, 45, 151] {
        let d = f::definition(vec![f::effect(6, aura, 0, 0.0)]);
        assert_eq!(classify(&d, 1, 1), Ok(SpellSpecific::Tracker));
    }
    let mut d = f::definition(vec![f::effect(35, 6, 0, 0.0), f::effect(27, 44, 0, 0.0)]);
    assert!(d.effects.iter().all(|e| e.is_aura()));
    assert_eq!(classify(&d, 1, 1), Ok(SpellSpecific::Normal));
    d.effects = vec![f::effect(6, 44, 0, 0.0), f::effect(6, 6, 0, 0.0)];
    assert_eq!(classify(&d, 30645, 30645), Ok(SpellSpecific::Normal));
    d.effects.swap(0, 1);
    assert_eq!(classify(&d, 30645, 30645), Ok(SpellSpecific::Charm));
    d.effects = vec![f::effect(6, 45, 0, 0.0)];
    assert_eq!(classify(&d, 30645, 30645), Ok(SpellSpecific::Tracker));
}

#[test]
fn aura_category_family_dispel_precede_full_mechanic_mask_and_frost() {
    use SpellAuraState::*;
    let mut d = family(7, [0x50, 0x4000000, 0, 0], 9);
    d.fields.category_id = 1133;
    d.fields.mechanic = 64;
    assert_eq!(aura::classify(&d, 1064), Ok(FaerieFire));
    d.fields.category_id = 0;
    assert_eq!(aura::classify(&d, 1064), Ok(DruidPeriodicHeal));
    for flags in [[0x10, 0, 0, 0], [0x40, 0, 0, 0], [0, 0x4000000, 0, 0]] {
        d.fields.spell_family_flags = flags;
        assert_eq!(aura::classify(&d, 1064), Ok(DruidPeriodicHeal));
    }
    d.fields.spell_family_name = 8;
    d.fields.spell_family_flags = [0x10000, 0, 0, 0];
    assert_eq!(aura::classify(&d, 1064), Ok(RoguePoisoned));
    d.fields.spell_family_flags = [0; 4];
    assert_eq!(aura::classify(&d, 1064), Ok(Enraged));
    d.fields.dispel = 0;
    assert_eq!(
        aura::classify(&d, 1064),
        Err(SpellSpecificError::UndefinedMechanicShift)
    );
    d.fields.mechanic = 15;
    d.fields.school_mask = 0x10;
    d.effects = vec![f::effect(6, 12, 0, 0.0)];
    assert_eq!(aura::classify(&d, 1064), Ok(Bleed));
    d.fields.mechanic = 18;
    d.effects[0].mechanic = 15;
    assert_eq!(aura::classify(&d, 1064), Ok(Bleed));
    d.effects[0].mechanic = 0;
    assert_eq!(aura::classify(&d, 1064), Ok(Frozen));
    for kind in [12, 26, 455] {
        d.effects[0].aura = kind;
        assert_eq!(aura::classify(&d, 1), Ok(Frozen));
    }
    d.effects[0].effect = 3;
    assert_eq!(aura::classify(&d, 1), Ok(Banished));
    d.effects[0].effect = 0;
    d.effects[0].mechanic = 64;
    assert_eq!(aura::classify(&d, 1), Ok(Banished));
    d.effects[0].effect = 3;
    assert_eq!(
        aura::classify(&d, 1),
        Err(SpellSpecificError::UndefinedMechanicShift)
    );
    d.effects[0].mechanic = 63;
    assert_eq!(aura::classify(&d, 1), Ok(Banished));
}

#[test]
fn every_source_aura_id_precedes_banish_and_otherwise_defaults() {
    use SpellAuraState::*;
    let mut d = f::definition(vec![]);
    d.fields.mechanic = 18;
    assert_eq!(aura::classify(&d, 1064), Ok(Dazed));
    assert_eq!(aura::classify(&d, 32216), Ok(Victorious));
    for id in [71465, 50241, 81262] {
        assert_eq!(aura::classify(&d, id), Ok(RaidEncounter));
    }
    for id in [
        6950, 9806, 9991, 13424, 13752, 16432, 20656, 25602, 32129, 35325, 35328, 35329, 35331,
        49163, 65863, 79559, 82855, 102953, 127907, 127913, 129007, 130159, 142537, 168455, 176905,
        189502, 201785, 201786, 201935, 239233, 319400, 321470, 331134,
    ] {
        assert_eq!(aura::classify(&d, id), Ok(FaerieFire));
    }
    assert_eq!(aura::classify(&d, 1), Ok(Banished));
    d.fields.mechanic = 0;
    assert_eq!(aura::classify(&d, 1), Ok(None));
}

#[test]
fn startup_uses_shared_rank_first_and_classifies_every_difficulty_before_publication() {
    use wow_data::forever_birth::BirthRecords;
    let rows = vec![
        ((900, 2), f::definition(vec![])),
        ((8118, 0), f::definition(vec![])),
        ((900, 0), f::definition(vec![])),
    ];
    let s = f::seeds(
        rows,
        Default::default(),
        BirthRecords {
            abilities: vec![f::ability(1, 8118, 900)],
            ..Default::default()
        },
    )
    .with_spell_ranks()
    .unwrap()
    .with_spell_required(vec![])
    .unwrap()
    .with_learn_skills(&f::items(), &mut f::no_draw)
    .unwrap();
    let raw = s.raw_catalog();
    assert_eq!(
        s.get_exact(900, 2).unwrap().spell_specific(),
        SpellSpecific::Normal
    );
    let s = s.with_specific_and_aura_state().unwrap();
    assert_eq!(s.specific_counts(), Some(SpecificCounts { definitions: 3 }));
    assert!(Arc::ptr_eq(&raw, &s.raw_catalog()));
    for view in s.records() {
        assert_eq!(view.spell_specific(), SpellSpecific::Scroll);
        assert_eq!(view.aura_state(), SpellAuraState::None);
    }
    assert!(matches!(
        s.with_specific_and_aura_state(),
        Err(SpellSpecificError::AlreadyApplied)
    ));
}

#[test]
fn phase_guards_and_undefined_inputs_do_not_return_a_partially_initialized_owner() {
    let s = f::seeds(vec![], Default::default(), Default::default());
    assert!(matches!(
        s.with_specific_and_aura_state(),
        Err(SpellSpecificError::RequiresLearnSkills)
    ));
    let s = f::before_specific(vec![])
        .with_specific_and_aura_state()
        .unwrap();
    assert_eq!(s.specific_counts(), Some(SpecificCounts { definitions: 0 }));
    let d = family(3, [0x1000000, 0, 0, 0], 0);
    assert!(matches!(
        f::before_specific(vec![((1, 0), d)]).with_specific_and_aura_state(),
        Err(SpellSpecificError::MissingPolymorphEffect)
    ));
    let mut d = f::definition(vec![]);
    d.fields.mechanic = 64;
    assert!(matches!(
        f::before_specific(vec![((1, 0), d)]).with_specific_and_aura_state(),
        Err(SpellSpecificError::UndefinedMechanicShift)
    ));
}
