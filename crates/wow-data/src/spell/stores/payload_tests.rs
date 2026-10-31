use super::*;
use crate::spell::SpellInfoPayloadResolutionErrorLikeCpp;
use crate::spell_db2::{
    SpellCastTimesEntry, SpellCastTimesStore, SpellCooldownsEntry, SpellCooldownsStore,
    SpellMiscEntry, SpellMiscStore, SpellPowerDifficultyEntry, SpellPowerDifficultyStore,
    SpellPowerEntry, SpellPowerStore,
};
use std::collections::HashSet;

fn difficulty_chain_3_to_0() -> crate::DifficultyStore {
    crate::DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 0,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])
}

fn payload_spell_store(
    exact_keys: &[(u32, u8)],
    misc_rows: Vec<SpellMiscEntry>,
    cast_times: Vec<SpellCastTimesEntry>,
    cooldowns: Vec<SpellCooldownsEntry>,
    powers: Vec<SpellPowerEntry>,
    power_difficulties: Vec<SpellPowerDifficultyEntry>,
) -> SpellStore {
    let mut store = SpellStore::new();
    let spell_ids = exact_keys
        .iter()
        .map(|(spell_id, _)| *spell_id)
        .collect::<HashSet<_>>();
    store.spell_info_keys_like_cpp =
        crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
            exact_keys.iter().copied(),
            &spell_ids,
        );
    let misc = SpellMiscStore::from_entries(misc_rows);
    let cast_times = SpellCastTimesStore::from_entries(cast_times);
    store.apply_db2_cast_times_like_cpp(&misc, &cast_times);
    store.apply_db2_cooldowns_like_cpp(&SpellCooldownsStore::from_entries(cooldowns));
    store.apply_db2_power_costs_like_cpp(
        &SpellPowerStore::from_entries(powers),
        &SpellPowerDifficultyStore::from_entries(power_difficulties),
    );
    store
}

fn misc(spell_id: u32, difficulty_id: u8, casting_time_index: u16) -> SpellMiscEntry {
    SpellMiscEntry {
        id: u32::from(difficulty_id) + 1,
        spell_id,
        difficulty_id,
        casting_time_index,
        ..SpellMiscEntry::default()
    }
}

fn cooldown(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    start_recovery_time: i32,
    recovery_time: i32,
) -> SpellCooldownsEntry {
    cooldown_with_category(
        id,
        spell_id,
        difficulty_id,
        start_recovery_time,
        recovery_time,
        recovery_time,
    )
}

fn cooldown_with_category(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    start_recovery_time: i32,
    recovery_time: i32,
    category_recovery_time: i32,
) -> SpellCooldownsEntry {
    SpellCooldownsEntry {
        id,
        difficulty_id,
        category_recovery_time,
        recovery_time,
        start_recovery_time,
        spell_id,
    }
}

fn power(id: u32, spell_id: u32, order_index: u8, mana_cost: i32) -> SpellPowerEntry {
    SpellPowerEntry {
        id,
        order_index,
        mana_cost,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_display_id: 0,
        alt_power_bar_id: 0,
        power_cost_pct: 0.0,
        power_cost_max_pct: 0.0,
        power_pct_per_second: 0.0,
        power_type: 0,
        required_aura_spell_id: 0,
        optional_cost: 0,
        spell_id,
    }
}

fn power_difficulty(id: u32, difficulty_id: u8, order_index: u8) -> SpellPowerDifficultyEntry {
    SpellPowerDifficultyEntry {
        id,
        difficulty_id,
        order_index,
    }
}

#[test]
fn actual_info3_key_uses_its_own_sparse_payload_without_info0_rows() {
    let spell_id = 71_101;
    let store = payload_spell_store(
        &[(spell_id, 3)],
        vec![misc(spell_id, 3, 1)],
        vec![SpellCastTimesEntry {
            id: 1,
            base: 1_400,
            per_level: 0,
            minimum: 800,
        }],
        vec![cooldown(1, spell_id, 3, 450, 900)],
        vec![power(1, spell_id, 0, 21)],
        vec![power_difficulty(1, 3, 0)],
    );
    assert!(store.contains_spell_info_exact_like_cpp(spell_id, 3));
    assert!(!store.contains_spell_info_exact_like_cpp(spell_id, 0));
    assert!(store.get(spell_id as i32).is_none());

    let info = store
        .spell_info_for_exact_key_like_cpp(spell_id as i32, 3, Some(&difficulty_chain_3_to_0()))
        .unwrap()
        .unwrap();
    assert_eq!(info.cast_time_ms, 1_400);
    assert_eq!(info.cooldown_ms, 450);
    assert_eq!(info.recovery_time_ms, 900);
    assert_eq!(
        store
            .cooldown_components_at_difficulty_like_cpp(
                spell_id as i32,
                3,
                Some(&difficulty_chain_3_to_0()),
            )
            .unwrap(),
        (450, 900, 900)
    );
    assert_eq!(info.power_costs.len(), 1);
    assert_eq!(info.power_costs[0].order_index, 0);
    assert_eq!(info.power_costs[0].mana_cost, 21);
}

