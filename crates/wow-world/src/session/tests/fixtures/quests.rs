//! Quest catalog and quest-packet fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn test_quest_template(id: u32) -> wow_data::quest::QuestTemplate {
    crate::quest_template_fixture::quest_template_row_for_test(id, 0, String::new())
}

pub(in crate::session::tests) fn seasonal_test_quest_template(
    id: u32,
    quest_sort_id: i32,
    event_id_for_quest: u16,
) -> wow_data::quest::QuestTemplate {
    let mut quest = test_quest_template(id);
    quest.quest_sort_id = quest_sort_id;
    quest.min_level = 0;
    quest.event_id_for_quest = event_id_for_quest;
    quest
}

pub(in crate::session::tests) fn seasonal_quest_store_like_cpp(
    ids: impl IntoIterator<Item = u32>,
) -> wow_data::quest::QuestStore {
    wow_data::quest::QuestStore::from_quests_like_cpp(
        ids.into_iter()
            .map(|id| seasonal_test_quest_template(id, -376, 9)),
    )
}

/// Move an old handle-less quest fixture onto the same generation-checked
/// canonical `Player` owner exercised by production before the behavior under
/// test can trigger map/visibility work and establish that owner itself.
pub(in crate::session::tests) fn adopt_player_quest_fixture_into_canonical_owner_like_cpp(
    session: &mut WorldSession,
) {
    assert!(session.player_handle_like_cpp.is_none());
    let quests = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("handle-less quest fixture");
    let currencies = session
        .player_currencies_like_cpp()
        .expect("handle-less currency fixture");
    let gold = session.player_gold_like_cpp();
    let reputation = session.reputation_mgr_like_cpp().cloned_state_like_cpp();
    install_canonical_player_owner_for_test(session, 0, 0);
    session.set_item_guid_generator_like_cpp(Arc::new(wow_core::ObjectGuidGenerator::new(
        HighGuid::Item,
        1,
    )));
    assert!(session.set_player_currencies_like_cpp(currencies));
    session.set_player_gold_like_cpp(gold);
    assert!(
        session
            .mutate_reputation_mgr_like_cpp(|manager| manager.replace_state_like_cpp(reputation))
            .is_some()
    );
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| *state = quests)
            .is_some(),
        "quest fixture must move to the canonical Player owner"
    );
}

pub(in crate::session::tests) fn assert_canonical_quest_status_like_cpp(
    session: &WorldSession,
    quest_id: u32,
    expected_status: Option<u8>,
    expected_rewarded: bool,
) {
    let state = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("canonical Player quest state");
    assert_eq!(
        state
            .statuses_like_cpp()
            .get(&quest_id)
            .map(|status| status.status),
        expected_status
    );
    assert_eq!(
        state.rewarded_quest_ids_like_cpp().contains(&quest_id),
        expected_rewarded
    );
}

pub(in crate::session::tests) fn seasonal_quest_v2_store_like_cpp(
    entries: impl IntoIterator<Item = (u32, u16)>,
) -> QuestV2Store {
    QuestV2Store::from_entries(entries.into_iter().map(|(id, unique_bit_flag)| {
        wow_data::progression_rewards::QuestV2Entry {
            id,
            unique_bit_flag,
        }
    }))
}

pub(in crate::session::tests) fn quest_giver_complete_packet_like_cpp(
    source_guid: ObjectGuid,
    quest_id: u32,
    from_script: bool,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.write_uint32(quest_id);
    pkt.write_bit(from_script);
    pkt.flush_bits();
    pkt
}

pub(in crate::session::tests) fn quest_giver_request_reward_packet_like_cpp(
    source_guid: ObjectGuid,
    quest_id: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.write_uint32(quest_id);
    pkt
}

pub(in crate::session::tests) fn quest_giver_choose_reward_packet_like_cpp(
    source_guid: ObjectGuid,
    quest_id: u32,
    choice_item_id: u32,
    loot_item_type: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.write_uint32(quest_id);
    // C++ QuestChoiceItem: 2-bit LootItemType, ItemInstance, int32 Quantity.
    pkt.write_bits(loot_item_type, 2);
    pkt.write_int32(choice_item_id as i32);
    pkt.write_int32(0); // RandomPropertiesSeed
    pkt.write_int32(0); // RandomPropertiesID
    pkt.write_bit(false); // ItemBonus.has_value()
    pkt.flush_bits();
    pkt.write_bits(0, 6); // ItemModList.Values.size()
    pkt.flush_bits();
    pkt.write_int32(if choice_item_id == 0 { 0 } else { 1 });
    pkt
}

// ── RuntimeTickOwner / RuntimeOutput tests (#NEXT.RUNTIME.L3.001) ─────────
