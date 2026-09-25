use super::*;

fn spell_area_store_for_authority_like_cpp(
    rows: impl IntoIterator<Item = wow_data::SpellAreaRowLikeCpp>,
) -> SpellAreaStoreLikeCpp {
    SpellAreaStoreLikeCpp::from_rows_like_cpp(rows, |_| true, |_| true, |_| true).store
}

#[test]
fn player_spell_hit_source_authority_evaluates_spell_area_quest_requirements_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [test_quest_template(10_045)],
    )));
    session.set_player_zone_area_like_cpp(3_518, 3_697);
    session.set_player_zone_area_authority_complete_like_cpp(true);
    session.complete_player_quest_status_authority_load_like_cpp();
    session.set_spell_area_store(Arc::new(spell_area_store_for_authority_like_cpp([
        wow_data::SpellAreaRowLikeCpp {
            spell_id: 32_649,
            area_id: 3_518,
            quest_start: 10_045,
            quest_start_status: 1 << crate::conditions::QUEST_STATUS_REWARDED_LIKE_CPP,
            quest_end_status: 0,
            quest_end: 0,
            aura_spell: 0,
            race_mask: 0,
            gender: wow_data::GENDER_NONE_LIKE_CPP,
            flags: SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
        },
    ])));
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "the issue #26 zone aura cannot fit while quest 10045 is exactly not rewarded"
    );

    session
        .quest_test_fixture_like_cpp
        .rewarded_quests
        .insert(10_045);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "the same C++ spell_area row can autocast after its rewarded requirement fits"
    );

    session.quest_test_fixture_like_cpp.rewarded_quests.clear();
    session.invalidate_player_quest_status_authority_like_cpp();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "missing quest-source authority must fail closed"
    );

    session.complete_player_quest_status_authority_load_like_cpp();
    session.set_player_zone_area_authority_complete_like_cpp(false);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a DB-seeded or zero terrain location must not prove the row out of scope"
    );
}

#[test]
fn player_spell_hit_source_authority_evaluates_spell_area_aura_requirement_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    let row = wow_data::SpellAreaRowLikeCpp {
        spell_id: 32_649,
        area_id: 0,
        quest_start: 0,
        quest_start_status: 0,
        quest_end_status: 0,
        quest_end: 0,
        aura_spell: 33_795,
        race_mask: 0,
        gender: wow_data::GENDER_NONE_LIKE_CPP,
        flags: SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
    };
    session.set_spell_area_store(Arc::new(spell_area_store_for_authority_like_cpp([row])));
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a missing positive aura prerequisite proves the SpellArea row cannot fit"
    );

    session
        .visible_auras
        .insert(0, test_visible_aura(0, 33_795));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a present positive prerequisite can make the AUTOCAST row fit"
    );

    session.set_spell_area_store(Arc::new(spell_area_store_for_authority_like_cpp([
        wow_data::SpellAreaRowLikeCpp {
            aura_spell: -33_795,
            ..row
        },
    ])));
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a present aura disproves a negative C++ SpellArea prerequisite"
    );

    session.visible_auras.clear();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an absent aura makes the negative prerequisite fit"
    );
}
