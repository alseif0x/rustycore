use std::collections::{BTreeMap, HashMap, HashSet};

use super::{
    SKILL_FIST_WEAPONS_LIKE_CPP, SKILL_UNARMED_LIKE_CPP, active_known_spell_for_send_like_cpp,
    apply_skill_rewarded_spell_changes_to_login_like_cpp, favorite_known_spells_for_send_like_cpp,
    loaded_spell_for_add_spell_side_effects_like_cpp, spell_charge_entry_from_db_like_cpp,
    spell_history_entry_from_db_like_cpp, sync_loaded_fist_weapons_with_unarmed_like_cpp,
};

#[test]
fn loaded_fist_weapons_mirrors_unarmed_after_all_skill_rows_like_cpp() {
    fn skill_info(skill_id: u16, rank: u16, max_rank: u16) -> wow_data::SkillInfoEntry {
        wow_data::SkillInfoEntry {
            skill_id,
            step: 0,
            rank,
            starting_rank: 1,
            max_rank,
            temp_bonus: 0,
            perm_bonus: 0,
        }
    }

    let mut records = HashMap::from([
        (
            SKILL_UNARMED_LIKE_CPP,
            crate::session::RepresentedPlayerSkillLikeCpp {
                skill_id: SKILL_UNARMED_LIKE_CPP,
                step: 0,
                value: 37,
                max: 400,
                profession_slot: -1,
                state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        ),
        (
            SKILL_FIST_WEAPONS_LIKE_CPP,
            crate::session::RepresentedPlayerSkillLikeCpp {
                skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
                step: 0,
                value: 12,
                max: 400,
                profession_slot: -1,
                state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        ),
    ]);
    let mut skill_info_by_id = BTreeMap::from([
        (
            SKILL_UNARMED_LIKE_CPP,
            skill_info(SKILL_UNARMED_LIKE_CPP, 37, 400),
        ),
        (
            SKILL_FIST_WEAPONS_LIKE_CPP,
            skill_info(SKILL_FIST_WEAPONS_LIKE_CPP, 12, 400),
        ),
    ]);

    sync_loaded_fist_weapons_with_unarmed_like_cpp(&mut records, &mut skill_info_by_id, 80);

    assert_eq!(
        skill_info_by_id
            .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
            .expect("loaded Fist Weapons slot")
            .rank,
        37
    );
    assert_eq!(
        records
            .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
            .expect("active persisted Fist Weapons")
            .value,
        37
    );
}

#[test]
fn loaded_fist_weapons_without_unarmed_is_cleared_like_cpp_set_skill_zero() {
    let mut records = HashMap::from([(
        SKILL_FIST_WEAPONS_LIKE_CPP,
        crate::session::RepresentedPlayerSkillLikeCpp {
            skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
            step: 0,
            value: 12,
            max: 400,
            profession_slot: -1,
            state: crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]);
    let mut skill_info_by_id = BTreeMap::from([(
        SKILL_FIST_WEAPONS_LIKE_CPP,
        wow_data::SkillInfoEntry {
            skill_id: SKILL_FIST_WEAPONS_LIKE_CPP,
            step: 0,
            rank: 12,
            starting_rank: 1,
            max_rank: 400,
            temp_bonus: 0,
            perm_bonus: 0,
        },
    )]);

    sync_loaded_fist_weapons_with_unarmed_like_cpp(&mut records, &mut skill_info_by_id, 80);

    assert!(
        !records.contains_key(&SKILL_FIST_WEAPONS_LIKE_CPP),
        "C++ marks the persisted skill deleted"
    );
    let cleared = skill_info_by_id
        .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
        .expect("C++ retains the cleared initial update-field slot");
    assert_eq!(cleared.rank, 0);
    assert_eq!(cleared.max_rank, 0);
}

#[test]
fn send_known_spells_filters_disabled_and_inactive_like_cpp() {
    assert_eq!(active_known_spell_for_send_like_cpp(118, 1, 0), Some(118));
    assert_eq!(
        active_known_spell_for_send_like_cpp(118, 0, 0),
        None,
        "C++ Player::SendKnownSpells skips inactive spells"
    );
    assert_eq!(
        active_known_spell_for_send_like_cpp(118, 1, 1),
        None,
        "C++ Player::SendKnownSpells skips disabled spells"
    );
    assert_eq!(active_known_spell_for_send_like_cpp(0, 1, 0), None);
}

#[test]
fn load_spells_keeps_inactive_non_disabled_spells_for_add_spell_side_effects_like_cpp() {
    assert_eq!(
        loaded_spell_for_add_spell_side_effects_like_cpp(118, 0),
        Some(118),
        "C++ Player::_LoadSpells still calls AddSpell for inactive rows; SendKnownSpells filters them later"
    );
    assert_eq!(
        loaded_spell_for_add_spell_side_effects_like_cpp(118, 1),
        None,
        "C++ AddSpell returns before cast side effects for disabled spell rows"
    );
    assert_eq!(loaded_spell_for_add_spell_side_effects_like_cpp(0, 0), None);
}

#[test]
fn login_skill_reward_spells_retain_cpp_dependent_ownership() {
    let mut known = vec![100, 200];
    let mut side_effects = vec![100, 200];
    let mut dependent = HashSet::from([200]);
    let mut removed = HashSet::new();

    apply_skill_rewarded_spell_changes_to_login_like_cpp(
        &mut known,
        &mut side_effects,
        &mut dependent,
        &mut removed,
        wow_data::SkillRewardedSpellChangesLikeCpp {
            learn: vec![300],
            remove: vec![200],
        },
    );

    assert_eq!(known, vec![100, 300]);
    assert_eq!(side_effects, vec![100, 300]);
    assert_eq!(dependent, HashSet::from([300]));
    assert_eq!(removed, HashSet::from([200]));
}

#[test]
fn send_known_spells_favorites_are_subset_of_sent_spells_like_cpp() {
    let favorites = HashSet::from([635, 999]);

    assert_eq!(
        favorite_known_spells_for_send_like_cpp(&[118, 635, 133], &favorites),
        vec![635],
        "C++ only marks favorite spells while iterating spells that are actually sent"
    );
}

#[test]
fn spell_history_entry_from_db_splits_spell_and_category_cooldowns_like_cpp() {
    let entry = spell_history_entry_from_db_like_cpp(133, 6948, 1_030, 12, 1_010, 1_000)
        .expect("future cooldown should be serialized");

    assert_eq!(entry.spell_id, 133);
    assert_eq!(entry.item_id, 6948);
    assert_eq!(entry.category, 12);
    assert_eq!(entry.recovery_time_ms, 30_000);
    assert_eq!(entry.category_recovery_time_ms, 10_000);
    assert_eq!(entry.mod_rate, 1.0);
    assert!(!entry.on_hold);
}

#[test]
fn spell_history_entry_omits_recovery_when_category_last_longer_like_cpp() {
    let entry = spell_history_entry_from_db_like_cpp(133, 0, 1_005, 12, 1_010, 1_000)
        .expect("future category cooldown should be serialized");

    assert_eq!(entry.category, 12);
    assert_eq!(entry.recovery_time_ms, 0);
    assert_eq!(entry.category_recovery_time_ms, 10_000);
}

#[test]
fn spell_history_entry_skips_expired_cooldowns_like_cpp() {
    assert_eq!(
        spell_history_entry_from_db_like_cpp(133, 0, 1_000, 12, 1_010, 1_000),
        None
    );
}

#[test]
fn spell_charge_entry_uses_first_recharge_and_consumed_count_like_cpp() {
    let entry = spell_charge_entry_from_db_like_cpp(42, 1_045, 2, 1_000)
        .expect("future charge should be serialized");

    assert_eq!(entry.category, 42);
    assert_eq!(entry.next_recovery_time_ms, 45_000);
    assert_eq!(entry.charge_mod_rate, 1.0);
    assert_eq!(entry.consumed_charges, 2);
}

#[test]
fn spell_charge_entry_skips_expired_recharges_like_cpp() {
    assert_eq!(
        spell_charge_entry_from_db_like_cpp(42, 1_000, 1, 1_000),
        None
    );
}
