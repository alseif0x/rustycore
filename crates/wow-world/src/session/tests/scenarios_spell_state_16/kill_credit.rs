use super::*;

#[tokio::test]
async fn spell_kill_credit_effect_row_rewards_player_monster_objective_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 740_i32;
    let player_guid = ObjectGuid::create_player(1, 57);
    let quest_id = 12_540;
    let creature_entry = 9_940;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 0, // C++ QUEST_OBJECTIVE_MONSTER.
        order: 0,
        storage_index: 0,
        object_id: creature_entry as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.quest_test_fixture_like_cpp.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT,
                effect_misc_value_1: creature_entry as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented kill-credit spell row should execute");

    assert!(
        !session
            .quest_test_fixture_like_cpp
            .player_quests
            .contains_key(&quest_id)
    );
    assert!(
        session
            .quest_test_fixture_like_cpp
            .rewarded_quests
            .contains(&quest_id)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::CooldownEvent,
        ]
    );
}

#[tokio::test]
async fn spell_kill_credit2_effect_row_rewards_current_session_like_cpp_without_group_fanout() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 741_i32;
    let player_guid = ObjectGuid::create_player(1, 58);
    let quest_id = 12_541;
    let creature_entry = 9_941;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 0, // C++ QUEST_OBJECTIVE_MONSTER.
        order: 0,
        storage_index: 0,
        object_id: creature_entry as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.quest_test_fixture_like_cpp.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT2,
                effect_misc_value_1: creature_entry as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented kill-credit2 spell row should execute");

    assert!(
        !session
            .quest_test_fixture_like_cpp
            .player_quests
            .contains_key(&quest_id)
    );
    assert!(
        session
            .quest_test_fixture_like_cpp
            .rewarded_quests
            .contains(&quest_id)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
