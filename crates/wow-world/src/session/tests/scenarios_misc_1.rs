//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn player_interaction_data_has_one_exact_reset_and_match_owner_like_cpp() {
    let trainer_a = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 100, 1);
    let trainer_b = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 101, 1);
    let mut interaction = PlayerInteractionDataLikeCpp::default();

    assert!(interaction.source_guid.is_empty());
    assert_eq!(interaction.trainer_id, 0);
    assert!(!interaction.trainer_matches(trainer_a, 0));

    interaction.set_trainer(trainer_a, 10);
    assert!(interaction.trainer_matches(trainer_a, 10));
    assert!(!interaction.trainer_matches(trainer_b, 10));
    assert!(!interaction.trainer_matches(trainer_a, 11));

    interaction.set_trainer(trainer_a, u32::MAX);
    assert!(interaction.trainer_matches(trainer_a, -1));
    interaction.set_trainer(trainer_a, 0x8000_0000);
    assert!(interaction.trainer_matches(trainer_a, i32::MIN));

    interaction.set_source(trainer_b);
    assert_eq!(interaction.source_guid, trainer_b);
    assert_eq!(
        interaction.trainer_id, 0,
        "a generic C++ interaction replaces the complete prior trainer provenance"
    );
    assert!(
        !interaction.trainer_matches(trainer_b, 0),
        "a generic source with C++'s reset TrainerId=0 is not an active trainer window"
    );
    assert!(!interaction.reset_if_source(trainer_a));
    assert_eq!(interaction.source_guid, trainer_b);
    assert!(interaction.reset_if_source(trainer_b));
    assert_eq!(interaction, PlayerInteractionDataLikeCpp::default());
}
#[test]
fn player_menu_state_does_not_survive_character_lifetime_like_cpp() {
    let (mut session, _, _) = make_session();
    let first_player = ObjectGuid::create_player(1, 70_001);
    let second_player = ObjectGuid::create_player(1, 70_002);
    let trainer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 100, 1);
    session.set_player_guid(Some(first_player));
    session.visible_auras.insert(0, test_visible_aura(0, 999));
    session.player_aura_authority_complete_like_cpp = true;
    assert!(session.set_complete_player_skill_records_like_cpp(
        HashMap::from([(
            95,
            RepresentedPlayerSkillLikeCpp {
                skill_id: 95,
                step: 0,
                value: 0,
                max: 0,
                profession_slot: -1,
                state: RepresentedPlayerSkillStateLikeCpp::Deleted,
            },
        )]),
        1,
    ));
    assert!(
        session
            .player_skill_non_durable_tombstones_like_cpp
            .contains(&95)
    );
    session.set_player_trainer_interaction_like_cpp(trainer, 77);
    session.record_spell_acquisition_post_commit_action_like_cpp(
        crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability {
            skill_id: u32::from(SKILL_RIDING_LIKE_CPP),
        },
    );
    session.gossip_options.push(GossipOptionInfo {
        gossip_option_id: 1,
        menu_id: 2,
        order_index: 3,
        option_npc: 4,
        action_menu_id: 5,
    });

    session.set_player_guid(Some(first_player));
    assert!(
        session.player_trainer_interaction_matches_like_cpp(trainer, 77),
        "reasserting the same Player identity must not reset its PlayerMenu"
    );
    assert_eq!(session.gossip_options.len(), 1);
    assert_eq!(
        session.visible_auras.get(&0).map(|aura| aura.spell_id),
        Some(999)
    );
    assert!(session.player_aura_authority_complete_like_cpp());
    assert!(
        session
            .player_skill_non_durable_tombstones_like_cpp
            .contains(&95),
        "reasserting the same Player identity retains its skill tombstones"
    );

    session.set_player_guid(None);

    assert!(session.player_interaction_source_guid_like_cpp().is_none());
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
    assert!(session.gossip_options.is_empty());
    assert!(
        session.visible_auras.is_empty(),
        "active auras cannot cross a C++ Player lifetime"
    );
    assert!(
        !session.player_aura_authority_complete_like_cpp(),
        "the next Player must establish its own complete aura authority"
    );
    assert!(
        session
            .player_skill_non_durable_tombstones_like_cpp
            .is_empty(),
        "skill tombstones cannot cross a C++ Player lifetime"
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty(),
        "post-commit player intents cannot cross a character lifetime"
    );

    session.set_player_guid(Some(second_player));
    assert!(session.player_interaction_source_guid_like_cpp().is_none());
    assert!(session.gossip_options.is_empty());
    assert!(session.visible_auras.is_empty());
    assert!(!session.player_aura_authority_complete_like_cpp());
}
#[test]
fn reset_seasonal_removes_older_than_start_and_erases_emptied_bucket_like_cpp() {
    let (mut session, _, _) = make_session();
    session.seed_seasonal_quest_status_like_cpp(7, 1001, 99);
    session.set_seasonal_quest_changed_like_cpp_for_test(true);

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::RemovedOlderCompletions
    );
    assert_eq!(outcome.removed_quest_ids, vec![1001]);
    assert_eq!(outcome.completed_bit_cleared, 0);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 1);
    assert_eq!(outcome.completed_bit_clear_unrepresented, 0);
    assert!(outcome.event_bucket_erased);
    assert_eq!(session.seasonal_quest_bucket_like_cpp(7), None);
    assert!(!session.seasonal_quest_changed_like_cpp());
    assert!(!outcome.seasonal_quest_changed);
}
#[test]
fn reset_seasonal_keeps_equal_and_newer_completions_like_cpp() {
    let (mut session, _, _) = make_session();
    session.seed_seasonal_quest_status_like_cpp(7, 1001, 100);
    session.seed_seasonal_quest_status_like_cpp(7, 1002, 101);
    session.set_quest_v2_store(Arc::new(seasonal_quest_v2_store_like_cpp([
        (1001, 65),
        (1002, 66),
    ])));
    assert!(session.set_loaded_quest_completed_bit_like_cpp(65));
    assert!(session.set_loaded_quest_completed_bit_like_cpp(66));
    session.set_seasonal_quest_changed_like_cpp_for_test(true);

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::NoOlderCompletions
    );
    assert!(outcome.removed_quest_ids.is_empty());
    assert_eq!(outcome.completed_bit_cleared, 0);
    let bucket = session
        .seasonal_quest_bucket_like_cpp(7)
        .expect("bucket kept");
    assert_eq!(bucket.get(&1001), Some(&100));
    assert_eq!(bucket.get(&1002), Some(&101));
    assert_eq!(
        session.represented_quest_completed_bits_like_cpp,
        BTreeSet::from([65, 66])
    );
    assert!(!session.seasonal_quest_changed_like_cpp());
}
#[test]
fn reset_seasonal_zero_or_missing_unique_bit_removes_without_inventing_bit_like_cpp() {
    let (mut session, _, _) = make_session();
    session.seed_seasonal_quest_status_like_cpp(7, 1001, 99);
    session.seed_seasonal_quest_status_like_cpp(7, 1002, 98);
    session.set_quest_v2_store(Arc::new(seasonal_quest_v2_store_like_cpp([(1001, 0)])));

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(outcome.removed_quest_ids, vec![1001, 1002]);
    assert_eq!(outcome.completed_bit_cleared, 0);
    assert_eq!(outcome.completed_bit_skipped_zero_unique_bit, 2);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 0);
    assert_eq!(outcome.completed_bit_clear_unrepresented, 0);
    assert_eq!(session.seasonal_quest_bucket_like_cpp(7), None);
    assert!(session.represented_quest_completed_bits_like_cpp.is_empty());
}
#[test]
fn reset_seasonal_missing_event_leaves_changed_false_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_seasonal_quest_changed_like_cpp_for_test(true);

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::MissingEvent
    );
    assert!(outcome.removed_quest_ids.is_empty());
    assert!(!session.seasonal_quest_changed_like_cpp());
    assert!(!outcome.seasonal_quest_changed);
}
#[test]
fn reset_seasonal_preexisting_empty_bucket_is_preserved_like_cpp() {
    let (mut session, _, _) = make_session();
    session.seed_empty_seasonal_event_bucket_like_cpp(7);
    session.set_seasonal_quest_changed_like_cpp_for_test(true);

    let outcome = session.reset_seasonal_quest_status_like_cpp(7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::EmptyEvent
    );
    assert!(outcome.removed_quest_ids.is_empty());
    assert!(session.seasonal_quest_bucket_like_cpp(7).is_some());
    assert!(!session.seasonal_quest_changed_like_cpp());
}
#[tokio::test]
async fn reset_seasonal_command_processing_drains_session_command_like_cpp() {
    let (mut session, _, _) = make_session();
    session.seed_seasonal_quest_status_like_cpp(7, 1001, 99);
    session.seed_seasonal_quest_status_like_cpp(7, 1002, 100);
    session
        .session_command_tx()
        .try_send(SessionCommand::ResetSeasonalQuestStatus(
            ResetSeasonalQuestStatusCommand {
                event_id: 7,
                event_start_time: 100,
            },
        ))
        .expect("command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(session.drain_session_commands().is_empty());
    let bucket = session
        .seasonal_quest_bucket_like_cpp(7)
        .expect("bucket kept");
    assert_eq!(bucket.get(&1001), None);
    assert_eq!(bucket.get(&1002), Some(&100));
}
#[tokio::test]
async fn criteria_tree_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 12_506;
    let criteria_tree_id = 90_001;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 14, // C++ QUEST_OBJECTIVE_CRITERIA_TREE.
        order: 0,
        storage_index: 0,
        object_id: criteria_tree_id as i32,
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
    session
        .kill_credit_criteria_tree_objective_like_cpp(criteria_tree_id)
        .await;

    assert_canonical_quest_status_like_cpp(&session, quest_id, None, true);
    assert_eq!(
        session.represented_quest_complete_status_updates_like_cpp(),
        &[RepresentedQuestCompleteStatusUpdateLikeCpp {
            quest_id,
            old_status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            new_status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            send_quest_update_called: true,
            quest_slot_state_complete_represented: true,
            quest_slot_state_live_update_unrepresented: true,
            visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
            spell_area_runtime_unrepresented: true,
            tracking_event_auto_reward_unrepresented: false,
            quest_tracker_complete_time_unrepresented: true,
            script_status_change_unrepresented: true,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddCreditSimple,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[test]
fn represented_player_condition_id_matches_cpp_lookup_semantics() {
    let (mut session, _, _) = make_session();
    session.player_class = 1;
    assert!(!session.represented_meets_player_condition_id_like_cpp(42));

    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    assert!(session.represented_meets_player_condition_id_like_cpp(0));
    assert!(session.represented_meets_player_condition_id_like_cpp(42));
    assert!(!session.represented_meets_player_condition_id_like_cpp(43));
    assert!(session.represented_meets_player_condition_id_like_cpp(999));
}
#[test]
fn represented_player_condition_explored_uses_area_bit_blocks_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 900,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: 65,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 901,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            explored: [900, 0],
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            explored: [901, 0],
            ..Default::default()
        },
    ])));

    assert!(!session.represented_meets_player_condition_id_like_cpp(42));

    session.represented_explored_zones_like_cpp[1] = 2;
    assert!(session.represented_meets_player_condition_id_like_cpp(42));
    assert!(!session.represented_meets_player_condition_id_like_cpp(43));
}
#[test]
fn represented_player_condition_area_uses_parent_chain_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_zone_area_like_cpp(12, 901);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 900,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 901,
            continent_id: 0,
            parent_area_id: 900,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            area_id: [900, 0, 0, 0],
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            area_id: [902, 0, 0, 0],
            ..Default::default()
        },
    ])));

    assert!(session.represented_meets_player_condition_id_like_cpp(42));
    assert!(!session.represented_meets_player_condition_id_like_cpp(43));
}
#[test]
fn represented_taxi_edge_distance_matches_cpp_condition_filter() {
    let (mut session, _, _) = make_session();
    session.player_class = 1;
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    assert_eq!(
        session.represented_taxi_edge_distance_like_cpp(true, 42, 1234),
        1234
    );
    assert_eq!(
        session.represented_taxi_edge_distance_like_cpp(true, 43, 1234),
        u16::MAX as u32
    );
    assert_eq!(
        session.represented_taxi_edge_distance_like_cpp(false, 42, 1234),
        u16::MAX as u32
    );
    assert_eq!(
        session.represented_taxi_edge_distance_like_cpp(true, 999, 1234),
        1234
    );
}
#[test]
fn represented_mount_x_display_usable_matches_cpp_condition_filter() {
    let (mut session, _, _) = make_session();
    session.player_class = 1;
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    assert!(session.represented_mount_x_display_usable_like_cpp(0));
    assert!(session.represented_mount_x_display_usable_like_cpp(42));
    assert!(!session.represented_mount_x_display_usable_like_cpp(43));
    assert!(session.represented_mount_x_display_usable_like_cpp(999));
}
#[test]
fn represented_taxi_usable_mount_displays_match_cpp_filter() {
    let (mut session, _, _) = make_session();
    session.player_class = 1;
    session.set_known_spells_like_cpp(vec![100]);
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 7,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 8,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_mount_x_display_store(Arc::new(wow_data::MountXDisplayStore::from_entries([
        wow_data::MountXDisplayEntry {
            id: 1,
            creature_display_info_id: 1000,
            player_condition_id: 42,
            mount_id: 7,
        },
        wow_data::MountXDisplayEntry {
            id: 2,
            creature_display_info_id: 1001,
            player_condition_id: 43,
            mount_id: 7,
        },
        wow_data::MountXDisplayEntry {
            id: 3,
            creature_display_info_id: 1002,
            player_condition_id: 0,
            mount_id: 7,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    assert_eq!(
        session.represented_taxi_usable_mount_displays_like_cpp(7),
        vec![1000, 1002]
    );
    assert!(
        session
            .represented_taxi_usable_mount_displays_like_cpp(8)
            .is_empty()
    );
    assert!(
        session
            .represented_taxi_usable_mount_displays_like_cpp(99)
            .is_empty()
    );
}
#[test]
fn summon_object_wild_session_resolver_ignores_other_effects_like_cpp() {
    let (mut session, _, _) = make_session();
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
        ..Default::default()
    };

    assert!(
        session
            .apply_effect_summon_object_wild_like_cpp(
                700,
                &effect,
                &target_data_with_destination_like_cpp(Position::ZERO),
            )
            .is_none()
    );
}
#[test]
fn summon_object_wild_session_resolver_rejects_missing_template_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_gameobject_template_lifecycle_store(Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::default(),
    ));

    let outcome = session
        .apply_effect_summon_object_wild_like_cpp(
            700,
            &summon_object_wild_effect_like_cpp(9001),
            &target_data_with_destination_like_cpp(Position::new(1.0, 2.0, 3.0, 0.5)),
        )
        .expect("wild effect should return represented outcome");

    assert_eq!(
        outcome.status,
        ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingTemplate
    );
    assert_eq!(outcome.template_entry, Some(9001));
    assert!(outcome.map_outcome.is_none());
}
#[test]
fn summon_object_wild_session_resolver_creates_go_with_explicit_destination_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 700_u32;
    let template_entry = 9001_u32;
    let player_guid = ObjectGuid::create_player(1, 7001);
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 20.0, 30.0, 1.0);
    let destination = Position::new(11.0, 22.0, 33.0, 0.75);
    insert_test_player_into_canonical_map_like_cpp(
        &canonical,
        player_guid,
        571,
        0,
        player_position,
    );
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, player_position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session
        .set_gameobject_template_lifecycle_store(summon_go_template_store_like_cpp(template_entry));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        summon_go_spell_misc_entry_like_cpp(spell_id, 77),
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 77,
            duration: 5_000,
            duration_per_level: 0,
            max_duration: 0,
        },
    ])));

    let outcome = session
        .apply_effect_summon_object_wild_like_cpp(
            i32::try_from(spell_id).unwrap(),
            &summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap()),
            &target_data_with_destination_like_cpp(destination),
        )
        .expect("wild effect should return represented outcome");

    assert_eq!(
        outcome.status,
        ApplyEffectSummonObjectWildSessionStatusLikeCpp::MapResolved
    );
    assert_eq!(outcome.template_entry, Some(template_entry));
    assert_eq!(outcome.duration_ms, Some(5_000));
    assert!(outcome.explicit_destination_used);
    assert!(!outcome.close_point_fallback_represented);
    let map_outcome = outcome.map_outcome.expect("map body should run");
    assert_eq!(
        map_outcome.status,
        wow_map::map::SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
    );
    assert_eq!(map_outcome.template_entry, template_entry);
    assert_eq!(map_outcome.spell_id, spell_id);
    assert_eq!(map_outcome.respawn_time_secs, Some(5));
    let go_guid = map_outcome.guid.expect("created GO guid");
    let manager = canonical.lock().unwrap();
    let go = manager
        .find_map(571, 0)
        .and_then(|managed| managed.map().get_typed_game_object(go_guid))
        .expect("summoned GO should be in canonical map");
    assert_eq!(go.world().position(), destination);
    assert_eq!(go.spell_id(), spell_id);
    assert_eq!(go.respawn_time(), 5);
}
#[test]
fn summon_object_slot_session_resolver_ignores_other_effects_like_cpp() {
    let (mut session, _, _) = make_session();
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
        ..Default::default()
    };

    assert!(
        session
            .apply_effect_summon_object_slot_like_cpp(
                700,
                &effect,
                &target_data_with_destination_like_cpp(Position::ZERO),
            )
            .is_none()
    );
}
#[test]
fn summon_object_slot_session_resolver_rejects_invalid_slot_like_cpp() {
    let (mut session, _, _) = make_session();
    let effect = summon_object_slot_effect_like_cpp(9001, MAX_GAMEOBJECT_SLOT_LIKE_CPP as u32);

    let outcome = session
        .apply_effect_summon_object_slot_like_cpp(
            700,
            &effect,
            &target_data_with_destination_like_cpp(Position::ZERO),
        )
        .expect("slot effect should return represented outcome");

    assert_eq!(
        outcome.status,
        ApplyEffectSummonObjectSlotSessionStatusLikeCpp::InvalidSlot
    );
    assert_eq!(outcome.slot, Some(MAX_GAMEOBJECT_SLOT_LIKE_CPP));
    assert!(outcome.map_outcome.is_none());
}
#[tokio::test]
async fn summon_object_wild_focus_implicit_destination_uses_focus_position_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 708_i32;
    let template_entry = 9009_u32;
    let player_guid = ObjectGuid::create_player(1, 7010);
    let focus_guid = test_gameobject_guid(9011, 7011);
    let player_position = Position::new(180.0, 280.0, 38.0, 0.0);
    let focus_position = Position::new(184.0, 282.0, 38.5, 2.0);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 =
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(spell_id, 181, vec![summon_effect]),
    );
    add_canonical_spell_focus_gameobject_on_map_like_cpp(
        &canonical,
        focus_guid,
        9_011,
        181,
        10,
        focus_position,
        571,
        0,
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 708,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("focus implicit destination should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("focus-destination summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        focus_position,
        "C++ TARGET_OBJECT_TYPE_DEST emergency branch sets m_targets.dst from focusObject before EffectSummonObjectWild"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "focus-destination summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn summon_object_slot_focus_implicit_destination_uses_focus_position_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 709_i32;
    let template_entry = 9012_u32;
    let player_guid = ObjectGuid::create_player(1, 7012);
    let focus_guid = test_gameobject_guid(9013, 7013);
    let player_position = Position::new(190.0, 290.0, 39.0, 0.0);
    let focus_position = Position::new(193.0, 294.0, 39.5, 2.4);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_slot_effect_like_cpp(i32::try_from(template_entry).unwrap(), 0);
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY;
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(spell_id, 181, vec![summon_effect]),
    );
    add_canonical_spell_focus_gameobject_on_map_like_cpp(
        &canonical,
        focus_guid,
        9_013,
        181,
        10,
        focus_position,
        571,
        0,
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 709,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("focus implicit destination slotted summon should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("focus-destination slotted summon should be visible");
    let owner = managed
        .map()
        .get_typed_player(player_guid)
        .expect("player remains slot owner");
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[0],
        summoned_guid
    );
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        focus_position,
        "C++ m_targets.dst from focusObject is consumed by EffectSummonObject before its caster close-point fallback"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "focus-destination slotted summon should trigger represented visibility create/update delivery"
    );
}
