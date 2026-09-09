//! Talent handlers regression scenarios, part 1 of 2.
//!
//! Moved out of the talent.rs root under #662; every test is unchanged.

use super::*;

#[tokio::test]
async fn confirm_respec_wipe_records_talent_reset_with_trainer_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let trainer = test_creature_guid(77);
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
    session.mark_represented_talents_loaded_like_cpp();
    session.set_player_gold_like_cpp(20_000);

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    let reset_update = send_rx
        .try_recv()
        .expect("C++ sends SendTalentsInfoData after successful ResetTalents");
    assert_eq!(
        reset_update,
        session
            .represented_update_talent_data_packet_like_cpp()
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.player_gold_like_cpp(), 10_000);
    assert_eq!(
        session.represented_talent_reset_cost_like_cpp(),
        Some(10_000)
    );
    assert_ne!(
        session.represented_talent_reset_time_secs_like_cpp(),
        Some(0)
    );
    assert_eq!(
        session.represented_talent_respec_criteria_events_like_cpp(),
        &[
            RepresentedTalentRespecCriteriaEventLikeCpp::MoneySpentOnRespecs { amount: 10_000 },
            RepresentedTalentRespecCriteriaEventLikeCpp::TotalRespecs { quantity: 1 },
        ],
        "C++ Player::ResetTalents updates money-spent and total-respec criteria after ModifyMoney"
    );
    assert_eq!(
        session.represented_talent_reset_script_hooks_like_cpp(),
        &[RepresentedTalentResetScriptHookLikeCpp { no_cost: false }],
        "C++ Player::ResetTalents starts with sScriptMgr->OnPlayerTalentsReset(this, noCost)"
    );
    assert!(
        session
            .represented_at_login_flag_removals_like_cpp()
            .is_empty(),
        "C++ only calls RemoveAtLoginFlag when AT_LOGIN_RESET_TALENTS is present"
    );
    assert_eq!(
        session.represented_confirm_respec_wipe_requests_like_cpp(),
        &[RepresentedConfirmRespecWipeLikeCpp {
            respec_master: trainer,
            respec_type: SPEC_RESET_TALENTS_LIKE_CPP,
        }]
    );
    assert_eq!(
        session.represented_talent_respec_visual_spell_casts_like_cpp(),
        &[RepresentedTalentRespecVisualSpellCastLikeCpp {
            caster_guid: trainer,
            target_guid: ObjectGuid::create_player(1, 42),
            spell_id: UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP,
            triggered: true,
            spell_runtime_unrepresented: true,
        }],
        "C++ casts unit->CastSpell(_player, 14867, true) after SendTalentsInfoData"
    );
}

#[tokio::test]
async fn confirm_respec_wipe_respects_no_reset_talent_cost_config_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let trainer = test_creature_guid(78);
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
    session.mark_represented_talents_loaded_like_cpp();
    session.set_player_gold_like_cpp(0);
    session.set_represented_talent_reset_state_like_cpp(500_000, 123);

    let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    let entry = crate::session::registry::get_handler(ClientOpcodes::ConfirmRespecWipe).unwrap();
    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    Arc::get_mut(&mut catalogs.progression)
        .unwrap()
        .no_reset_talent_cost = true;
    let references = Arc::strong_count(&catalogs.progression);
    (entry.handler)(
        &mut session,
        &catalogs,
        confirm_respec_wipe_packet(trainer, SPEC_RESET_TALENTS_LIKE_CPP),
    )
    .await;
    assert_eq!(Arc::strong_count(&catalogs.progression), references);

    let reset_update = send_rx
        .try_recv()
        .expect("C++ ResetTalents skips money checks when CONFIG_NO_RESET_TALENT_COST is true");
    assert_eq!(
        reset_update,
        session
            .represented_update_talent_data_packet_like_cpp()
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.player_gold_like_cpp(),
        0,
        "C++ no-cost reset does not modify player money"
    );
    assert_eq!(
        session.represented_talent_reset_cost_like_cpp(),
        Some(0),
        "C++ SetTalentResetCost receives the zero cost when NoResetTalentsCost bypasses the money gate"
    );
    assert_ne!(
        session.represented_talent_reset_time_secs_like_cpp(),
        Some(123),
        "C++ still updates TalentResetTime inside the final !noCost block"
    );
    assert_eq!(
        session.represented_talent_respec_criteria_events_like_cpp(),
        &[
            RepresentedTalentRespecCriteriaEventLikeCpp::MoneySpentOnRespecs { amount: 0 },
            RepresentedTalentRespecCriteriaEventLikeCpp::TotalRespecs { quantity: 1 },
        ],
        "C++ still updates respec criteria inside the final !noCost block with cost zero"
    );
    assert_eq!(
        session.represented_confirm_respec_wipe_requests_like_cpp(),
        &[RepresentedConfirmRespecWipeLikeCpp {
            respec_master: trainer,
            respec_type: SPEC_RESET_TALENTS_LIKE_CPP,
        }]
    );
}

