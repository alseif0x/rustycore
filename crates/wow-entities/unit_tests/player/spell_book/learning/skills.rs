use super::*;

#[test]
fn trait_override_is_canonical_and_primary_profession_child_does_not_consume_a_slot() {
    let fixture = Fixture {
        spells: vec![SpellFacts {
            definition: Some(DefinitionFacts {
                effects: vec![EffectFacts {
                    kind: 118,
                    aura: false,
                    aura_kinds: vec![],
                    misc: 5,
                    mask: [0; 4],
                }],
                has_effect: vec![(36, false), (44, false), (155, false), (40, false)],
                ..DefinitionFacts::regular(1)
            }),
            learned_skill: Some(SpellLearningSkill {
                skill: 5,
                step: 1,
                value: 0,
                max_value: 0,
            }),
            ..SpellFacts::plain(1, 1)
        }],
        lines: vec![(
            5,
            Some(SkillLineMetadata {
                id: 5,
                category: 11,
                parent_skill: 6,
                parent_tier_index: 0,
            }),
        )],
        race_class: vec![(5, None)],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.free = 2;
    effects.traits.insert(7, 10);
    let mut request = AddPlayerSpell::learned(1, true, false);
    request.trait_data = Some(PlayerSpellTrait::new(7, 1));
    book.add_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.free, 2);
    assert!(book.has_override_spell(10, 1));
    book.add_override_spell(10, 1);
    book.remove_override_spell(10, 1);
    book.remove_override_spell(10, 1);
    assert!(!book.has_override_spell(10, 1));
    assert!(!book.overrides.contains_key(&10));
}
