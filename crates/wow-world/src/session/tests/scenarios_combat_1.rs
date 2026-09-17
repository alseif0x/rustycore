//! Session scenarios exercising the represented combat responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn combat_rating_multiplier_uses_gt_table_like_cpp() {
    let (mut session, _, _) = make_session();
    assert_eq!(session.combat_rating_multiplier_like_cpp(80, 8), 1.0);

    let mut columns = [0.0f32; wow_data::CombatRatingsGameTableLikeCpp::VALUE_COLUMN_COUNT];
    columns[wow_data::CombatRatingsEntryLikeCpp::CRIT_MELEE] = 45.905987;
    columns[wow_data::CombatRatingsEntryLikeCpp::DODGE] = 45.250187;
    session.set_combat_ratings_game_table(Arc::new(
        wow_data::CombatRatingsGameTableLikeCpp::from_rows([
            wow_data::CombatRatingsEntryLikeCpp::from_columns(columns),
        ]),
    ));

    assert!((session.combat_rating_multiplier_like_cpp(1, 8) - (1.0 / 45.905987)).abs() < 0.00001);
    assert!((session.combat_rating_multiplier_like_cpp(1, 2) - (1.0 / 45.250187)).abs() < 0.00001);
    assert_eq!(
        session.combat_rating_multiplier_like_cpp(1, 23),
        1.0,
        "C++ ratings without a CombatRatings column use default multiplier"
    );
}
#[tokio::test]
async fn player_kill_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let victim_guid = ObjectGuid::create_player(1, 84);
    let quest_id = 12_503;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: 0,
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
    session.killed_player_credit_like_cpp(victim_guid).await;

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
            ServerOpcodes::QuestUpdateAddPvpCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
}
#[tokio::test]
async fn player_kill_same_faction_objective_skips_opposite_team_victim_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let victim_guid = ObjectGuid::create_player(1, 84);
    let (victim_tx, _victim_rx) = flume::bounded(1);
    let registry = Arc::new(PlayerRegistry::default());
    let mut victim_info = broadcast_info(victim_guid, victim_tx);
    victim_info.identity.race = 2; // Orc/Horde; player test race defaults to Human/Alliance.
    registry.register_or_replace(victim_guid, victim_info, Default::default());

    let quest_id = 12_504;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
        order: 0,
        storage_index: 0,
        object_id: 0,
        amount: 1,
        flags: QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.player_race = 1;
    session.set_player_guid(Some(player_guid));
    session.set_player_registry(registry);
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
    session.killed_player_credit_like_cpp(victim_guid).await;

    let state = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("canonical Player quest state");
    let status = state
        .statuses_like_cpp()
        .get(&quest_id)
        .expect("quest remains");
    assert_eq!(status.objective_counts, vec![0]);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn legacy_only_npc_interaction_uses_combat_reach_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 44);
    let questgiver_guid = test_creature_guid(15);
    let manager = shared_map_manager();

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let mut creature = crate::map_manager::WorldCreature::new(
        questgiver_guid,
        505,
        Position::new(15.75, 0.0, 0.0, 0.0),
        100,
        80,
        1,
        2,
        0.0,
        1,
        35,
        wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
        0,
    );
    creature.creature.unit_mut().set_combat_reach(0.0);
    manager
        .write()
        .unwrap()
        .add_creature(571, 0, 0, 0, creature);
    session.set_map_manager(Arc::clone(&manager));

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        None,
        "C++ Player::GetNPCIfCanInteractWith uses creature combat reach + 4.0 plus both combat radii, not a fixed 8-yard radius"
    );

    manager
        .write()
        .unwrap()
        .find_creature_mut(571, 0, questgiver_guid)
        .unwrap()
        .creature
        .unit_mut()
        .set_combat_reach(1.5);

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 505,
            position: Position::new(15.75, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
}
#[test]
fn canonical_player_skills_follow_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_563);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SkillOwner".to_string(),
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
    let owned = HashMap::from([(
        333,
        RepresentedPlayerSkillLikeCpp {
            skill_id: 333,
            step: 1,
            value: 150,
            max: 225,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]);

    assert!(session.set_complete_player_skill_records_like_cpp(owned.clone(), 1));
    assert_eq!(
        session.resolved_player_skill_records_like_cpp(),
        Some(owned.clone())
    );
    assert_eq!(
        session.complete_player_skill_occupied_slots_like_cpp(),
        Some(1)
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
        session.resolved_player_skill_records_like_cpp(),
        Some(owned.clone())
    );

    let replacement_record = wow_entities::PlayerSkillRecord {
        skill_line_id: 202,
        current_value: 300,
        max_value: 300,
        step: 1,
        profession_slot: 1,
        state: wow_entities::PlayerSkillLoadState::Unchanged,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.replace_skill_records_like_cpp(
        vec![replacement_record.clone()],
        true,
        true,
        Some(1),
        BTreeSet::new(),
    );
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_player_skill_records_like_cpp(), None);
    assert_eq!(session.resolved_player_skill_value_like_cpp(333), None);
    assert!(!session.set_complete_player_skill_records_like_cpp(owned, 1));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.skill_records_like_cpp().to_vec()
            }),
        Some(vec![replacement_record])
    );
}
#[test]
fn canonical_player_damage_control_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_576);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DamageControlOwner".to_string(),
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

    session.set_player_cheat_god_like_cpp(true);
    session.set_player_normal_damage_immune_like_cpp(true);
    session.set_player_environmental_damage_immune_like_cpp(true);
    assert_eq!(
        session.resolved_player_damage_control_like_cpp(),
        Some(wow_entities::PlayerDamageControlStateLikeCpp {
            cheat_god: true,
            normal_damage_immune: true,
            environmental_damage_immune: true,
        })
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert!(
        session
            .resolved_player_damage_control_like_cpp()
            .is_some_and(|state| state.cheat_god)
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_normal_damage_immune_like_cpp(true);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_player_damage_control_like_cpp(), None);
    session.set_player_cheat_god_like_cpp(true);
    session.set_player_environmental_damage_immune_like_cpp(true);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .damage_control_like_cpp()),
        Some(wow_entities::PlayerDamageControlStateLikeCpp {
            cheat_god: false,
            normal_damage_immune: true,
            environmental_damage_immune: false,
        })
    );
}
#[test]
fn visibility_distance_adds_both_combat_reaches_with_cpp_strict_boundary() {
    let source = Position::ZERO;
    let inside = Position::new(103.49, 0.0, 500.0, 0.0);
    let boundary = Position::new(103.5, 0.0, 0.0, 0.0);

    assert!(crate::session_rules::visibility_distance_allows_like_cpp(
        &source, 1.5, &inside, 2.0, 100.0,
    ));
    assert!(
        !crate::session_rules::visibility_distance_allows_like_cpp(
            &source, 1.5, &boundary, 2.0, 100.0,
        ),
        "C++ Position::IsInDist2d uses a strict less-than comparison"
    );
}
#[test]
fn death_sync_preserves_existing_canonical_death_time_ms() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_001);
    register_test_creature(&mut session, manager.clone(), guid, 40);

    {
        let mut manager = manager.write().unwrap();
        let world_creature = manager.find_creature_mut(0, 0, guid).unwrap();
        world_creature.creature.mark_ai_dead(1_234);
    }
    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(40);
        })
        .unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(
        world_creature.creature.ai_ownership().death_time_ms,
        Some(1_234)
    );
    assert_eq!(world_creature.current_hp(), 0);
}
#[tokio::test]
async fn heal_max_health_zero_damage_missing_target_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 49);
    let missing_creature_guid = test_creature_guid(18_015);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(65, 100);

    session
        .apply_heal_max_health_like_cpp(0, 0, player_guid, missing_creature_guid)
        .await
        .expect("C++ !unitTarget guard makes heal-max-health a no-op");

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn primary_heal_max_health_nonzero_damage_heals_target_missing_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 733_i32;
    let player_guid = ObjectGuid::create_player(1, 50);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(35, 80);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH,
            effect_base_points: 1,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented primary heal-max-health should execute");

    assert_eq!(
        session.player_health_like_cpp(),
        80,
        "C++ damage != 0 heals the missing health of the unit target"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[test]
fn combat_tick_uses_canonical_player_victim_when_session_target_is_empty_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_014);
    let player = ObjectGuid::create_player(1, 63);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Attacker".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(guid));
            player.unit_mut().set_target(guid);
            player
                .unit_mut()
                .add_unit_state(UnitState::MELEE_ATTACKING.bits());
        })
        .unwrap();
    session.combat_target = None;
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert!(world_creature.current_hp() < 40);
    assert_eq!(session.combat_target, Some(guid));
}
#[test]
fn combat_tick_uses_canonical_player_base_attack_timer_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_016);
    let player = ObjectGuid::create_player(1, 65);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Timer".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_attacking(Some(guid));
            player.unit_mut().set_target(guid);
            player
                .unit_mut()
                .add_unit_state(UnitState::MELEE_ATTACKING.bits());
            player
                .unit_mut()
                .set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            player
                .unit_mut()
                .set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
            let mut effective = *player.effective_combat_stats_like_cpp();
            effective.weapon_damage[WeaponAttackType::BaseAttack as usize] = [11.0, 11.0];
            player.replace_effective_combat_stats_like_cpp(effective);
        })
        .unwrap();
    session.combat_target = None;
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();
    let after_first = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(
        after_first, 29,
        "melee consumes the canonical Player effective weapon range instead of Unit's stale mirror"
    );

    session.tick_combat_sync();
    let after_second = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(after_second, after_first);
}
#[test]
fn combat_tick_los_failure_resets_timer_without_damage_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_022);
    let player = ObjectGuid::create_player(1, 71);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Los".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    // Drive the swing with line of sight denied. The retired
    // `player_melee_los_to_target_like_cpp` field was the only writer of this
    // value and had no production writer at all, so #28 made it a parameter of
    // the lifted function; this is the branch the field existed to reach.
    let swings = session
        .mutate_canonical_player_like_cpp(|player| {
            take_canonical_player_attack_swings_like_cpp(
                player,
                0,
                true,
                true,
                false,
                [crate::session::RepresentedMeleeDamageBonusLikeCpp::NONE; 2],
                crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                Default::default(),
            )
        })
        .flatten();
    assert!(
        swings.is_none_or(|(swings, _)| swings.is_empty()),
        "a swing with no line of sight must produce no damage"
    );

    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(hp, 40);
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        2_000
    );
}
#[test]
fn combat_tick_canonical_player_without_victim_does_not_use_stale_session_target_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_028);
    let player = ObjectGuid::create_player(1, 77);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "NoVictim".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);

    session.tick_combat_sync();

    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        40
    );
    assert_eq!(session.resolved_combat_target_like_cpp(), Some(None));
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(false));
}
#[test]
fn combat_tick_without_melee_attacking_state_skips_update_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_029);
    let player = ObjectGuid::create_player(1, 78);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "NotMelee".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        40
    );
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        0
    );
}

