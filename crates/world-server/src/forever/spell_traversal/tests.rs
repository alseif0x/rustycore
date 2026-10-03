//! Synthetic ABI and production-composition checks, not native client evidence.
use super::*;
use std::{collections::BTreeSet, sync::Arc};
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};
use wow_persistence::forever::spells::server::ServerSpellRows;
use wow_world::forever::spells::SpellLoadPlan;

#[test]
fn abi_layout_matches_cpp_without_packed_fields_or_serializing_padding() {
    assert_eq!(size_of::<Key>(), 8);
    assert_eq!(align_of::<Key>(), 4);
    assert_eq!(std::mem::offset_of!(Key, difficulty), 4);
}

#[test]
fn empty_replay_is_valid_but_an_incorrect_capacity_is_not() {
    let orders = replay(&[], [], &[], 0).unwrap();
    assert!(orders.primary.is_empty());
    assert!(orders.secondary.is_empty());
    assert!(replay(&[], [], &[], 1).is_err());
    assert!(replay(&[], [], &[(1, 0)], 0).is_err());
}

#[test]
fn signed_difficulties_unnamed_helpers_and_duplicate_server_attempts_retain_exact_sets() {
    let orders = replay(
        &[(3, -1), (7, 0), (3, i16::MIN), (2, i16::MAX)],
        [(3, -1), (3, i16::MIN), (2, i16::MAX)],
        &[(9, 0), (9, 1), (9, 0), (5, -7)],
        6,
    )
    .unwrap();
    let expected: BTreeSet<(u32, i16)> = BTreeSet::from([
        (3, -1),
        (3, i16::MIN),
        (2, i16::MAX),
        (9, 0),
        (9, 1),
        (5, -7),
    ]);
    for order in [orders.primary, orders.secondary] {
        assert_eq!(order.len(), expected.len());
        assert_eq!(
            order
                .into_iter()
                .map(Into::into)
                .collect::<BTreeSet<(u32, i16)>>(),
            expected
        );
    }
}

#[test]
fn impossible_history_membership_collision_and_count_fail_closed() {
    assert!(replay(&[(1, 0), (1, 0)], [(1, 0)], &[], 1).is_err());
    assert!(replay(&[(1, 0)], [(2, 0)], &[], 1).is_err());
    assert!(replay(&[(1, 0)], [(1, 0), (1, 0)], &[], 1).is_err());
    // Collision is per client SpellID, not only exact difficulty.
    assert!(replay(&[(1, 0)], [(1, 0)], &[(1, 9)], 2).is_err());
    assert!(replay(&[(1, 0)], [(1, 0)], &[(2, 0)], 1).is_err());
    assert!(replay(&[(1, 0)], [(1, 0)], &[(2, 0)], 3).is_err());
}

#[test]
fn independent_replays_are_stateless_deterministic_and_can_run_concurrently() {
    let helpers: Vec<_> = (1..=200).rev().map(|id| (id, id as i16 - 100)).collect();
    let clients: Vec<_> = helpers
        .iter()
        .copied()
        .filter(|key| key.0 % 3 != 0)
        .collect();
    let count = clients.len() + 3;
    let server = [(201, -1), (201, 0), (201, 1), (201, 0)];
    let expected = replay(&helpers, clients.iter().copied(), &server, count).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let helpers = &helpers;
            let clients = &clients;
            let expected = &expected;
            scope.spawn(move || {
                let actual = replay(helpers, clients.iter().copied(), &server, count).unwrap();
                assert_eq!(actual.primary, expected.primary);
                assert_eq!(actual.secondary, expected.secondary);
            });
        }
    });
}

fn misc(id: u32) -> SpellMiscRecord {
    SpellMiscRecord {
        id,
        attributes: [0; 17],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        pv_p_duration_index: 0,
        range_index: 0,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 135754,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_visual_script: 0,
        active_spell_visual_script: 0,
        spell_id: id,
    }
}
fn input() -> SpellDefinitionSeeds {
    let rows = SpellRecords {
        spell_names: [9, 2, 5]
            .map(|id| SpellNameRecord {
                id,
                name: SpellText::default(),
            })
            .to_vec(),
        spell_misc: [9, 2, 5, 77].map(misc).to_vec(),
        summon_properties: [121, 647, 628]
            .map(|id| SummonPropertiesRecord {
                id,
                control: -9,
                faction: 8,
                title: 99,
                slot: 5,
                flags: [i32::MIN, -1],
            })
            .to_vec(),
        ..Default::default()
    };
    let catalog = rows
        .finish(
            SpellRecords::default(),
            SpellRecords::default(),
            6,
            SpellLocaleRecords::default(),
            SpellLocaleRecords::default(),
            &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
        )
        .unwrap();
    SpellLoadPlan::build(Arc::new(catalog))
        .unwrap()
        .with_server_spells(ServerSpellRows::default())
        .unwrap()
        .with_id_corrections()
        .unwrap()
}

#[test]
fn production_join_native_replay_and_global_phase_finish_before_raw_publication() {
    let input = input();
    assert_eq!(input.client_counts().unnamed_helpers, 1);
    let expected = {
        let inputs = input.traversal_inputs().unwrap();
        replay(
            inputs.helper_insertions(),
            inputs.client_keys(),
            inputs.server_requests(),
            input.len(),
        )
        .unwrap()
    };
    let result = correct(input).unwrap();
    assert!(result.traversal_inputs().is_none());
    assert_eq!(result.len(), 3);
    let counts = result.global_correction_counts().unwrap();
    assert_eq!(counts.definitions, 3);
    assert_eq!(counts.passive_flight_flags, 3);
    assert_eq!(counts.summon_properties_applied, 3);
    assert_eq!(
        result
            .records()
            .map(|view| (view.spell_id(), view.difficulty()))
            .collect::<Vec<_>>(),
        expected
            .primary
            .into_iter()
            .map(Into::into)
            .collect::<Vec<(u32, i16)>>()
    );
    for id in [9, 2, 5] {
        assert_eq!(
            result.get_exact(id, 0).unwrap().fields().attributes[0],
            0x40
        );
        assert_eq!(
            result
                .corrected_difficulties(id)
                .unwrap()
                .map(|view| view.difficulty())
                .collect::<Vec<_>>(),
            expected
                .secondary
                .iter()
                .filter(|key| key.id == id)
                .map(|key| key.difficulty)
                .collect::<Vec<_>>()
        );
    }
    let raw = result.raw_catalog();
    assert_eq!(raw.summon_properties(121).unwrap().title, 4);
    assert_eq!(raw.summon_properties(647).unwrap().title, 4);
    assert_eq!(raw.summon_properties(628).unwrap().control, 2);
}

#[test]
fn premature_raw_reader_prevents_global_mutation_without_copy_on_write() {
    let input = input();
    let raw = input.raw_catalog();
    assert!(correct(input).is_err());
    assert_eq!(raw.summon_properties(121).unwrap().title, 99);
    assert_eq!(raw.summon_properties(628).unwrap().control, -9);
}

#[test]
fn retired_input_cannot_be_replayed_or_corrected_twice() {
    let result = correct(input()).unwrap();
    assert!(correct(result).is_err());
}
