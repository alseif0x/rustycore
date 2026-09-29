use std::collections::{BTreeMap, HashMap};

use super::{
    SKILL_FIST_WEAPONS_LIKE_CPP, SKILL_UNARMED_LIKE_CPP,
    sync_loaded_fist_weapons_with_unarmed_like_cpp,
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
