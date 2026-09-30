//! Removal equivalence and continuation contracts; authored, not executed.
use super::*;
use crate::{PlayerKnownSpellRecord, PlayerSpellLoadState, PlayerSpellRuntimeState};
use std::collections::{BTreeMap, BTreeSet};

mod support;
mod legacy;
use support::*;

#[test]
fn unlearn_preserves_lazy_read_owner_skill_override_cleanup_and_notice_order() {
    use SpellUnlearnStep::*;
    use SpellUnlearnOwnerStep::{DropOverridesAndTrait, Forget};
    let mut runtime = runtime(vec![10, 20, 30, 40]);
    runtime.set_trait_definition_id_like_cpp(10, Some(7));
    let catalog = Catalog {
        next: BTreeMap::from([(10, 20)]),
        talents: BTreeSet::from([20]),
        requiring: BTreeMap::from([(10, vec![30])]),
        learned: BTreeMap::from([(10, vec![SpellUnlearnEdge {
            spell_id: 40, overrides_spell_id: 100,
        }])]),
        traits: BTreeMap::from([(7, 200)]),
        ..Default::default()
    };
    let trace = remove(&mut runtime, &catalog, 10, true);
    assert_eq!(trace, vec![
        Known(10), RowsComplete, InvalidateRows, NextRank(10), Talent(20), Known(20),
        Requiring(10), Known(30), Known(30), RowsComplete, InvalidateRows, NextRank(30),
        Requiring(30), Owner(Forget { spell_id: 30, preserve_complete: false }),
        DowngradeSkill(30), Learned(30), PreviousRank(30),
        Owner(DropOverridesAndTrait { spell_id: 30 }),
        TitanGrip(30), DualWield(30), Offhand,
        Unlearned { spell_id: 30, suppress_messaging: false },
        Owner(Forget { spell_id: 10, preserve_complete: false }), DowngradeSkill(10),
        Learned(10), Known(40), RowsComplete, InvalidateRows, NextRank(40), Requiring(40),
        Owner(Forget { spell_id: 40, preserve_complete: false }),
        DowngradeSkill(40), Learned(40), PreviousRank(40),
        Owner(DropOverridesAndTrait { spell_id: 40 }),
        TitanGrip(40), DualWield(40), Offhand,
        Unlearned { spell_id: 40, suppress_messaging: false },
        RemoveOverride { overridden: 100, replacement: 40 },
        PreviousRank(10), Owner(DropOverridesAndTrait { spell_id: 10 }), TraitOverride(7),
        RemoveOverride { overridden: 200, replacement: 10 },
        TitanGrip(10), DualWield(10), Offhand,
        Unlearned { spell_id: 10, suppress_messaging: true }, Done,
    ]);
    assert_eq!(runtime.known_spells_like_cpp(), &[20]);
}

#[test]
fn unlearn_unknown_spell_does_not_resolve_catalogs_or_write_owner() {
    let mut runtime = runtime(vec![10]);
    let before = runtime.clone();
    assert_eq!(remove(&mut runtime, &Catalog::default(), 999, false),
        vec![SpellUnlearnStep::Known(999), SpellUnlearnStep::Done]);
    assert_eq!(runtime, before);
}