#[tokio::test]
async fn login_at_login_reset_talents_resets_without_cost_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let talent_tabs = install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
    session.mark_represented_talents_loaded_like_cpp();
    session.set_player_gold_like_cpp(9_999);

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;
    assert!(
        drain_sent_packets(&send_rx).iter().any(|packet| *packet
            == session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()),
        "C++ sends SendTalentsInfoData after LearnTalent"
    );
    assert!(session.known_spells_like_cpp().contains(&50_101));

    session.set_represented_at_login_flags_like_cpp(
        AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
    );

    assert!(
        session.apply_represented_login_talent_reset_if_needed_like_cpp(),
        "C++ CharacterHandler applies AT_LOGIN_RESET_TALENTS through ResetTalents(true)"
    );

    let packets = drain_sent_packets(&send_rx);
    assert!(
        packets.iter().any(|packet| *packet
            == session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()),
        "C++ resends SendTalentsInfoData after ResetTalents(true)"
    );
    let expected_notification = PrintNotification {
        notify_text: "Your talents have been reset.".to_string(),
    }
    .to_bytes();
    assert_eq!(
        packets
            .iter()
            .filter(|packet| **packet == expected_notification)
            .count(),
        1,
        "C++ SendNotification(LANG_RESET_TALENTS) should be emitted once"
    );
    assert_eq!(
        session.represented_talent_reset_script_hooks_like_cpp(),
        &[RepresentedTalentResetScriptHookLikeCpp { no_cost: true }]
    );
    assert_eq!(
        session.represented_at_login_flags_like_cpp(),
        AT_LOGIN_RENAME_LIKE_CPP,
        "C++ removes only AT_LOGIN_RESET_TALENTS"
    );
    assert_eq!(
        session.represented_at_login_flag_removals_like_cpp(),
        &[RepresentedAtLoginFlagRemovalLikeCpp {
            flags: AT_LOGIN_RESET_TALENTS_LIKE_CPP,
            persist: true,
            db_statement_unrepresented: true,
        }]
    );
    assert_eq!(
        session.player_gold_like_cpp(),
        9_999,
        "C++ ResetTalents(true) skips ModifyMoney and criteria/accounting"
    );
    assert_eq!(session.represented_talent_reset_cost_like_cpp(), Some(0));
    assert_eq!(
        session.represented_talent_reset_time_secs_like_cpp(),
        Some(0)
    );
    assert!(
        session
            .represented_talent_respec_criteria_events_like_cpp()
            .is_empty(),
        "C++ skips criteria updates when noCost=true"
    );
    assert!(
        !session.known_spells_like_cpp().contains(&50_101),
        "C++ ResetTalents(true) still removes active talents"
    );
}