#[test]
fn white_swing_applies_autoattack_damage_auras_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_030);
    let player = ObjectGuid::create_player(1, 84);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Swing".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let aura_spell_id = 90_997_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        aura_spell_id,
        wow_data::SpellInfo {
            spell_id: aura_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    [crate::session::RepresentedMeleeDamageBonusLikeCpp::NONE; 2],
                    crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                    Default::default(),
                )
            })
            .flatten()
            .map(|(swings, _)| {
                swings
                    .into_iter()
                    .map(|swing| swing.damage)
                    .collect::<Vec<_>>()
            })
    };

    assert_eq!(
        swing(&mut session),
        Some(vec![7]),
        "the white swing uses the canonical effective weapon range"
    );

    session
        .apply_aura(aura_spell_id, player, 30_000, 1)
        .expect("apply autoattack damage aura");
    assert_eq!(
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player.unit().mod_autoattack_damage_pct_like_cpp()
            })
            .expect("canonical player"),
        2.0
    );
    assert_eq!(
        swing(&mut session),
        Some(vec![14]),
        "C++ MeleeDamageBonusDone adds the SPELL_AURA_MOD_AUTOATTACK_DAMAGE percentage"
    );

    session
        .remove_aura(0)
        .expect("remove autoattack damage aura");
    assert_eq!(swing(&mut session), Some(vec![7]));
}