#[test]
fn unlearn_failed_child_forget_returns_to_caller_and_keeps_later_override_effect() {
    let mut runtime = runtime(vec![10, 20, 30]);
    runtime.add_override_spell_like_cpp(100, 20);
    let catalog = Catalog {
        requiring: BTreeMap::from([(10, vec![20, 30])]),
        learned: BTreeMap::from([(10, vec![SpellUnlearnEdge {
            spell_id: 20, overrides_spell_id: 100,
        }])]),
        ..Default::default()
    };
    let trace = drive(&mut runtime, &catalog, 10, false,
        &BTreeSet::from([20]), &BTreeSet::new());
    assert_eq!(runtime.known_spells_like_cpp(), &[20]);
    assert_eq!(runtime.removed_known_spells_like_cpp(), &BTreeSet::from([10, 30]));
    assert_eq!(trace.iter().filter(|step| matches!(step,
        SpellUnlearnStep::Owner(SpellUnlearnOwnerStep::Forget { spell_id: 20, .. })
    )).count(), 1, "the second visit keeps the original seen guard");
    assert!(!trace.contains(&SpellUnlearnStep::DowngradeSkill(20)));
    assert!(trace.contains(&SpellUnlearnStep::RemoveOverride { overridden: 100, replacement: 20 }));
    assert!(!runtime.override_spells_like_cpp().contains_key(&100));
    assert!(trace.contains(&SpellUnlearnStep::Unlearned { spell_id: 10, suppress_messaging: false }));
}

#[test]
fn unlearn_cycle_and_duplicates_keep_known_rows_invalidation_before_seen() {
    let mut runtime = runtime(vec![10, 20]);
    let catalog = Catalog {
        requiring: BTreeMap::from([(10, vec![20, 20]), (20, vec![10])]),
        ..Default::default()
    };
    let trace = remove(&mut runtime, &catalog, 10, false);
    assert_eq!(trace.iter().filter(|step| **step == SpellUnlearnStep::InvalidateRows).count(), 3);
    assert_eq!(trace.iter().filter(|step| **step == SpellUnlearnStep::RowsComplete).count(), 3);
    assert_eq!(trace.iter().filter(|step| matches!(step,
        SpellUnlearnStep::Owner(SpellUnlearnOwnerStep::Forget { .. })
    )).count(), 2);
    assert_eq!(trace.iter().filter(|step| **step == SpellUnlearnStep::Known(20)).count(), 3);
    assert!(runtime.known_spells_like_cpp().is_empty());
}

#[test]
fn unlearn_next_rank_talent_read_precedes_known_membership_even_when_unknown() {
    let mut runtime = runtime(vec![10]);
    let catalog = Catalog { next: BTreeMap::from([(10, 20)]), ..Default::default() };
    let trace = remove(&mut runtime, &catalog, 10, false);
    let talent = trace.iter().position(|step| *step == SpellUnlearnStep::Talent(20)).unwrap();
    assert_eq!(trace[talent + 1], SpellUnlearnStep::Known(20));
    assert!(!trace.contains(&SpellUnlearnStep::NextRank(20)));
    assert_eq!(runtime.removed_known_spells_like_cpp(), &BTreeSet::from([10]));
}

#[test]
fn unlearn_failed_trait_writer_skips_lookup_but_retains_cleanup_and_notice() {
    let mut runtime = runtime(vec![10]);
    runtime.set_trait_definition_id_like_cpp(10, Some(7));
    let catalog = Catalog { traits: BTreeMap::from([(7, 20)]), ..Default::default() };
    let trace = drive(&mut runtime, &catalog, 10, false,
        &BTreeSet::new(), &BTreeSet::from([10]));
    assert!(!trace.contains(&SpellUnlearnStep::TraitOverride(7)));
    assert_eq!(runtime.trait_definition_ids_like_cpp().get(&10), Some(&7));
    assert!(trace.ends_with(&[
        SpellUnlearnStep::TitanGrip(10), SpellUnlearnStep::DualWield(10),
        SpellUnlearnStep::Offhand,
        SpellUnlearnStep::Unlearned { spell_id: 10, suppress_messaging: false },
        SpellUnlearnStep::Done,
    ]));
}