#[tokio::test]
async fn login_at_login_reset_spells_removes_known_spells_and_notifies_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_known_spells_like_cpp(vec![118, 133, 19740]);
    session.learn_dependent_known_spell_like_cpp(19740);
    session.set_represented_at_login_flags_like_cpp(
        AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_RESET_SPELLS_LIKE_CPP | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
    );

    assert!(
        session.apply_represented_login_spell_reset_if_needed_like_cpp(),
        "C++ CharacterHandler applies AT_LOGIN_RESET_SPELLS through Player::ResetSpells"
    );

    let expected_notification = PrintNotification {
        notify_text: "Your spells have been reset.".to_string(),
    }
    .to_bytes();
    let mut packets = Vec::new();
    while let Ok(packet) = send_rx.try_recv() {
        packets.push(packet);
    }
    assert_eq!(
        packets
            .iter()
            .filter(|packet| **packet == expected_notification)
            .count(),
        1,
        "C++ SendNotification(LANG_RESET_SPELLS) should be emitted once"
    );
    assert!(
        session.known_spells_like_cpp().is_empty(),
        "C++ ResetSpells(false) removes every spell in the copied PlayerSpellMap before relearning defaults"
    );
    assert_eq!(
        session.represented_at_login_flags_like_cpp(),
        AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
        "C++ removes only AT_LOGIN_RESET_SPELLS"
    );
    assert_eq!(
        session.represented_at_login_flag_removals_like_cpp(),
        &[RepresentedAtLoginFlagRemovalLikeCpp {
            flags: AT_LOGIN_RESET_SPELLS_LIKE_CPP,
            persist: true,
            db_statement_unrepresented: true,
        }]
    );
}

#[tokio::test]
async fn login_spell_reset_relearns_quest_rewarded_spells_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let quest_id = 7001;
    let reward_spell_id = 9001;
    let learned_spell_id = 9101;

    let mut reward_spell = test_learn_spell_info_like_cpp(reward_spell_id, learned_spell_id);
    reward_spell.effects[0].effect_index = 0;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(reward_spell_id, reward_spell);
    session.set_spell_store(Arc::new(spell_store));
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_like_cpp([
            test_rewarded_skill_ability_like_cpp(
                learned_spell_id,
                wow_data::skill::SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP,
            ),
        ]),
    ));

    let mut quest = test_quest_template_like_cpp(quest_id);
    quest.reward_spell = reward_spell_id as u32;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.rewarded_quests.insert(quest_id);
    session.set_known_spells_like_cpp(vec![118, learned_spell_id]);
    session.set_represented_at_login_flags_like_cpp(AT_LOGIN_RESET_SPELLS_LIKE_CPP);

    assert!(
        session.apply_represented_login_spell_reset_if_needed_like_cpp(),
        "C++ ResetSpells calls LearnQuestRewardedSpells after removing the copied PlayerSpellMap"
    );
    let _notification = send_rx
        .try_recv()
        .expect("C++ still notifies after ResetSpells");

    assert_eq!(
        session.known_spells_like_cpp(),
        &[learned_spell_id],
        "C++ LearnQuestRewardedSpells casts the reward spell when it teaches a missing quest-rewarded ability"
    );
}

#[tokio::test]
async fn login_spell_reset_skips_quest_reward_spell_without_rewarded_skill_ability_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let quest_id = 7002;
    let reward_spell_id = 9002;
    let learned_spell_id = 9102;

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        reward_spell_id,
        test_learn_spell_info_like_cpp(reward_spell_id, learned_spell_id),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_like_cpp([
            test_rewarded_skill_ability_like_cpp(learned_spell_id, 1),
        ]),
    ));

    let mut quest = test_quest_template_like_cpp(quest_id);
    quest.reward_spell = reward_spell_id as u32;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.rewarded_quests.insert(quest_id);
    session.set_known_spells_like_cpp(vec![learned_spell_id]);
    session.set_represented_at_login_flags_like_cpp(AT_LOGIN_RESET_SPELLS_LIKE_CPP);

    assert!(session.apply_represented_login_spell_reset_if_needed_like_cpp());
    let _notification = send_rx
        .try_recv()
        .expect("C++ still notifies after ResetSpells");

    assert!(
        session.known_spells_like_cpp().is_empty(),
        "C++ LearnQuestRewardedSpells requires SKILL_LINE_ABILITY_REWARDED_FROM_QUEST for missing first learned spell"
    );
}

