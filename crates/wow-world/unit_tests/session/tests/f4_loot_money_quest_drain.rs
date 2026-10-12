//! #1263 F4 LootMoney dependency (8): `CMSG_LOOT_MONEY` drains the money change
//! through the App objective owner, whose completion runs the visibility refresh.
//! C++: `LootHandler.cpp:142` → `Player::ModifyMoney`/`SetMoney`/`MoneyChanged`
//! (`Player.cpp:23350`/`:23376`/`:16162`) → `CompleteQuest` (`:14496`) →
//! `SetQuestStatus` → `SendQuestUpdate` → `UpdateVisibleGameobjectsOrSpellClicks`.

use super::*;

const QUEST_ID: u32 = 12_541;
const CLICK_NPC_ENTRY: u32 = 712;
const CLICK_SPELL_ID: u32 = 912;

/// 90 copper, a quest whose money objective needs 100, a conditioned
/// spell-click creature (visible or, as the control, not) and `coins` of loot.
fn loot_money_quest_session_like_cpp(
    coins: u32,
    creature_visible: bool,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 44);
    let creature_guid = test_creature_guid(132);
    let loot_owner = ObjectGuid::create_item(1, 61_950);
    let mut quest = test_quest_template(QUEST_ID);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: QUEST_ID * 10,
        quest_id: QUEST_ID,
        obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP,
        order: 0,
        storage_index: -1,
        object_id: 0,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });

    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
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
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical spell-click viewer map");
    session.set_player_gold_like_cpp(90);
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.insert_status_like_cpp(
                    QUEST_ID,
                    crate::handlers::quest::PlayerQuestStatus {
                        quest_id: QUEST_ID,
                        status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                        explored: false,
                        accept_time_secs: 0,
                        end_time_secs: 0,
                        objective_counts: vec![],
                        slot: 0,
                    },
                );
            })
            .is_some(),
        "quest fixture must be loaded into the canonical Player owner"
    );
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: CLICK_NPC_ENTRY,
            source_entry: CLICK_SPELL_ID as i32,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 999_999,
            ..Condition::default()
        }]),
    ));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: CLICK_NPC_ENTRY,
            spell_id: CLICK_SPELL_ID,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == CLICK_NPC_ENTRY,
        |spell| spell == CLICK_SPELL_ID,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        CLICK_NPC_ENTRY,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    if creature_visible {
        session
            .core
            .client_visible_guids_like_cpp
            .insert(creature_guid);
    }

    session.loot.set_active_loot_guid(loot_owner);
    session.loot.insert_cached_loot_for_owner_like_cpp(
        loot_owner,
        wow_loot::CreatureLoot {
            loot_guid: loot_owner,
            coins,
            unlooted_count: 0,
            loot_type: wow_packet::packets::loot::LOOT_TYPE_ITEM_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    session.set_loot_money_persistence_test_result_like_cpp(true);
    (session, send_rx)
}

/// Dispatch `CMSG_LOOT_MONEY` through its single production registration.
async fn dispatch_registered_loot_money_like_cpp(session: &mut WorldSession) {
    let entries = crate::session::registry::registered_handler_entries_like_cpp()
        .filter(|entry| entry.opcode == wow_constants::ClientOpcodes::LootMoney)
        .collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        1,
        "exactly one production LootMoney registration"
    );
    let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    catalogs.id_generators = Arc::new(session.id_generators_for_test_like_cpp());
    let mut pkt = wow_packet::WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();
    (entries[0].handler)(session, &catalogs, pkt).await;
}

fn quest_status_like_cpp(session: &WorldSession) -> Option<u8> {
    session
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| state.statuses_like_cpp().get(&QUEST_ID).map(|s| s.status))
}

/// Loot `coins` through the registered consumer and return the final copper,
/// quest status, the published server opcodes and their raw packets.
async fn loot_money_packets_like_cpp(
    coins: u32,
    creature_visible: bool,
) -> (Option<u64>, Option<u8>, Vec<ServerOpcodes>, Vec<Vec<u8>>) {
    let (mut session, send_rx) = loot_money_quest_session_like_cpp(coins, creature_visible);
    let _ = drain_server_packet_bytes(&send_rx);
    dispatch_registered_loot_money_like_cpp(&mut session).await;
    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    (
        session.resolved_player_money_like_cpp(),
        quest_status_like_cpp(&session),
        opcodes,
        packets,
    )
}

