//! Original rule cases relocated from World scenarios_spell_state_1.
//! Metadata fixtures expose the same resolved IDs; state is the real owner.
//! Old represented removed-row selectors are read from the canonical removed
//! membership they projected. No Session row schema or fake transition is copied.
use super::{support::*, *};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn remove_known_spell_removes_non_talent_higher_ranks_like_cpp() {
    let mut runtime = runtime(vec![10, 20, 30, 40]);
    let catalog = Catalog {
        next: BTreeMap::from([(10, 20), (20, 30)]),
        ..Default::default()
    };
    remove(&mut runtime, &catalog, 10, false);

    assert_eq!(
        runtime.known_spells_like_cpp(),
        &[40],
        "C++ Player::RemoveSpell recursively removes known non-talent higher ranks before removing the current spell"
    );
    assert!(
        runtime.known_spells_like_cpp().iter().map(|id| (*id, false))
            .chain(runtime.removed_known_spells_like_cpp().iter().map(|id| (*id, true)))
            .all(|(spell_id, removed)| spell_id == 40 || removed)
    );
}

#[test]
fn remove_known_spell_preserves_talent_higher_rank_like_cpp() {
    let mut runtime = runtime(vec![10, 20]);
    let catalog = Catalog {
        next: BTreeMap::from([(10, 20)]),
        talents: BTreeSet::from([20]),
        ..Default::default()
    };
    remove(&mut runtime, &catalog, 10, false);

    assert_eq!(
        runtime.known_spells_like_cpp(),
        &[20],
        "C++ skips recursive higher-rank removal when the next spell has SPELL_ATTR0_CU_IS_TALENT"
    );
}

#[test]
fn remove_known_spell_removes_spells_requiring_it_like_cpp() {
    let mut runtime = runtime(vec![10, 100, 101, 200]);
    let catalog = Catalog {
        requiring: BTreeMap::from([(10, vec![100, 101]), (11, vec![100])]),
        ..Default::default()
    };
    remove(&mut runtime, &catalog, 10, false);

    assert_eq!(
        runtime.known_spells_like_cpp(),
        &[200],
        "C++ Player::RemoveSpell removes spells returned by GetSpellsRequiringSpellBounds recursively"
    );
    assert_eq!(
        runtime.removed_known_spells_like_cpp().iter().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([10, 100, 101]),
        "C++ marks the removed required spell and its non-dependent known dependants as PLAYERSPELL_REMOVED"
    );
    remove(&mut runtime, &catalog, 11, false);
    assert_eq!(
        runtime.known_spells_like_cpp(),
        &[200],
        "C++ returns before recursive required-spell cleanup when the removed spell is not known"
    );
}

#[test]
fn remove_known_spell_removes_learned_dependent_spells_and_overrides_like_cpp() {
    let mut runtime = runtime(vec![10, 20, 30]);
    runtime.add_override_spell_like_cpp(100, 20);
    let catalog = Catalog {
        learned: BTreeMap::from([(10, vec![SpellUnlearnEdge {
            spell_id: 20, overrides_spell_id: 100,
        }])]),
        ..Default::default()
    };
    remove(&mut runtime, &catalog, 10, false);

    assert_eq!(
        runtime.known_spells_like_cpp(),
        &[30],
        "C++ Player::RemoveSpell removes spells returned by GetSpellLearnSpellMapBounds"
    );
    assert!(
        runtime.override_spells_like_cpp().is_empty(),
        "C++ removes OverridesSpell pairs for learned dependent spells"
    );
    assert_eq!(
        runtime.removed_known_spells_like_cpp().iter().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([10, 20])
    );
}

