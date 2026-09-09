//! Spell scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_giver_choose_reward_records_reward_spell_cast_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7023;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_spell = 12_345;
    quest.reward_display_spell = [22_001, 22_002, 0];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_spell_casts_like_cpp(),
        &[RepresentedQuestRewardSpellCastLikeCpp {
            quest_id,
            spell_id: 12_345,
            kind: RepresentedQuestRewardSpellKindLikeCpp::RewardSpell,
            can_delay_teleport_like_cpp: true,
            spell_info_lookup_unrepresented: true,
            caster_selection_unrepresented: true,
            cast_spell_runtime_unrepresented: true,
        }]
    );
    assert!(!session.represented_can_delay_teleport_like_cpp());
}
#[tokio::test]
async fn quest_giver_choose_reward_records_display_spells_only_without_reward_spell_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7024;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP;
    quest.reward_display_spell = [22_001, 0, 22_003];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_spell_casts_like_cpp(),
        &[
            RepresentedQuestRewardSpellCastLikeCpp {
                quest_id,
                spell_id: 22_001,
                kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell { index: 0 },
                can_delay_teleport_like_cpp: true,
                spell_info_lookup_unrepresented: true,
                caster_selection_unrepresented: false,
                cast_spell_runtime_unrepresented: true,
            },
            RepresentedQuestRewardSpellCastLikeCpp {
                quest_id,
                spell_id: 22_003,
                kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell { index: 2 },
                can_delay_teleport_like_cpp: true,
                spell_info_lookup_unrepresented: true,
                caster_selection_unrepresented: false,
                cast_spell_runtime_unrepresented: true,
            },
        ]
    );
    assert!(!session.represented_can_delay_teleport_like_cpp());
}
#[tokio::test]
async fn quest_confirm_accept_source_spell_records_two_self_casts_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 191);
    let quest_id = 70121;
    let mut quest = quest_template_with_objective_count(quest_id, 2);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.source_spell_id = 12_345;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-spell-only quest should still insert represented local AddQuest state");
    assert_eq!(status.quest_id, quest_id);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert!(!status.explored);
    assert_eq!(status.objective_counts, vec![0, 0]);
    assert_eq!(status.slot, 0);
    let registry = session.player_registry().expect("test installs registry");
    let snapshot = registry
        .loot_player_context(receiver_guid)
        .expect("receiver canonical state should sync after source-spell quest insertion");
    assert_eq!(
        snapshot.active_quest_statuses.get(&quest_id),
        Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: None,
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: Some(12_345),
            represented_source_spell_self_casts: 2,
        }]
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_bound_objective_broadcasts_to_group_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 193);
    let other_guid = ObjectGuid::create_player(1, 194);
    let quest_id = 7115;
    let source_item_id = 9203;
    let quest_log_item_id = 9303;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: quest_log_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.cache_item_template_addon_quest_log_item_id_like_cpp(source_item_id, quest_log_item_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_loaded_player_name_like_cpp("Receiver".to_string());
    session.register_in_player_registry();

    let (mut sender_session, sender_rx) = make_session();
    sender_session.set_player_guid(Some(sender_guid));
    sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
    sender_session.set_player_registry(Arc::clone(&player_registry));
    sender_session.register_in_player_registry();
    assert!(sender_session.adopt_registered_canonical_player_fixture_like_cpp());
    add_active_quest_in_slot_with_status(
        &mut sender_session,
        quest_id,
        0,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    sender_session.sync_player_registry_state_like_cpp();

    let (mut other_session, other_rx) = make_session();
    other_session.set_player_guid(Some(other_guid));
    other_session.set_loaded_player_name_like_cpp("Other".to_string());
    other_session.set_player_registry(player_registry);
    other_session.register_in_player_registry();

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender_guid);
    group.add_member(receiver_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let self_packet = send_rx.try_recv().expect("receiver group packet");
    assert_eq!(sender_rx.try_recv().unwrap(), self_packet);
    assert_eq!(other_rx.try_recv().unwrap(), self_packet);
    assert!(send_rx.try_recv().is_err());
    let mut packet = WorldPacket::from_bytes(&self_packet);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
    assert_eq!(
        packet.read_uint8().unwrap(),
        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
    );
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_uint32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert!(!packet.read_bit().unwrap());
    assert!(!packet.read_bit().unwrap());
    assert_eq!(packet.read_bits(3).unwrap(), 3);
}
