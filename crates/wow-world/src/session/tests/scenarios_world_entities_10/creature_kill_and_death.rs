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
