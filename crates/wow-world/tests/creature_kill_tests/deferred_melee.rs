//! Original melee enqueue→full pending driver→reward/quest/death case.
use super::*;

#[tokio::test]
async fn combat_tick_kill_keeps_empty_creature_loot_non_lootable_after_pending_drain_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_015);
    let player = ObjectGuid::create_player(1, 63);
    session.fixture_kill_prepare_melee(player, guid, 1, 0, 400);
    let mut quest_store = wow_data::quest::QuestStore::new();
    quest_store.quests.insert(
        9_001,
        wow_data::quest::QuestTemplate {
            id: 9_001,
            quest_type: 0,
            quest_level: 1,
            quest_max_scaling_level: 0,
            quest_package_id: 0,
            min_level: 1,
            quest_sort_id: 0,
            quest_info_id: 0,
            suggested_group_num: 0,
            reward_next_quest: 0,
            reward_xp_difficulty: 0,
            reward_xp_multiplier: 1.0,
            reward_money_difficulty: 0,
            reward_money_multiplier: 1.0,
            reward_bonus_money: 0,
            reward_display_spell: [0; wow_data::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
            reward_spell: 0,
            reward_honor: 0,
            reward_title_id: 0,
            reward_skill_line_id: 0,
            reward_skill_points: 0,
            reward_mail_template_id: 0,
            reward_mail_delay_secs: 0,
            reward_mail_sender_entry: 0,
            reward_faction_ids: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_values: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_overrides: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_cap_in: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_flags: 0,
            source_item_id: 0,
            source_item_count: 0,
            source_spell_id: 0,
            limit_time_secs: 0,
            expansion: 0,
            flags: 0,
            flags_ex: 0,
            flags_ex2: 0,
            special_flags: 0,
            event_id_for_quest: 0,
            reward_items: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
            reward_amounts: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
            reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
            reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
            item_drop: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
            item_drop_quantity: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
            log_title: String::new(),
            log_description: String::new(),
            quest_description: String::new(),
            area_description: String::new(),
            quest_completion_log: String::new(),
            objectives: vec![wow_data::quest::QuestObjective {
                id: 1,
                quest_id: 9_001,
                obj_type: 0,
                order: 0,
                storage_index: 0,
                object_id: 9001,
                amount: 1,
                flags: 0,
                flags2: 0,
                progress_bar_weight: 0.0,
                description: String::new(),
            }],
            allowable_races: 0,
            allowable_classes: 0,
            max_level: 0,
            prev_quest_id: 0,
            next_quest_id: 0,
            exclusive_group: 0,
            breadcrumb_for_quest_id: 0,
            dependent_previous_quests: Vec::new(),
            dependent_breadcrumb_quests: Vec::new(),
            required_min_rep_faction: 0,
            required_min_rep_value: 0,
            required_max_rep_faction: 0,
            required_max_rep_value: 0,
            required_skill_id: 0,
            required_skill_points: 0,
            reward_choice_items: [(0, 0); wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
            reward_choice_item_types: [0; wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
        },
    );
    session.set_quest_store(Arc::new(quest_store));
    insert_player_quest_gameplay_status_for_test(
        &mut session,
        9_001,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 9_001,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );
    register_test_creature(&mut session, manager.clone(), guid, 3);
    session
        .fixture_melee_mutate_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.fixture_kill_tick_combat();

    assert!(session.fixture_kill_loot(guid).is_none());
    assert_eq!(
        session.fixture_kill_observations().pending_loot,
        &vec![guid]
    );

    process_pending_for_loot_test(&mut session).await;

    let loot = session
        .fixture_kill_loot(guid)
        .expect("melee kill loot is generated from pending bridge");
    assert!(loot.allowed_looters.contains(&player));
    assert_eq!(loot.loot_type, LOOT_TYPE_CORPSE_LIKE_CPP);
    assert_eq!((loot.coins, loot.unlooted_count), (0, 0));
    assert!(session.character_player_xp_for_test() > 0);
    let quests = player_quest_gameplay_snapshot_for_test(&session).unwrap();
    let quest = quests.status_like_cpp(9_001).unwrap();
    assert_eq!(
        quest.status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_eq!(quest.objective_counts, vec![1]);
    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert!(world_creature.creature.is_tapped_by(player));
    assert!(
        !world_creature
            .creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::Lootable as u32)
    );
    assert!(
        !world_creature
            .creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::SKINNABLE)
    );
    assert_eq!(
        session.fixture_kill_observations().events,
        &[
            RepresentedCreatureKillEventLikeCpp::KillerProc {
                attacker_guid: player,
                victim_guid: guid,
            },
            RepresentedCreatureKillEventLikeCpp::TapperTargetDiesProc {
                tapper_guid: player,
                victim_guid: guid,
            },
            RepresentedCreatureKillEventLikeCpp::VictimDeathProc { victim_guid: guid },
            RepresentedCreatureKillEventLikeCpp::DeliveredKillingBlowCriteria {
                player_guid: player,
                victim_guid: guid,
                quantity: 1,
            },
            RepresentedCreatureKillEventLikeCpp::DeathStateJustDied { victim_guid: guid },
            RepresentedCreatureKillEventLikeCpp::ZoneScriptUnitDeath { unit_guid: guid },
            RepresentedCreatureKillEventLikeCpp::LootFlagsApplied {
                creature_guid: guid,
                lootable: false,
                can_skin: false,
                skinnable: false,
            },
            RepresentedCreatureKillEventLikeCpp::CreatureOnHealthDepletedAi {
                creature_guid: guid,
                attacker_guid: player,
                is_kill: true,
            },
            RepresentedCreatureKillEventLikeCpp::CreatureJustDiedAi {
                creature_guid: guid,
                killer_guid: player,
            },
            RepresentedCreatureKillEventLikeCpp::ScriptMgrOnCreatureKill {
                killer_guid: player,
                creature_guid: guid,
            },
        ]
    );
}