#[test]
fn white_swing_applies_creature_type_melee_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_031);
    let player = ObjectGuid::create_player(1, 85);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Versus".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    // Template creature type 7 (undead) and the matching `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS`
    // and `SPELL_AURA_MOD_DAMAGE_DONE_CREATURE` effects.
    session.set_creature_template_lifecycle_store_like_cpp(Arc::new(
        wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                entry: 9001,
                creature_type: 7,
                ..Default::default()
            },
        ]),
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura, amount) in [
        (
            90_998_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            100,
        ),
        (
            90_999_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
            5,
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura,
                    effect_misc_value_1: 1 << 6,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        // Hoisted: the bonus resolves the canonical snapshot, so it must not run
        // inside the mutable owner borrow.
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                    Default::default(),
                )
            })
            .flatten()
            .map(|(swings, _)| {
                swings
                    .into_iter()
                    .map(|swing| swing.damage)
                    .collect::<Vec<_>>()
            })
    };

    // Without the versus aura the flat creature-type benefit applies alone.
    let base = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(base[0].flat, 0);

    session
        .apply_aura(90_999, player, 30_000, 1)
        .expect("apply flat creature-type aura");
    let with_flat = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(with_flat[0].flat, 5);
    assert_eq!(with_flat[0].pct, 1.0);

    session
        .apply_aura(90_998, player, 30_000, 1)
        .expect("apply versus creature-type aura");
    let with_pct = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(with_pct[0].flat, 5);
    assert_eq!(with_pct[0].pct, 2.0);

    // `((7 + 5) * 2.0)`.
    assert_eq!(
        swing(&mut session),
        Some(vec![24]),
        "C++ MeleeDamageBonusDone"
    );
}