#[test]
fn remove_known_spell_marks_previous_rank_dependent_like_cpp() {
    let mut runtime = runtime(vec![10, 20]);
    runtime.mark_known_spell_dependent_like_cpp(20);
    let catalog = Catalog {
        previous: BTreeMap::from([(20, 10)]),
        ranked: BTreeSet::from([10, 20]),
        ..Default::default()
    };
    remove(&mut runtime, &catalog, 20, false);

    assert_eq!(runtime.known_spells_like_cpp(), &[10]);
    assert!(
        runtime.dependent_known_spells_like_cpp().contains(&10),
        "C++ RemoveSpell copies cur_dependent to the previous rank before AddSpell reactivates it"
    );
    assert!(
        !runtime.removed_known_spells_like_cpp().contains(&20),
        "represented dependent spells are still skipped from normal removed-spell save evidence"
    );
}

#[test]
fn remove_known_spell_clears_previous_rank_dependent_when_current_is_independent_like_cpp() {
    let mut runtime = runtime(vec![10, 20]);
    runtime.mark_known_spell_dependent_like_cpp(10);
    let catalog = Catalog {
        previous: BTreeMap::from([(20, 10)]),
        ranked: BTreeSet::from([10, 20]),
        ..Default::default()
    };
    remove(&mut runtime, &catalog, 20, false);

    assert_eq!(runtime.known_spells_like_cpp(), &[10]);
    assert!(
        !runtime.dependent_known_spells_like_cpp().contains(&10),
        "C++ RemoveSpell updates previous-rank dependent state when it differs from the removed rank"
    );
    assert!(
        runtime.removed_known_spells_like_cpp().contains(&20),
        "non-dependent removed current rank still carries represented _SaveSpells delete evidence"
    );
}

#[test]
fn remove_known_spell_does_not_create_missing_previous_rank_like_cpp() {
    let mut runtime = runtime(vec![20]);
    let catalog = Catalog {
        previous: BTreeMap::from([(20, 10)]),
        ranked: BTreeSet::from([10, 20]),
        ..Default::default()
    };
    remove(&mut runtime, &catalog, 20, false);

    assert!(
        runtime.known_spells_like_cpp().is_empty(),
        "C++ RemoveSpell only reactivates a previous rank when prev_id already exists in PlayerSpellMap; Rust does not invent an absent previous row in the represented model"
    );
}

#[test]
fn remove_known_spell_erases_override_source_like_cpp() {
    let mut runtime = runtime(vec![10, 30]);
    runtime.add_override_spell_like_cpp(10, 20);
    runtime.add_override_spell_like_cpp(30, 40);
    remove(&mut runtime, &Catalog::default(), 10, false);

    assert!(
        !runtime.override_spells_like_cpp().contains_key(&10),
        "C++ Player::RemoveSpell erases m_overrideSpells[spell_id] after removing the spell"
    );
    assert_eq!(
        runtime.override_spells_like_cpp().get(&30).cloned().unwrap_or_default()
            .into_iter().collect::<Vec<_>>(),
        vec![40],
        "unrelated override spell entries are not removed by m_overrideSpells.erase(spell_id)"
    );
}

#[test]
fn remove_known_spell_removes_trait_definition_override_like_cpp() {
    let mut runtime = runtime(vec![20, 40]);
    runtime.add_override_spell_like_cpp(10, 20);
    runtime.add_override_spell_like_cpp(30, 40);
    runtime.set_trait_definition_id_like_cpp(20, Some(7));
    let catalog = Catalog { traits: BTreeMap::from([(7, 10)]), ..Default::default() };
    remove(&mut runtime, &catalog, 20, false);

    assert!(
        !runtime.override_spells_like_cpp().get(&10).is_some_and(|spells| spells.contains(&20)),
        "C++ Player::RemoveSpell removes TraitDefinition OverridesSpellID -> spell_id pairs"
    );
    assert_eq!(
        runtime.override_spells_like_cpp().get(&30).cloned().unwrap_or_default()
            .into_iter().collect::<Vec<_>>(),
        vec![40],
        "trait-definition cleanup must not remove unrelated override mappings"
    );
    assert!(
        !runtime.trait_definition_ids_like_cpp().contains_key(&20),
        "removed PlayerSpell no longer owns a represented TraitDefinitionId"
    );
}
