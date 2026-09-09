//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn creature_kill_target_dies_proc_filters_group_reward_distance_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_007);
    let player = ObjectGuid::create_player(1, 48);
    let near_member = ObjectGuid::create_player(1, 49);
    let far_member = ObjectGuid::create_player(1, 50);
    let (near_tx, _near_rx) = flume::bounded(10);
    let (far_tx, _far_rx) = flume::bounded(10);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut near_info = broadcast_info(near_member, near_tx);
    near_info.placement.position = Position::new(20.0, 10.0, 0.0, 0.0);
    let mut far_info = broadcast_info(far_member, far_tx);
    far_info.placement.position = Position::new(200.0, 10.0, 0.0, 0.0);
    player_registry.register_or_replace(near_member, near_info, Default::default());
    player_registry.register_or_replace(far_member, far_info, Default::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player);
    group.add_member(near_member);
    group.add_member(far_member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.player_guid = Some(player);
    session.player_position = Some(Position::new(10.0, 10.0, 0.0, 0.0));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
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
    register_test_creature(&mut session, manager.clone(), guid, 40);

    session.apply_damage(None, guid, 100).await.unwrap();

    let target_dies_tappers: Vec<_> = session
        .represented_creature_kill_events_like_cpp()
        .iter()
        .filter_map(|event| match event {
            RepresentedCreatureKillEventLikeCpp::TapperTargetDiesProc {
                tapper_guid,
                victim_guid,
            } if *victim_guid == guid => Some(*tapper_guid),
            _ => None,
        })
        .collect();
    assert_eq!(target_dies_tappers, vec![player, near_member]);
    assert!(
        !target_dies_tappers.contains(&far_member),
        "C++ Player::IsAtGroupRewardDistance suppresses TARGET_DIES proc for far tappers"
    );
    let authority = session
        .mutate_world_creature(guid, |creature| {
            creature.creature.loot_authority_like_cpp().clone()
        })
        .expect("dead creature retains its object-owned loot authority");
    assert!(
        authority.personal_snapshot_like_cpp(far_member).is_some(),
        "C++ overworld loot still creates an independent pool for every connected tapper, regardless of group reward distance"
    );
    let selected_loot = session
        .loot_table
        .get(&guid)
        .expect("the handling session retains only its personal loot view");
    assert_eq!(selected_loot.allowed_looters, vec![player]);
}
#[tokio::test]
async fn creature_kill_notifies_current_tapper_pet_after_death_state_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_008);
    let player = ObjectGuid::create_player(1, 51);
    let pet_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 500, 52);
    session.player_guid = Some(player);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    register_test_creature(&mut session, manager.clone(), guid, 40);

    session.apply_damage(None, guid, 100).await.unwrap();

    assert_eq!(
        session.represented_creature_kill_events_like_cpp(),
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
            RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi {
                tapper_guid: player,
                pet_guid,
                victim_guid: guid,
            },
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
#[tokio::test]
async fn creature_kill_sets_skinning_flags_when_skin_loot_template_exists_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_009);
    let player = ObjectGuid::create_player(1, 52);
    let skin_loot_id = 777;
    let mut skinning_store = wow_loot::LootStore::for_kind_like_cpp(LootStoreKind::Skinning);
    skinning_store
        .load_rows_like_cpp(
            [wow_loot::LootTemplateRow {
                entry: skin_loot_id,
                item: wow_loot::LootStoreItem {
                    item_id: 9_002,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: 1,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Skinning, skinning_store);
    session.set_loot_stores(Arc::new(stores));
    session.player_guid = Some(player);
    session.set_map_manager(manager);
    session.current_map_id = 0;
    session.register_world_creature(
        0,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(guid, 9001, 40),
        3,
        5,
        20.0,
        0,
        skin_loot_id,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );

    session.apply_damage(None, guid, 100).await.unwrap();

    let manager = session.map_manager.as_ref().unwrap().read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert!(
        world_creature
            .creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::CanSkin as u32)
    );
    assert!(
        world_creature
            .creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::SKINNABLE)
    );
    assert!(
        session
            .represented_creature_kill_events_like_cpp()
            .contains(&RepresentedCreatureKillEventLikeCpp::LootFlagsApplied {
                creature_guid: guid,
                lootable: false,
                can_skin: true,
                skinnable: true,
            })
    );
}
#[tokio::test]
async fn spell_damage_skips_dead_creature_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_010);
    session.player_guid = Some(ObjectGuid::create_player(1, 53));
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(20);
            creature
                .creature
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
            creature.creature.clear_data_changes();
        })
        .unwrap();

    session.apply_damage(None, guid, 7).await.unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 20);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn spell_instakill_effect_row_kills_creature_and_logs_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 728_i32;
    let guid = test_creature_guid(18_013);
    let player_guid = ObjectGuid::create_player(1, 56);
    session.player_guid = Some(player_guid);
    session.set_player_level_like_cpp(80);
    register_test_creature(&mut session, manager.clone(), guid, 40);

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_INSTAKILL,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("represented instakill effect row should execute");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 0);
    assert!(
        !world_creature.creature.is_alive(),
        "C++ EffectInstaKill delegates to Unit::Kill after logging"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellInstakillLog,
            ServerOpcodes::CooldownEvent
        ]
    );
    let mut instakill = wow_packet::WorldPacket::from_bytes(&packets[1]);
    assert_eq!(
        instakill.read_uint16().expect("opcode"),
        ServerOpcodes::SpellInstakillLog as u16
    );
    assert_eq!(instakill.read_packed_guid().expect("target"), guid);
    assert_eq!(instakill.read_packed_guid().expect("caster"), player_guid);
    assert_eq!(instakill.read_int32().expect("spell id"), spell_id);
    assert!(instakill.is_empty());
}
#[tokio::test]
async fn spell_heal_syncs_canonical_creature_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_003);
    session.player_guid = Some(ObjectGuid::create_player(1, 43));
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(20);
            creature.creature.clear_data_changes();
        })
        .unwrap();

    session.apply_heal(None, guid, 7).await.unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 27);
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
}
#[tokio::test]
async fn spell_heal_skips_dead_creature_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_004);
    session.player_guid = Some(ObjectGuid::create_player(1, 45));
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(20);
            creature
                .creature
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
            creature.creature.clear_data_changes();
        })
        .unwrap();

    session.apply_heal(None, guid, 7).await.unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 20);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn spell_self_heal_adds_half_effective_heal_threat_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 144);
    let tank_guid = ObjectGuid::create_player(1, 145);
    let creature_guid = test_creature_guid(18_144);
    let heal_spell_id = 18_144;
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ThreatHealer".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(50, 100);
    // `register_world_creature` bootstraps the legacy facade in instance
    // zero. Configure that fixture before the canonical Player makes the
    // session resolve legacy mutations through the test instance below.
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 7);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9_001,
        position,
        0,
        7,
        80,
    );
    {
        let mut legacy = manager.write().unwrap();
        let creature = legacy
            .remove_creature_any(0, 0, creature_guid)
            .expect("move the represented creature into the test instance");
        let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        legacy.add_creature(0, 7, grid_x, grid_y, creature);
    }
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(tank_guid);
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.add_threat(tank_guid, 100.0);
            combat.add_threat(player_guid, 10.0);
        })
        .unwrap();
    session.sync_represented_creature_threat_to_canonical_like_cpp(
        creature_guid,
        player_guid,
        10.0,
    );
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: HashMap::from([(
            heal_spell_id as u32,
            wow_data::SpellThreatEntryLikeCpp {
                flat_mod: 4,
                pct_mod: 2.0,
                ap_pct_mod: 0.0,
            },
        )]),
    }));
    let mut heal_spell_store = wow_data::SpellStore::new();
    heal_spell_store.insert(
        heal_spell_id,
        threat_spell_info_like_cpp(
            heal_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
            20,
        ),
    );
    session.set_spell_store(Arc::new(heal_spell_store));

    session
        .execute_spell(heal_spell_id, player_guid)
        .await
        .unwrap();

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 7, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, Some(38.0));
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 7, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        Some(tank_guid),
        "C++ heal threat does not bypass the regular victim-selection thresholds"
    );
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(38.0)
    );

    let no_helpful_threat_spell_id = 18_145;
    let mut no_helpful_threat_store = wow_data::SpellStore::new();
    let mut attributes = [0; 15];
    attributes[4] = wow_data::spell::attributes::SPELL_ATTR4_NO_HELPFUL_THREAT;
    no_helpful_threat_store
        .insert_spell_misc_attributes_like_cpp(no_helpful_threat_spell_id, attributes);
    session.set_spell_store(Arc::new(no_helpful_threat_store));
    session
        .apply_heal(Some(no_helpful_threat_spell_id), player_guid, 10)
        .await
        .unwrap();
    assert_eq!(session.player_health_like_cpp(), 80);
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 7, creature_guid)
            .unwrap()
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(player_guid),
        Some(38.0),
        "C++ ForwardThreatForAssistingMe returns before forwarding SPELL_ATTR4_NO_HELPFUL_THREAT heals"
    );
}
#[tokio::test]
async fn spell_threat_effect_adds_creature_threat_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 787_i32;
    let player_guid = ObjectGuid::create_player(1, 787);
    let creature_guid = test_creature_guid(18_787);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ThreatCaster".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT,
            35,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented EffectThreat should execute");

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, Some(35.0));
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(35.0)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_modify_threat_percent_scales_existing_creature_threat_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 788_i32;
    let player_guid = ObjectGuid::create_player(1, 788);
    let creature_guid = test_creature_guid(18_788);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ThreatScaler".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(player_guid, 120.0);
        })
        .unwrap();
    session.sync_represented_creature_threat_to_canonical_like_cpp(
        creature_guid,
        player_guid,
        120.0,
    );
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT,
            -50,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented EffectModifyThreatPercent should execute");

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, Some(60.0));
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(60.0)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_taunt_effect_matches_caster_threat_to_highest_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 793_i32;
    let player_guid = ObjectGuid::create_player(1, 793);
    let other_player_guid = ObjectGuid::create_player(1, 1793);
    let creature_guid = test_creature_guid(18_793);
    let position = Position::new(10.0, 10.0, 0.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let registry = Arc::new(PlayerRegistry::default());
    let (observer_tx, _observer_rx) = flume::bounded(4);
    let (observer_command_tx, observer_command_rx) = flume::bounded(4);
    let observer_guid = ObjectGuid::create_player(1, 2793);
    let mut observer = broadcast_info_with_command(observer_guid, observer_tx, observer_command_tx);
    observer.placement.position = position;
    registry.register_or_replace(observer_guid, observer, Default::default());
    session.set_player_registry(registry);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TauntCaster".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 0);
    add_canonical_test_player_on_map(&canonical, observer_guid, position, 0, 0);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9001,
        position,
        0,
        0,
        80,
    );
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    session
        .mutate_world_creature(creature_guid, |creature| {
            let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
            combat.add_threat(other_player_guid, 120.0);
            combat.add_threat(player_guid, 5.0);
        })
        .unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    let mut taunt_spell = threat_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        0,
    );
    taunt_spell.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT;
    spell_store.insert(spell_id, taunt_spell);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            duration_index: 7,
            spell_id: spell_id as u32,
            ..Default::default()
        },
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 7,
            duration: 3_000,
            duration_per_level: 0,
            max_duration: 3_000,
        },
    ])));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented EffectTaunt should execute");

    let manager_guard = manager.read().unwrap();
    let combat = &manager_guard
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat;
    let legacy_threat = combat.threat_value(player_guid);
    assert_eq!(legacy_threat, Some(120.0));
    assert!(
        combat
            .threat_ref(player_guid)
            .is_some_and(wow_entities::ThreatReferenceState::is_taunting),
        "C++ MOD_TAUNT forces the caster above ordinary threat for the aura duration"
    );
    drop(manager_guard);
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(120.0)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::AuraUpdate,
            ServerOpcodes::CooldownEvent
        ]
    );
    let observer_command = observer_command_rx
        .try_recv()
        .expect("nearby sessions receive the taunt AuraUpdate command");
    let SessionCommand::SendIfVisibleLikeCpp(observer_command) = observer_command else {
        panic!("expected visibility-gated taunt fanout");
    };
    assert_eq!(
        u16::from_le_bytes([
            observer_command.packet_bytes[0],
            observer_command.packet_bytes[1],
        ]),
        ServerOpcodes::AuraUpdate as u16
    );
}
#[tokio::test]
async fn spell_threat_effect_skips_dead_caster_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 789_i32;
    let player_guid = ObjectGuid::create_player(1, 789);
    let creature_guid = test_creature_guid(18_789);
    let manager = shared_map_manager();
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(0, 100);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        threat_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT,
            35,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("dead caster EffectThreat should execute as C++ no-op");

    let legacy_threat = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .creature
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid);
    assert_eq!(legacy_threat, None);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