#[tokio::test]
async fn login_spell_reset_reward_spell_minus_one_removes_source_spell_auras_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 7003);
    let quest_id = 7003;
    let source_spell_id = 9201;
    let other_spell_id = 9202;

    session.set_player_guid(Some(player_guid));
    let mut quest = test_quest_template_like_cpp(quest_id);
    quest.reward_spell = u32::MAX;
    quest.source_spell_id = source_spell_id as u32;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.rewarded_quests.insert(quest_id);
    session
        .visible_auras
        .insert(1, test_visible_aura_like_cpp(1, source_spell_id));
    session
        .visible_auras
        .insert(2, test_visible_aura_like_cpp(2, source_spell_id));
    session
        .visible_auras
        .insert(3, test_visible_aura_like_cpp(3, other_spell_id));
    session.set_known_spells_like_cpp(vec![118]);
    session.set_represented_at_login_flags_like_cpp(AT_LOGIN_RESET_SPELLS_LIKE_CPP);

    assert!(
        session.apply_represented_login_spell_reset_if_needed_like_cpp(),
        "C++ ResetSpells calls LearnQuestRewardedSpells after removing spells"
    );
    let _aura_remove_1 = send_rx
        .try_recv()
        .expect("C++ RemoveAurasDueToSpell sends aura removal updates");
    let _aura_remove_2 = send_rx
        .try_recv()
        .expect("C++ RemoveAurasDueToSpell removes every matching aura");
    let _notification = send_rx
        .try_recv()
        .expect("C++ still notifies after ResetSpells");

    assert!(
        session
            .visible_auras
            .values()
            .all(|aura| aura.spell_id != source_spell_id),
        "C++ RewardSpell=-1 removes auras due to SourceSpellID"
    );
    assert!(
        session
            .visible_auras
            .values()
            .any(|aura| aura.spell_id == other_spell_id),
        "C++ RemoveAurasDueToSpell does not remove unrelated aura spells"
    );
    assert!(
        session.known_spells_like_cpp().is_empty(),
        "RewardSpell=-1 removes source auras and returns without relearning spells"
    );
}

#[tokio::test]
async fn learn_talent_updates_represented_active_group_and_sends_talents_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent_tabs = install_test_talent_store(&mut session, &[(101, 2, 50_101)]);
    session.mark_represented_talents_loaded_like_cpp();
    session.set_represented_active_talent_group_like_cpp(1);
    session.set_represented_bonus_talent_groups_like_cpp(1);

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 2))
        .await;

    let sent = send_rx
        .try_recv()
        .expect("C++ sends SendTalentsInfoData after successful LearnTalent");
    assert_eq!(
        sent,
        session
            .represented_update_talent_data_packet_like_cpp()
            .to_bytes()
    );
    let packet = session.represented_update_talent_data_packet_like_cpp();
    assert_eq!(
        packet.groups[1].talents,
        vec![wow_packet::packets::misc::TalentInfoLikeCpp {
            talent_id: 101,
            rank: 2,
        }]
    );
    assert_eq!(
        packet.unspent_talent_points, 68,
        "C++ LearnTalent recomputes CharacterPoints from CalculateTalentsPoints - spent talents"
    );
}

#[tokio::test]
async fn registered_learn_talent_borrows_the_supplied_process_catalog() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let tabs = install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
    session.mark_represented_talents_loaded_like_cpp();
    let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    let entry = crate::session::registry::get_handler(ClientOpcodes::LearnTalent).unwrap();
    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    (entry.handler)(&mut session, &catalogs, learn_talent_packet(101, 0)).await;
    assert!(send_rx.is_empty(), "missing tab must not publish success");
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .is_empty()
    );
    let tabs = Arc::new(tabs);
    Arc::get_mut(&mut catalogs.player_bootstrap)
        .unwrap()
        .talent_tabs = Arc::clone(&tabs);
    let references = Arc::strong_count(&tabs);
    (entry.handler)(&mut session, &catalogs, learn_talent_packet(101, 0)).await;
    assert_eq!(
        send_rx.try_recv().unwrap(),
        session
            .represented_update_talent_data_packet_like_cpp()
            .to_bytes()
    );
    assert_eq!(
        Arc::strong_count(&tabs),
        references,
        "Session must not retain the catalog"
    );
    assert!(send_rx.is_empty());
}

