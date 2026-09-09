//! Spell scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_personal_loot_remote_quest_and_spell_conditions_use_registry_like_cpp() {
    let (session, _) = make_session_with_send_capacity(1);
    let mut active_quest_statuses = HashMap::new();
    active_quest_statuses.insert(100, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    active_quest_statuses.insert(200, crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP);
    let mut rewarded_quests = HashSet::new();
    rewarded_quests.insert(300);
    let remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: vec![12_345],
        active_quest_statuses,
        active_quest_objective_counts: HashMap::new(),
        rewarded_quests,
        inventory_item_counts: HashMap::new(),
        is_current: false,
    };

    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(9, 100, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(28, 200, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(8, 300, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(14, 400, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        remote_context.quest_status(300),
        QUEST_STATUS_REWARDED_LIKE_CPP
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(14, 300, 0, 0),
            &remote_context,
        ),
        Some(false),
        "C++ Player::GetQuestStatus returns REWARDED before QUEST_STATUS_NONE"
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(25, 12_345, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(47, 100, 0x08, 0),
            &remote_context,
        ),
        Some(true)
    );
}
#[tokio::test]
async fn loot_unit_valid_target_interrupts_active_cast_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_034);
    session.set_player_guid(Some(player_guid));
    install_active_spell_cast(&mut session, player_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
}
#[tokio::test]
async fn loot_unit_valid_target_removes_looting_interrupt_auras_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_035);
    session.set_player_guid(Some(player_guid));
    install_visible_aura_with_interrupt_flags(
        &mut session,
        3,
        777,
        player_guid,
        SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
    );
    install_visible_aura_with_interrupt_flags(&mut session, 4, 778, player_guid, 0);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(!session.visible_auras.contains_key(&3));
    assert!(session.visible_auras.contains_key(&4));
}
#[tokio::test]
async fn loot_roll_need_vote_broadcasts_immediate_roll_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(5);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_052);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(5);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let _start_roll = send_rx.try_recv().unwrap();
    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let _remote_start_roll = candidate_rx.try_recv().unwrap();
    session
        .handle_loot_roll(LootRoll {
            loot_obj: loot_object,
            loot_list_id: 0,
            roll_type: ROLL_VOTE_NEED_LIKE_CPP,
        })
        .await;

    let local_roll = send_rx.try_recv().unwrap();
    let mut local_roll = WorldPacket::from_bytes(&local_roll);
    assert_eq!(
        local_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_roll.read_packed_guid().unwrap(), player_guid);
    assert_eq!(local_roll.read_int32().unwrap(), 0);
    assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_NEED_LIKE_CPP);
    assert_eq!(local_roll.read_int32().unwrap(), 0);
    assert_eq!(local_roll.read_bits(2).unwrap(), 0);
    assert_eq!(local_roll.read_bits(3).unwrap(), 1);

    let remote_roll = candidate_rx.try_recv().unwrap();
    let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
    assert_eq!(
        remote_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_roll.read_packed_guid().unwrap(), player_guid);
}