async fn loot_money_outcome_like_cpp(
    coins: u32,
    creature_visible: bool,
) -> (Option<u64>, Option<u8>, Vec<ServerOpcodes>) {
    let (money, status, opcodes, _) = loot_money_packets_like_cpp(coins, creature_visible).await;
    (money, status, opcodes)
}

/// #1359: the refresh publishes the creature's viewer-filtered NpcFlags. The
/// spell-click row's condition (aura 999999) fails, so C++
/// `Unit::BuildValuesUpdate`'s `ViewerDependentValue<NpcFlagsTag>` clears
/// `UNIT_NPC_FLAG_SPELLCLICK` for this viewer: the values update carries the
/// two NpcFlags words, both zero.
#[tokio::test]
async fn loot_money_completion_refresh_publishes_the_filtered_npc_flags_like_cpp() {
    let (_, _, _, packets) = loot_money_packets_like_cpp(10, true).await;
    let (_, _, _, control_packets) = loot_money_packets_like_cpp(10, false).await;

    let mut update = wow_packet::packets::update::UnitDataValuesDeltaUpdate::default();
    update.changed_object_type_mask = 1 << TYPEID_UNIT;
    update.unit_data_mask[113 / 32] |= 1 << (113 % 32);
    update.unit_data_mask[114 / 32] |= 1 << (114 % 32);
    update.npc_flags = [0, 0];
    let expected = wow_packet::packets::update::UpdateObject::unit_values_update(
        test_creature_guid(132),
        571,
        update,
    )
    .to_bytes();

    assert_eq!(
        packets.iter().filter(|bytes| **bytes == expected).count(),
        1,
        "the visible creature receives exactly one filtered NpcFlags update"
    );
    assert!(!control_packets.contains(&expected));
}

#[tokio::test]
async fn loot_money_completing_a_money_objective_refreshes_visibility_before_quest_update_complete_like_cpp()
 {
    let (money, status, opcodes) = loot_money_outcome_like_cpp(10, true).await;
    let (control_money, control_status, control_opcodes) =
        loot_money_outcome_like_cpp(10, false).await;

    assert_eq!(money, Some(100));
    assert_eq!(control_money, Some(100));
    assert_eq!(
        status,
        Some(crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP),
        "the drained money change completed the quest in the canonical owner"
    );
    assert_eq!(control_status, status);
    let complete = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::QuestUpdateComplete)
        .expect("quest completion is published");
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::QuestUpdateComplete)
            .count(),
        1
    );
    // The visible spell-click creature adds exactly one UpdateObject, and it is
    // published inside the completion, before the completion notice.
    let control_complete = control_opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::QuestUpdateComplete)
        .expect("control quest completion is published");
    let updates_before = |opcodes: &[ServerOpcodes], end: usize| {
        opcodes[..end]
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::UpdateObject)
            .count()
    };
    assert_eq!(
        updates_before(&opcodes, complete),
        updates_before(&control_opcodes, control_complete) + 1,
        "visible: {opcodes:?}; control: {control_opcodes:?}"
    );
    assert_eq!(
        opcodes[complete..],
        control_opcodes[control_complete..],
        "nothing after the completion notice depends on the refresh"
    );
}

#[tokio::test]
async fn loot_money_below_a_money_objective_neither_completes_nor_refreshes_like_cpp() {
    let (money, status, opcodes) = loot_money_outcome_like_cpp(5, true).await;
    let (_, _, control_opcodes) = loot_money_outcome_like_cpp(5, false).await;

    assert_eq!(money, Some(95));
    assert_eq!(
        status,
        Some(crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert!(
        !opcodes.contains(&ServerOpcodes::QuestUpdateComplete),
        "no completion below the objective amount: {opcodes:?}"
    );
    assert_eq!(
        opcodes, control_opcodes,
        "no visibility refresh without a completion"
    );
}
