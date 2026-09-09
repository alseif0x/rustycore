//! Session scenarios exercising the represented progression responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn reputation_min_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_508;
    let faction_id = 7;
    let rep_list_id: u32 = 5;
    let mut faction = FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP,
        order: 0,
        storage_index: -1,
        object_id: faction_id as i32,
        amount: 1_000,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));
    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(rep_list_id)
        .expect("reputation state")
        .standing = 900;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.reputation_changed_like_cpp(faction_id, 100).await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn increase_reputation_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_509;
    let faction_id = 7;
    let rep_list_id: u32 = 5;
    let mut faction = FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: faction_id as i32,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
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

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.reputation_changed_like_cpp(faction_id, 100).await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn reputation_max_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_510;
    let faction_id = 7;
    let rep_list_id: u32 = 5;
    let mut faction = FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP,
        order: 0,
        storage_index: -1,
        object_id: faction_id as i32,
        amount: 1_000,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));
    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(rep_list_id)
        .expect("reputation state")
        .standing = 1_100;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );

    adopt_player_quest_fixture_into_canonical_owner_like_cpp(&mut session);
    session.reputation_changed_like_cpp(faction_id, -100).await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[test]
fn canonical_player_talents_and_glyphs_follow_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_562);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TalentOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    let mut owned = wow_entities::PlayerTalentRuntimeState {
        talents_loaded: true,
        glyphs_loaded: true,
        active_group: 1,
        bonus_groups: 1,
        reset_talents_cost: 100_000,
        reset_talents_time_secs: 12_345,
        ..Default::default()
    };
    owned.talent_groups[1].insert(42, 1);
    owned.glyph_groups[1][2] = 700;

    assert_eq!(
        session.mutate_player_talent_runtime_like_cpp(|runtime| *runtime = owned.clone()),
        Some(())
    );
    assert_eq!(
        session.player_talent_runtime_snapshot_like_cpp(),
        Some(owned.clone())
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.player_talent_runtime_snapshot_like_cpp(),
        Some(owned.clone())
    );

    let mut replacement_state = wow_entities::PlayerTalentRuntimeState {
        talents_loaded: true,
        glyphs_loaded: true,
        active_group: 0,
        bonus_groups: 0,
        reset_talents_cost: 500_000,
        reset_talents_time_secs: 98_765,
        ..Default::default()
    };
    replacement_state.talent_groups[0].insert(99, 2);
    replacement_state.glyph_groups[0][3] = 900;
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.replace_talent_runtime_like_cpp(replacement_state.clone());
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_talent_runtime_snapshot_like_cpp(), None);
    assert_eq!(
        session
            .mutate_player_talent_runtime_like_cpp(|_| panic!("stale owner must not run mutation")),
        None::<()>
    );
    assert!(!session.set_represented_active_talent_group_like_cpp(1));
    assert!(!session.set_represented_bonus_talent_groups_like_cpp(1));
    assert!(!session.set_represented_talent_reset_state_like_cpp(10_000, 1));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.talent_runtime_like_cpp().clone()
            }),
        Some(replacement_state)
    );
}
#[test]
fn canonical_player_reputation_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_572);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ReputationOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(
        session
            .mutate_reputation_mgr_like_cpp(|mgr| {
                let mut faction = crate::reputation::FactionStateLikeCpp::new_like_cpp(
                    72,
                    5,
                    ReputationFlagsLikeCpp::VISIBLE,
                );
                faction.standing = 1_234;
                mgr.insert_state_for_test_like_cpp(faction);
                mgr.apply_force_reaction_like_cpp(
                    87,
                    wow_data::reputation::ReputationRankLikeCpp::Hostile,
                    true,
                );
            })
            .is_some()
    );
    session.set_championing_faction_like_cpp(72);
    assert_eq!(
        session.with_reputation_mgr_like_cpp(|mgr| {
            (
                mgr.get_state(5).map(|state| state.standing),
                mgr.forced_rank_by_faction_id_like_cpp(87),
            )
        }),
        Some((
            Some(1_234),
            Some(wow_data::reputation::ReputationRankLikeCpp::Hostile)
        ))
    );
    assert_eq!(session.resolved_championing_faction_like_cpp(), Some(72));

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session
            .with_reputation_mgr_like_cpp(|mgr| { mgr.get_state(5).map(|state| state.standing) }),
        Some(Some(1_234))
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().championing_faction_id = 999;
    replacement
        .gameplay_state_mut()
        .reputations
        .push(wow_entities::PlayerReputationRecord {
            faction_id: 999,
            reputation_list_id: 6,
            standing: 7_777,
            ..Default::default()
        });
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.with_reputation_mgr_like_cpp(|_| ()), None);
    assert_eq!(session.mutate_reputation_mgr_like_cpp(|_| ()), None);
    session.set_championing_faction_like_cpp(72);
    assert_eq!(session.resolved_championing_faction_like_cpp(), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                (
                    player.gameplay_state().championing_faction_id,
                    player.gameplay_state().reputations.clone(),
                )
            }),
        Some((
            999,
            vec![wow_entities::PlayerReputationRecord {
                faction_id: 999,
                reputation_list_id: 6,
                standing: 7_777,
                ..Default::default()
            }]
        ))
    );
}
#[test]
fn reputation_low_level_rate_uses_script_adjusted_gray_level_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_reputation_rates_like_cpp(ReputationRatesLikeCpp {
        low_level_quest: 0.5,
        ..ReputationRatesLikeCpp::default()
    });

    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            75,
            100,
            7,
            false,
        ),
        100
    );

    session.set_represented_gray_level_script_override_like_cpp(80, 79);
    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            75,
            100,
            7,
            false,
        ),
        50
    );
}
#[test]
fn reputation_gain_recruit_a_friend_bonus_requires_configured_distance_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1);
    let recruit_guid = ObjectGuid::create_player(1, 2);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_recruiter_id_like_cpp(2);
    session.set_reputation_rates_like_cpp(ReputationRatesLikeCpp {
        recruit_a_friend_bonus: 0.1,
        recruit_a_friend_distance: 10.0,
        ..ReputationRatesLikeCpp::default()
    });

    let (recruit_tx, _recruit_rx) = flume::bounded(10);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut recruit_info = broadcast_info(recruit_guid, recruit_tx);
    recruit_info.placement.map_id = 571;
    recruit_info.placement.position = Position::new(25.0, 0.0, 0.0, 0.0);
    recruit_info.identity.account_id = 2;
    player_registry.register_or_replace(recruit_guid, recruit_info, Default::default());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(recruit_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_state(SessionState::LoggedIn);

    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            80,
            100,
            7,
            false,
        ),
        100
    );
}
#[test]
fn first_login_start_all_reputation_applies_cpp_common_and_alliance_lists() {
    let (mut session, _, send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    session.set_faction_store(Arc::new(first_login_reputation_faction_store_like_cpp()));
    let _ = session
        .reputation_mgr_like_cpp_mut()
        .initialize_factions_packet_like_cpp();
    session.set_start_all_reputation_like_cpp(true);

    let applied = session.apply_represented_first_login_reputation_like_cpp();

    assert_eq!(
        applied,
        FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP.len()
            + FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP.len()
    );
    let faction_store = session.faction_store().expect("faction store").clone();
    for faction_id in FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP
        .iter()
        .chain(FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP.iter())
    {
        let faction = faction_store.get(*faction_id).expect("configured faction");
        let state = session
            .reputation_mgr_like_cpp()
            .get_state(faction.reputation_index as u32)
            .expect("reputation state");
        assert_eq!(
            state.standing,
            wow_data::reputation::REPUTATION_CAP_LIKE_CPP,
            "C++ CharacterHandler passes 42999, then ReputationMgr clamps at GetMaxReputation"
        );
    }
    let horde = faction_store.get(76).expect("horde faction");
    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(horde.reputation_index as u32)
            .expect("horde state")
            .standing,
        0,
        "Alliance first login must not apply the Horde-only branch"
    );

    let packet = drain_server_packet_bytes(&send_rx)
        .into_iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SetFactionStanding)
        })
        .expect("C++ repMgr.SendState(nullptr) equivalent");
    let mut reader = wow_packet::WorldPacket::from_bytes(&packet);
    reader.skip_opcode();
    assert_eq!(reader.read_float().unwrap(), 0.0);
    assert_eq!(reader.read_uint32().unwrap(), applied as u32);
}
#[test]
fn first_login_start_all_reputation_is_config_gated_and_uses_horde_branch() {
    let (mut session, _, send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(0, 2, 1, 1, 0);
    session.set_faction_store(Arc::new(first_login_reputation_faction_store_like_cpp()));
    let _ = session
        .reputation_mgr_like_cpp_mut()
        .initialize_factions_packet_like_cpp();

    assert_eq!(
        session.apply_represented_first_login_reputation_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());

    session.set_start_all_reputation_like_cpp(true);
    let applied = session.apply_represented_first_login_reputation_like_cpp();

    assert_eq!(
        applied,
        FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP.len()
            + FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP.len()
    );
    let faction_store = session.faction_store().expect("faction store").clone();
    let orgrimmar = faction_store.get(76).expect("horde faction");
    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(orgrimmar.reputation_index as u32)
            .expect("horde state")
            .standing,
        wow_data::reputation::REPUTATION_CAP_LIKE_CPP
    );
    let stormwind = faction_store.get(72).expect("alliance faction");
    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(stormwind.reputation_index as u32)
            .expect("alliance state")
            .standing,
        0,
        "Horde first login must not apply the Alliance-only branch"
    );
}
#[test]
fn player_registry_reputation_snapshot_syncs_from_canonical_player_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_registry = Arc::new(PlayerRegistry::default());
    let player_guid = ObjectGuid::create_player(1, 604);

    session.set_player_guid(Some(player_guid));
    session.player_name = Some("RepSnapshot".into());
    session.player_position = Some(Position::new(10.0, 10.0, 0.0, 0.0));
    session.current_map_id = 571;
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&player_registry));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| {
        player
            .gameplay_state_mut()
            .reputations
            .push(wow_entities::PlayerReputationRecord {
                faction_id: 72,
                standing: 1234,
                flags: 0,
                ..Default::default()
            });
    });

    session.register_in_player_registry();

    // #252: the standings are no longer mirrored, so prove the resolver reads
    // them off the canonical owner, and defaults when there is no owner to read.
    assert_eq!(
        player_registry
            .quest_sharing_snapshot(player_guid, Some(&canonical))
            .expect("quest sharing snapshot")
            .reputation_standings,
        Some(vec![(72, 1234)])
    );
    // Without a canonical owner the standings are unknown, not empty: the
    // consumer must not read an absent owner as "no reputation" (#252).
    assert!(
        player_registry
            .quest_sharing_snapshot(player_guid, None)
            .expect("identity stays directory-owned and still resolves")
            .reputation_standings
            .is_none()
    );
}
#[test]
fn represented_faction_reaction_static_branch_uses_player_reputation_and_at_war_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(72, 1),
    ])));
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(1, 72, 0, 0, 0),
            faction_template_entry(2, 930, 0, 0, 0),
        ]),
    ));
    let state = session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(1)
        .unwrap();
    state.standing = 3_500;

    let input = RepresentedFactionReactionInputLikeCpp {
        source_faction_template_id: 1,
        target_faction_template_id: 2,
        target_has_player_owner: true,
        target_player_owner_is_current_session: true,
        target_player_contested_pvp: false,
        target_is_unit: true,
        target_ignores_reputation: false,
    };

    assert_eq!(
        session.represented_faction_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Friendly
    );

    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(1)
        .unwrap()
        .flags |= ReputationFlagsLikeCpp::AT_WAR;

    assert_eq!(
        session.represented_faction_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Neutral
    );
}
#[test]
fn represented_get_reaction_player_controlled_reputation_branch_matches_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(930, 2),
    ])));
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(1, 72, 0, 0, 0),
            faction_template_entry(2, 930, 0, 0, 0),
        ]),
    ));

    let input = represented_get_reaction_input_like_cpp();
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Friendly
    );

    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(2)
        .unwrap()
        .flags |= ReputationFlagsLikeCpp::AT_WAR;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Hostile
    );

    session
        .reputation_mgr_like_cpp_mut()
        .apply_force_reaction_like_cpp(
            930,
            wow_data::reputation::ReputationRankLikeCpp::Hated,
            true,
        );
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Hated
    );
}
