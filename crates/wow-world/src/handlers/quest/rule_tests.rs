use super::*;
use wow_persistence::{QuestPoiBlobLoadRowLikeCpp, QuestPoiPointLoadRowLikeCpp};

#[test]
fn quest_giver_query_quest_reads_respond_to_giver_as_bit_like_cpp() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 123, 456);

    for (bit_byte, expected) in [(0x80, true), (0x00, false), (0x01, false)] {
        let mut packet = quest_giver_cmsg_packet(guid, 7001, bit_byte);
        let (parsed_guid, quest_id, respond_to_giver) =
            read_quest_giver_query_quest_like_cpp(&mut packet).unwrap();

        assert_eq!(parsed_guid, guid);
        assert_eq!(quest_id, 7001);
        assert_eq!(
            respond_to_giver, expected,
            "C++ ReadBit reads the high bit; byte {bit_byte:#04x} must not be treated as bool"
        );
    }
}

#[test]
fn quest_giver_accept_quest_reads_start_cheat_as_bit_like_cpp() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 123, 456);

    for (bit_byte, expected) in [(0x80, true), (0x00, false), (0x01, false)] {
        let mut packet = quest_giver_cmsg_packet(guid, 7002, bit_byte);
        let (parsed_guid, quest_id, start_cheat) =
            read_quest_giver_accept_quest_like_cpp(&mut packet).unwrap();

        assert_eq!(parsed_guid, guid);
        assert_eq!(quest_id, 7002);
        assert_eq!(
            start_cheat, expected,
            "C++ ReadBit reads the high bit; byte {bit_byte:#04x} must not be treated as bool"
        );
    }
}

#[test]
fn quest_giver_creature_id_is_zero_for_gameobject_sources_like_cpp() {
    assert_eq!(
        quest_giver_creature_id_from_source_like_cpp(creature_guid(15513, 27)),
        15513
    );
    assert_eq!(
        quest_giver_creature_id_from_source_like_cpp(gameobject_guid(180516, 301)),
        0
    );
}

#[test]
fn query_quest_completion_builds_creature_then_masked_go_entries_like_cpp() {
    let mut store = store_with_quests(&[77]);
    store.ender_quests.entry(1234).or_default().push(77);
    store.ender_quests.entry(12).or_default().push(77);
    store
        .gameobject_ender_quests
        .entry(0x5678)
        .or_default()
        .push(77);

    let response = represented_quest_completion_npc_response_like_cpp(&store, &[77]);

    assert_eq!(response.len(), 1);
    assert_eq!(response[0].quest_id, 77);
    assert_eq!(response[0].npcs, vec![12, 1234, 0x8000_5678u32 as i32]);
}

#[test]
fn query_quest_completion_skips_negative_missing_and_oversized_creature_entries_like_cpp() {
    let mut store = store_with_quests(&[5]);
    store
        .ender_quests
        .entry(i32::MAX as u32 + 1)
        .or_default()
        .push(5);
    store
        .gameobject_ender_quests
        .entry(u32::MAX)
        .or_default()
        .push(5);

    let response = represented_quest_completion_npc_response_like_cpp(&store, &[-1, 999, 5]);

    assert_eq!(response.len(), 1);
    assert_eq!(response[0].quest_id, 5);
    assert_eq!(response[0].npcs, vec![-1]);
}

fn quest_giver_cmsg_packet(
    guid: ObjectGuid,
    quest_id: u32,
    bit_byte: u8,
) -> wow_packet::WorldPacket {
    let mut packet = wow_packet::WorldPacket::new_empty();
    packet.write_packed_guid(&guid);
    packet.write_uint32(quest_id);
    packet.write_uint8(bit_byte);
    packet.reset_read();
    packet
}

fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

fn gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

fn store_with_quests(ids: &[u32]) -> wow_data::quest::QuestStore {
    wow_data::quest::QuestStore::from_quests_like_cpp(ids.iter().copied().map(quest_template))
}

fn quest_template(id: u32) -> wow_data::quest::QuestTemplate {
    wow_data::quest::QuestTemplate {
        id,
        quest_type: 2,
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
        log_title: format!("Quest {id}"),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
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
    }
}

fn quest_poi_blob_row_like_cpp(quest_id: i32, idx1: i32) -> QuestPoiBlobLoadRowLikeCpp {
    QuestPoiBlobLoadRowLikeCpp {
        quest_id,
        blob_index: 1,
        idx1,
        objective_index: -1,
        quest_objective_id: 2,
        quest_object_id: 3,
        map_id: 571,
        ui_map_id: 486,
        priority: 4,
        flags: 5,
        world_effect_id: 6,
        player_condition_id: 7,
        navigation_player_condition_id: 8,
        spawn_tracking_id: 9,
        always_allow_merging_blobs: false,
    }
}

#[test]
fn quest_poi_typed_rows_join_points_and_skip_unknown_groups_like_cpp() {
    let store = build_quest_poi_store_like_cpp(
        vec![QuestPoiPointLoadRowLikeCpp {
            quest_id: 77,
            idx1: 3,
            x: 10,
            y: 11,
            z: 12,
        }],
        vec![
            quest_poi_blob_row_like_cpp(77, 3),
            quest_poi_blob_row_like_cpp(88, 9),
        ],
    );

    assert_eq!(store.len(), 1);
    assert_eq!(store[&77].blobs[0].points[0].x, 10);
    assert!(!store.contains_key(&88));
}
