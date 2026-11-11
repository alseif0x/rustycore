//! Generic boundary probes still resolve actual World definitions and effects.
use super::{PlayerSpellBook, SpellBookMutation};
use crate::forever::player::{
    CastOverrideAura, CastSpellAuras, CastSpellError, CastSpellResult, GenericCastOverrideAura,
    GenericCastSpellAuras, GenericCastSpellError, GenericCastSpellResult,
};
use crate::forever::spells::{
    SpellConstructorFields, SpellDefinitionError, SpellDefinitionSeeds, SpellDefinitionView,
    SpellEffectValues, SpellEffectView, cast_test_definitions, cast_test_shared_catalog_pair,
};
use wow_data::forever_spells::{DifficultyRecord, SpellText};
use wow_entities::{SpellBookDefinition, SpellDefinitionSource};

// Neither view is Clone/Copy. The wrapper keeps the actual immutable borrowed
// definition, so all resolver phases must use the lower contract rather than
// reaching through a concrete World type or copying a catalog.
struct BorrowedDefinition<'a>(SpellDefinitionView<'a>);
struct BorrowedDefinitions<'a>(&'a SpellDefinitionSeeds);
#[derive(Debug, PartialEq, Eq)]
enum DefinitionError {
    Lookup(SpellDefinitionError),
}

impl<'a> SpellBookDefinition for BorrowedDefinition<'a> {
    type Effect = SpellEffectView<'a>;
    fn spell_id(&self) -> u32 {
        self.0.spell_id()
    }
    fn attributes(&self) -> &[u32; 17] {
        SpellBookDefinition::attributes(&self.0)
    }
    fn custom_attributes(&self) -> u32 {
        self.0.custom_attributes()
    }
    fn stances(&self) -> u64 {
        self.0.stances()
    }
    fn caster_aura_state(&self) -> u32 {
        self.0.caster_aura_state()
    }
    fn equipped_item_class(&self) -> i32 {
        self.0.equipped_item_class()
    }
    fn family_name(&self) -> u32 {
        self.0.family_name()
    }
    fn family_flags(&self) -> [u32; 4] {
        self.0.family_flags()
    }
    fn effect(&self, index: usize) -> Option<Self::Effect> {
        self.0.effect(index)
    }
    fn effect_count(&self) -> usize {
        self.0.effect_count()
    }
    fn effects(&self) -> impl Iterator<Item = Self::Effect> + '_ {
        self.0.effects()
    }
    fn has_effect(&self, kind: u32) -> bool {
        self.0.has_effect(kind)
    }
}
impl SpellDefinitionSource for BorrowedDefinitions<'_> {
    type Error = DefinitionError;
    type Definition<'a>
        = BorrowedDefinition<'a>
    where
        Self: 'a;
    fn definition(
        &self,
        spell: u32,
        difficulty: i16,
    ) -> Result<Option<Self::Definition<'_>>, Self::Error> {
        self.0
            .definition(spell, difficulty)
            .map(|view| view.map(BorrowedDefinition))
            .map_err(DefinitionError::Lookup)
    }
    fn owns<'a>(&self, definition: &Self::Definition<'a>) -> bool
    where
        Self: 'a,
    {
        self.0.owns(&definition.0)
    }
}
fn panic_order(_: &[SpellBookMutation], _: usize) -> Result<Vec<u32>, &'static str> {
    panic!("absent override group must not query order")
}