#[test]
fn actual_info3_inherits_cooldown_and_sparse_power_orders_without_base_payload() {
    let spell_id = 71_102;
    let store = payload_spell_store(
        &[(spell_id, 0), (spell_id, 3)],
        vec![misc(spell_id, 3, 1)],
        vec![SpellCastTimesEntry {
            id: 1,
            base: 2_000,
            per_level: 0,
            minimum: 0,
        }],
        vec![cooldown(1, spell_id, 0, 350, 700)],
        vec![power(1, spell_id, 0, 33), power(2, spell_id, 1, 44)],
        vec![power_difficulty(1, 0, 0), power_difficulty(2, 3, 1)],
    );
    assert!(store.contains_spell_info_exact_like_cpp(spell_id, 0));
    assert!(store.get(spell_id as i32).is_none());

    let info = store
        .spell_info_for_exact_key_like_cpp(spell_id as i32, 3, Some(&difficulty_chain_3_to_0()))
        .unwrap()
        .unwrap();
    assert_eq!(info.cast_time_ms, 2_000);
    assert_eq!(info.cooldown_ms, 350);
    assert_eq!(info.recovery_time_ms, 700);
    assert_eq!(
        store
            .cooldown_components_at_difficulty_like_cpp(
                spell_id as i32,
                3,
                Some(&difficulty_chain_3_to_0()),
            )
            .unwrap(),
        (350, 700, 700)
    );
    assert_eq!(
        info.power_costs
            .iter()
            .map(|cost| (cost.order_index, cost.mana_cost))
            .collect::<Vec<_>>(),
        vec![(0, 33), (1, 44)]
    );
}

#[test]
fn cooldown_components_keep_spell_and_category_recovery_separate() {
    let spell_id = 71_106;
    let store = payload_spell_store(
        &[(spell_id, 3)],
        vec![misc(spell_id, 3, 0)],
        vec![],
        vec![cooldown_with_category(1, spell_id, 3, 0, 700, 1_100)],
        vec![],
        vec![],
    );
    let difficulties = difficulty_chain_3_to_0();
    let info = store
        .spell_info_for_exact_key_like_cpp(spell_id as i32, 3, Some(&difficulties))
        .unwrap()
        .unwrap();
    assert_eq!(info.cooldown_ms, 0);
    assert_eq!(info.recovery_time_ms, 1_100);
    assert_eq!(
        store
            .cooldown_components_at_difficulty_like_cpp(spell_id as i32, 3, Some(&difficulties),)
            .unwrap(),
        (0, 700, 1_100)
    );
}

#[test]
fn explicit_child_zero_cooldown_and_power_rows_override_parent_values() {
    let spell_id = 71_103;
    let store = payload_spell_store(
        &[(spell_id, 0), (spell_id, 3)],
        vec![misc(spell_id, 3, 0)],
        vec![],
        vec![
            cooldown(1, spell_id, 0, 500, 900),
            cooldown(2, spell_id, 3, 0, 0),
        ],
        vec![power(1, spell_id, 0, 75), power(2, spell_id, 0, 0)],
        vec![power_difficulty(1, 0, 0), power_difficulty(2, 3, 0)],
    );
    let info = store
        .spell_info_for_exact_key_like_cpp(spell_id as i32, 3, Some(&difficulty_chain_3_to_0()))
        .unwrap()
        .unwrap();
    assert_eq!(info.cast_time_ms, 0);
    assert_eq!(info.cooldown_ms, 0);
    assert_eq!(info.recovery_time_ms, 0);
    assert_eq!(info.power_costs.len(), 1);
    assert_eq!(info.power_costs[0].mana_cost, 0);
}

#[test]
fn selected_misc_null_cast_time_blocks_parent_and_index_zero_uses_real_lookup() {
    let spell_id = 71_105;
    let difficulties = difficulty_chain_3_to_0();
    let missing_child_entry = payload_spell_store(
        &[(spell_id, 0), (spell_id, 3)],
        vec![misc(spell_id, 0, 2), misc(spell_id, 3, 7)],
        vec![SpellCastTimesEntry {
            id: 2,
            base: 1_800,
            per_level: 0,
            minimum: 0,
        }],
        vec![],
        vec![],
        vec![],
    );
    let info = missing_child_entry
        .spell_info_for_exact_key_like_cpp(spell_id as i32, 3, Some(&difficulties))
        .unwrap()
        .unwrap();
    assert_eq!(info.cast_time_ms, 0);

    let indexed_zero_entry = payload_spell_store(
        &[(spell_id, 0)],
        vec![misc(spell_id, 0, 0)],
        vec![SpellCastTimesEntry {
            id: 0,
            base: 620,
            per_level: 0,
            minimum: 300,
        }],
        vec![],
        vec![],
        vec![],
    );
    let info = indexed_zero_entry
        .spell_info_for_exact_key_like_cpp(spell_id as i32, 0, None)
        .unwrap()
        .unwrap();
    assert_eq!(info.cast_time_ms, 620);
}

#[test]
fn cyclic_incomplete_payload_difficulty_chain_is_not_reported_as_free() {
    let spell_id = 71_104;
    let store = payload_spell_store(&[(spell_id, 3)], vec![], vec![], vec![], vec![], vec![]);
    let cyclic = crate::DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 4,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 4,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 3,
            toggle_difficulty_id: 0,
        },
    ]);

    assert!(matches!(
        store.spell_info_for_exact_key_like_cpp(spell_id as i32, 3, Some(&cyclic)),
        Err(SpellInfoPayloadResolutionErrorLikeCpp::Catalog(
            crate::spell::DataSpellCatalogResolutionError::DifficultyCycle
        ))
    ));
}