#[tokio::test]
async fn learn_talents_delegates_each_id_as_rank_zero_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let talent_tabs =
        install_test_talent_store(&mut session, &[(101, 0, 50_101), (202, 0, 50_202)]);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talents(&talent_tabs, learn_talents_packet(&[101, 202]))
        .await;

    let first = send_rx
        .try_recv()
        .expect("first C++ delegated LearnTalent should send talent data");
    let second = send_rx
        .try_recv()
        .expect("second C++ delegated LearnTalent should send talent data");
    assert!(!first.is_empty());
    assert!(!second.is_empty());
    assert!(send_rx.try_recv().is_err());

    let packet = session.represented_update_talent_data_packet_like_cpp();
    assert_eq!(
        packet.groups[0].talents,
        vec![
            wow_packet::packets::misc::TalentInfoLikeCpp {
                talent_id: 101,
                rank: 0,
            },
            wow_packet::packets::misc::TalentInfoLikeCpp {
                talent_id: 202,
                rank: 0,
            },
        ]
    );
    assert_eq!(packet.unspent_talent_points, 69);
}

#[tokio::test]
async fn learn_talent_rejects_zero_character_points_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent_tabs = install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
    session.mark_represented_talents_loaded_like_cpp();
    session.set_player_character_points_like_cpp(0);

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .is_empty()
    );
    assert_eq!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .unspent_talent_points,
        0
    );
}

#[tokio::test]
async fn learn_talent_rejects_unloaded_snapshot_without_sending_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent_tabs = install_test_talent_store(&mut session, &[(101, 0, 50_101)]);

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .is_empty()
    );
    assert!(!session.represented_talents_loaded_like_cpp());
}

#[tokio::test]
async fn learn_talent_rejects_missing_talent_tab_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent_tabs = install_test_talent_store_without_tab(&mut session, &[(101, 0, 50_101)]);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .is_empty()
    );
}

#[tokio::test]
async fn learn_talent_rejects_wrong_class_talent_tab_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent_tabs =
        install_test_talent_store_with_tab_class_mask(&mut session, &[(101, 0, 50_101)], 1 << 1);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .is_empty()
    );
}

#[tokio::test]
async fn learn_talent_rejects_rank_outside_cpp_max_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent_tabs = install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 9))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .is_empty()
    );
}

#[tokio::test]
async fn learn_talent_rejects_known_or_higher_rank_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent_tabs = install_test_talent_store(&mut session, &[(101, 1, 50_101)]);
    session.mark_represented_talents_loaded_like_cpp();
    assert!(session.load_represented_talent_row_like_cpp(&talent_tabs, 101, 1, 0));

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents,
        vec![wow_packet::packets::misc::TalentInfoLikeCpp {
            talent_id: 101,
            rank: 1,
        }]
    );
}

#[tokio::test]
async fn learn_talent_enforces_prereq_rank_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let prereq = test_talent_entry_like_cpp(101, 1, 50_101);
    let mut dependent = test_talent_entry_like_cpp(202, 0, 50_202);
    dependent.prereq_talent[0] = 101;
    dependent.prereq_rank[0] = 1;
    let talent_tabs =
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![prereq, dependent], 1);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(202, 0))
        .await;
    assert!(send_rx.try_recv().is_err());

    assert!(session.load_represented_talent_row_like_cpp(&talent_tabs, 101, 1, 0));
    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(202, 0))
        .await;

    let sent = send_rx
        .try_recv()
        .expect("C++ accepts dependent talent once prereq rank is known");
    assert!(!sent.is_empty());
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .contains(&wow_packet::packets::misc::TalentInfoLikeCpp {
                talent_id: 202,
                rank: 0,
            })
    );
}