#[test]
fn unlearn_complete_rows_keep_new_tombstone_gap_and_erase_dependent_row() {
    let mut runtime = runtime(vec![10, 20]);
    runtime.mark_known_spell_dependent_like_cpp(20);
    runtime.replace_rows_like_cpp(BTreeMap::from([
        (10, PlayerKnownSpellRecord {
            spell_id: 10, state: PlayerSpellLoadState::New, active: true,
            disabled: true, dependent: false, favorite: true,
        }),
        (20, PlayerKnownSpellRecord {
            spell_id: 20, state: PlayerSpellLoadState::Unchanged, active: true,
            disabled: false, dependent: true, favorite: false,
        }),
    ]), true);
    let catalog = Catalog { requiring: BTreeMap::from([(10, vec![20])]), ..Default::default() };
    let trace = remove(&mut runtime, &catalog, 10, false);
    assert!(!trace.contains(&SpellUnlearnStep::InvalidateRows));
    assert_eq!(runtime.rows_like_cpp().get(&10), Some(&PlayerKnownSpellRecord {
        spell_id: 10, state: PlayerSpellLoadState::Removed, active: false,
        disabled: false, dependent: false, favorite: false,
    }), "the current Rust representation tombstones even a New independent row");
    assert!(!runtime.rows_like_cpp().contains_key(&20));
    assert_eq!(runtime.removed_known_spells_like_cpp(), &BTreeSet::from([10]));
    assert!(runtime.rows_complete_like_cpp());
}

#[test]
fn unlearn_invalid_signed_spell_and_learned_ids_keep_original_conversion_gates() {
    let mut runtime = runtime(vec![-1, 10]);
    let trace = remove(&mut runtime, &Catalog::default(), -1, false);
    assert!(!trace.iter().any(|step| matches!(step, SpellUnlearnStep::NextRank(_))));
    assert!(!trace.iter().any(|step| matches!(step, SpellUnlearnStep::Unlearned { .. })));
    assert_eq!(runtime.known_spells_like_cpp(), &[10]);
    let catalog = Catalog {
        learned: BTreeMap::from([(10, vec![SpellUnlearnEdge {
            spell_id: u32::MAX, overrides_spell_id: 100,
        }])]),
        ..Default::default()
    };
    let trace = remove(&mut runtime, &catalog, 10, false);
    assert!(!trace.contains(&SpellUnlearnStep::Known(-1)));
    assert!(!trace.iter().any(|step| matches!(step, SpellUnlearnStep::RemoveOverride { .. })));
}

#[test]
fn unlearn_retains_existing_snapshot_iterator_and_reads_each_edge_after_skill() {
    use std::{cell::Cell, rc::Rc};
    let polls = Rc::new(Cell::new(0));
    let mut operation = SpellUnlearnOperation::new(10, false);
    operation.advance(SpellUnlearnInput::Known(true));
    operation.advance(SpellUnlearnInput::RowsComplete(true));
    assert_eq!(operation.step(), SpellUnlearnStep::NextRank(10));
    operation.advance(SpellUnlearnInput::Rank(0));
    operation.advance(SpellUnlearnInput::Requiring(Vec::new()));
    assert_eq!(operation.step(), SpellUnlearnStep::Owner(SpellUnlearnOwnerStep::Forget {
        spell_id: 10, preserve_complete: true,
    }));
    operation.advance(SpellUnlearnInput::Owner(Some(SpellUnlearnOwnerOutcome::Forgotten {
        was_dependent: false,
    })));
    assert_eq!(operation.step(), SpellUnlearnStep::DowngradeSkill(10));
    operation.advance(SpellUnlearnInput::Applied);
    assert_eq!(operation.step(), SpellUnlearnStep::Learned(10));
    let observed = Rc::clone(&polls);
    operation.advance(SpellUnlearnInput::Learned(vec![SpellUnlearnEdge {
        spell_id: 20, overrides_spell_id: 100,
    }].into_iter().map(move |edge| {
        observed.set(observed.get() + 1);
        edge
    })));
    assert_eq!(polls.get(), 0);
    assert_eq!(operation.step(), SpellUnlearnStep::Known(20));
    assert_eq!(polls.get(), 1);
    assert_eq!(operation.step(), SpellUnlearnStep::Known(20), "an outstanding read remains stable");
}