#[test]
fn white_swing_applies_victim_aurastate_and_mechanic_melee_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_032);
    let player = ObjectGuid::create_player(1, 86);
    // C++ `MECHANIC_STUN` (`SharedDefines.h:2552`).
    const MECHANIC_STUN: i32 = 12;
    // The victim aura carrier: a creature aura whose `SpellInfo::Mechanic` is
    // the STUN mechanic, so `Unit::HasAuraWithMechanic(1 << 12)` matches.
    const VICTIM_MECHANIC_SPELL: i32 = 70_001;

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "VictimState".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura, amount, misc) in [
        (
            91_100_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            100,
            i32::from(wow_entities::AURA_STATE_WOUNDED_20_PERCENT),
        ),
        (
            91_101_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            100,
            MECHANIC_STUN,
        ),
        (
            91_102_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            100,
            13,
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura,
                    effect_misc_value_1: misc,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        VICTIM_MECHANIC_SPELL,
        0,
        wow_data::SpellHitMetadataLikeCpp {
            spell_mechanic: MECHANIC_STUN as i8,
            ..Default::default()
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        // Hoisted: the bonus resolves the canonical snapshot, so it must not run
        // inside the mutable owner borrow.
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                    Default::default(),
                )
            })
            .flatten()
            .map(|(swings, _)| {
                swings
                    .into_iter()
                    .map(|swing| swing.damage)
                    .collect::<Vec<_>>()
            })
    };

    // At full health the wounded-aurastate aura misses and no creature aura
    // carries the mechanic, so both terms stay neutral.
    session
        .apply_aura(91_100, player, 30_000, 1)
        .expect("apply versus-aurastate aura");
    session
        .apply_aura(91_101, player, 30_000, 1)
        .expect("apply versus-mechanic aura");
    session
        .apply_aura(91_102, player, 30_000, 1)
        .expect("apply non-matching versus-mechanic aura");
    let healthy = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(healthy[0].flat, 0);
    assert_eq!(healthy[0].pct, 1.0);

    // C++ `Unit::Update` sets `AURA_STATE_WOUNDED_20_PERCENT` for a living unit
    // below 20% health; 7/40 is below it.
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.unit_mut().set_health(7);
        })
        .expect("wounded victim");
    let wounded = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(wounded[0].flat, 0);
    assert_eq!(wounded[0].pct, 2.0);

    // The victim's STUN aura completes `HasAuraWithMechanic(1 << 12)`; the
    // FREEZE-misc aura does not.
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    VICTIM_MECHANIC_SPELL as u32,
                    player,
                    0,
                    0,
                ));
        })
        .expect("victim mechanic aura");
    let stunned = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(stunned[0].flat, 0);
    assert_eq!(stunned[0].pct, 4.0);

    // `((7 + 0) * 4.0)`.
    assert_eq!(
        swing(&mut session),
        Some(vec![28]),
        "C++ MeleeDamageBonusDone"
    );
}