#[tokio::test]
async fn learn_talent_enforces_tier_spent_points_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let filler = test_talent_entry_like_cpp(101, 4, 50_101);
    let mut tier_one = test_talent_entry_like_cpp(202, 0, 50_202);
    tier_one.tier_id = 1;
    let talent_tabs =
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![filler, tier_one], 1);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(202, 0))
        .await;
    assert!(send_rx.try_recv().is_err());

    assert!(session.load_represented_talent_row_like_cpp(&talent_tabs, 101, 4, 0));
    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(202, 0))
        .await;

    let sent = send_rx
        .try_recv()
        .expect("C++ accepts tier-one talent after five points in the tree");
    assert!(!sent.is_empty());
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .contains(&wow_packet::packets::misc::TalentInfoLikeCpp {
                talent_id: 202,
                rank: 0,
            })
    );
}

#[tokio::test]
async fn learn_talent_rejects_invalid_learn_spell_trigger_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent = test_talent_entry_like_cpp(101, 0, 50_101);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(50_101, test_learn_spell_info_like_cpp(50_101, 60_101));
    let talent_tabs = install_test_talent_entries_with_spell_store_like_cpp(
        &mut session,
        vec![talent],
        spell_store,
    );
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .is_empty(),
        "C++ SpellMgr::IsSpellValid rejects SPELL_EFFECT_LEARN_SPELL when TriggerSpell is missing"
    );
    assert!(!session.known_spells_like_cpp().contains(&50_101));
}

#[tokio::test]
async fn learn_talent_learns_active_talent_spell_and_trigger_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let talent = test_talent_entry_like_cpp(101, 0, 50_101);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(50_101, test_learn_spell_info_like_cpp(50_101, 60_101));
    spell_store.insert(60_101, test_spell_info_like_cpp(60_101));
    let talent_tabs = install_test_talent_entries_with_spell_store_like_cpp(
        &mut session,
        vec![talent],
        spell_store,
    );
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(
        !send_rx
            .try_recv()
            .expect("C++ sends talent data after AddTalent succeeds")
            .is_empty()
    );
    assert!(session.known_spells_like_cpp().contains(&50_101));
    assert!(
        session.known_spells_like_cpp().contains(&60_101),
        "C++ Player::AddTalent learns the talent spell, whose LearnSpell effect teaches TriggerSpell"
    );
}

#[tokio::test]
async fn learn_talent_upgrade_removes_previous_rank_spell_and_trigger_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
    talent.spell_rank[1] = 50_102;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(50_101, test_learn_spell_info_like_cpp(50_101, 60_101));
    spell_store.insert(60_101, test_spell_info_like_cpp(60_101));
    spell_store.insert(50_102, test_learn_spell_info_like_cpp(50_102, 60_102));
    spell_store.insert(60_102, test_spell_info_like_cpp(60_102));
    let talent_tabs = install_test_talent_entries_with_spell_store_like_cpp(
        &mut session,
        vec![talent],
        spell_store,
    );
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;
    assert!(!send_rx.try_recv().expect("rank 0 learn sends").is_empty());
    assert!(session.known_spells_like_cpp().contains(&50_101));
    assert!(session.known_spells_like_cpp().contains(&60_101));

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 1))
        .await;

    assert!(!send_rx.try_recv().expect("rank 1 learn sends").is_empty());
    assert!(
        !session.known_spells_like_cpp().contains(&50_101),
        "C++ Player::AddTalent removes the previous rank spell before learning the new rank"
    );
    assert!(
        !session.known_spells_like_cpp().contains(&60_101),
        "C++ Player::AddTalent removes direct LearnSpell triggers from the previous rank"
    );
    assert!(session.known_spells_like_cpp().contains(&50_102));
    assert!(session.known_spells_like_cpp().contains(&60_102));
}

#[tokio::test]
async fn learn_talent_removes_change_talent_interrupt_auras_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let talent_tabs = install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
    session.mark_represented_talents_loaded_like_cpp();
    session.visible_auras.insert(
        1,
        visible_aura(1, SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP),
    );
    session.visible_auras.insert(2, visible_aura(2, 0));

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(
        send_rx.try_recv().is_ok(),
        "C++ sends updates after successful AddTalent and aura removal"
    );
    assert!(
        !session.visible_auras.contains_key(&1),
        "C++ Player::AddTalent(learning=true) removes ChangeTalent interrupt auras"
    );
    assert!(
        session.visible_auras.contains_key(&2),
        "unrelated auras survive ChangeTalent interrupt removal"
    );
}