#[test]
fn non_clone_borrowed_view_resolves_player_and_aura_phases_through_the_source_contract() {
    let mut aura_fields = SpellConstructorFields {
        spell_family_name: 7,
        ..Default::default()
    };
    aura_fields.attributes[8] = 0x40000;
    aura_fields.attributes[11] = 0x200;
    let seeds = cast_test_definitions(
        vec![
            ((1, 0), Default::default(), vec![]),
            (
                (2, 0),
                SpellConstructorFields {
                    spell_family_name: 7,
                    spell_family_flags: [0, 0, 4, 0],
                    ..Default::default()
                },
                vec![],
            ),
            ((3, 0), Default::default(), vec![]),
            (
                (4, 0),
                SpellConstructorFields {
                    spell_family_name: 8,
                    ..Default::default()
                },
                vec![],
            ),
            (
                (20, 0),
                aura_fields,
                vec![SpellEffectValues {
                    effect: 6,
                    aura: 332,
                    class_mask: [0, 0, 4, 0],
                    base_points: 999.,
                    ..Default::default()
                }],
            ),
            (
                (21, 0),
                Default::default(),
                vec![SpellEffectValues {
                    effect: 0,
                    aura: 332,
                    ..Default::default()
                }],
            ),
        ],
        vec![],
    );
    let source = BorrowedDefinitions(&seeds);
    let mut book = PlayerSpellBook::default();
    book.add_override_spell(1, 2);
    book.add_override_spell(3, 4);
    let amount = 3.;
    let regular = [GenericCastOverrideAura::new(
        source.definition(20, 0).unwrap().unwrap(),
        0,
        &amount,
    )];
    let mut calls = 0;
    let result: GenericCastSpellResult<BorrowedDefinition<'_>> = book
        .cast_spell_info(
            source.definition(1, 0).unwrap().unwrap(),
            &source,
            0,
            &GenericCastSpellAuras::new(&regular, &[]),
            |_, count| {
                assert_eq!(count, 1);
                calls += 1;
                Ok::<_, &'static str>(if calls == 1 { vec![2] } else { vec![4] })
            },
        )
        .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(result.definition().spell_id(), 4);
    assert_eq!(result.trigger_flags(), 0x404);
    assert!(source.owns(result.definition()));
    assert!(book.is_empty());
    assert!(book.has_override_spell(1, 2) && book.has_override_spell(3, 4));

    // A raw aura number on a non-aura effect must not pass the real predicate.
    let invalid = [GenericCastOverrideAura::new(
        source.definition(21, 0).unwrap().unwrap(),
        0,
        &amount,
    )];
    let result = PlayerSpellBook::default().cast_spell_info(
        source.definition(1, 0).unwrap().unwrap(),
        &source,
        0,
        &GenericCastSpellAuras::new(&invalid, &[]),
        panic_order,
    );
    assert!(matches!(
        result,
        Err(GenericCastSpellError::InvalidAuraEffect)
    ));
}

#[test]
fn generic_definition_errors_retain_the_concrete_source_error_type() {
    let difficulty = |id, fallback_difficulty_id| DifficultyRecord {
        id,
        name: SpellText::default(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id,
        min_players: 0,
        max_players: 0,
        flags: 0,
        item_context: 0,
        toggle_difficulty_id: 0,
        group_size_health_curve_id: 0,
        group_size_dmg_curve_id: 0,
        group_size_spell_points_curve_id: 0,
        unknown1105: 0,
    };
    let seeds = cast_test_definitions(
        vec![((1, 0), Default::default(), vec![])],
        vec![difficulty(9, 8), difficulty(8, 9)],
    );
    let source = BorrowedDefinitions(&seeds);
    let mut book = PlayerSpellBook::default();
    book.add_override_spell(1, 2);
    let result = book.cast_spell_info(
        source.definition(1, 0).unwrap().unwrap(),
        &source,
        9,
        &GenericCastSpellAuras::new(&[], &[]),
        |_, _| Ok::<_, &'static str>(vec![2]),
    );
    assert!(matches!(
        result,
        Err(GenericCastSpellError::Definition(DefinitionError::Lookup(
            SpellDefinitionError::DifficultyCycle
        )))
    ));
    assert!(book.has_override_spell(1, 2));
    assert!(book.is_empty());
}

#[test]
fn shared_raw_catalog_does_not_admit_foreign_derived_initial_or_aura_definitions() {
    let rows = || {
        vec![
            ((1, 0), Default::default(), vec![]),
            (
                (20, 0),
                Default::default(),
                vec![SpellEffectValues {
                    effect: 6,
                    aura: 332,
                    misc_values: [1, 0],
                    ..Default::default()
                }],
            ),
        ]
    };
    let (owner, other) = cast_test_shared_catalog_pair(rows(), rows());
    assert!(owner.owns(&owner.get_exact(1, 0).unwrap()));
    assert!(other.owns(&other.get_exact(1, 0).unwrap()));
    assert!(!owner.owns(&other.get_exact(1, 0).unwrap()));
    let book = PlayerSpellBook::default();
    let result: Result<CastSpellResult<'_>, CastSpellError<&'static str>> = book.cast_spell_info(
        other.get_exact(1, 0).unwrap(),
        &owner,
        0,
        &CastSpellAuras::new(&[], &[]),
        panic_order,
    );
    assert!(matches!(
        result,
        Err(CastSpellError::MismatchedDefinitionOwner)
    ));
    let amount = 1.;
    let regular = [CastOverrideAura::new(
        other.get_exact(20, 0).unwrap(),
        0,
        &amount,
    )];
    let result = book.cast_spell_info(
        owner.get_exact(1, 0).unwrap(),
        &owner,
        0,
        &CastSpellAuras::new(&regular, &[]),
        panic_order,
    );
    assert!(matches!(
        result,
        Err(CastSpellError::MismatchedDefinitionOwner)
    ));
    assert!(book.is_empty());
}
