//! Tests for the quest module.
//!
//! Separated from quest.rs under #687.

use super::*;
use wow_core::guid::HighGuid;

#[test]
fn quest_giver_status_multiple_writes_status_as_uint64_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1234, 5678);
    let status = 0x1_0000_0020_u64;
    let bytes = QuestGiverStatusMultiple {
        statuses: vec![(guid, status)],
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QuestGiverStatusMultiple as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_int32().unwrap(), 1);
    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_uint64().unwrap(), status);
}

#[test]
fn quest_giver_quest_details_sallina_capture_length_matches_cpp_like_cpp() {
    // The C++ runtime normalizes realm 0 to the active realm before this packet is captured.
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 530, 0, 15513, 27);
    let bytes = QuestGiverQuestDetails {
        giver_guid: guid,
        giver_creature_id: 15513,
        quest_id: 10070,
        quest_flags: [0x0008_0088, 0, 0],
        suggested_party_members: 0,
        objectives: Vec::new(),
        rewards: QuestRewardsBlock::default(),
        title: "Well Watcher Solanian".to_string(),
        description: "And now, I need for you to do something.$B$BWell Watcher Solanian is in need of your services.  You would do well to ingratiate yourself with him.$B$BHe awaits you on the exterior platform that the ramp in this chamber leads up to.".to_string(),
        log_description: "Speak with Well Watcher Solanian at the Sunspire on Sunstrider Isle.".to_string(),
        auto_launched: false,
    }
    .to_bytes();

    // C++ capture `/tmp/cpp-sallina-packets/0564...QuestDetails.bin` is 769
    // bytes of payload; Rust `to_bytes` includes the 2-byte opcode.
    assert_eq!(bytes.len(), 771);

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_int32().unwrap(), 10070);
    for _ in 0..10 {
        let _ = pkt.read_int32().unwrap();
    }
    assert_eq!(pkt.read_int32().unwrap(), QUEST_EMOTE_COUNT as i32);
    assert_eq!(pkt.read_int32().unwrap(), 0); // Objectives count.
    assert_eq!(pkt.read_int32().unwrap(), 0); // QuestStartItemID.
    // C++ `QuestGiverQuestDetails::Write` has no QuestInfoID field here.
    assert_eq!(pkt.read_int32().unwrap(), 0); // QuestSessionBonus.
    assert_eq!(pkt.read_int32().unwrap(), 15513); // QuestGiverCreatureID.
}

#[test]
fn query_quest_info_response_tail_matches_configured_cpp_layout_like_cpp() {
    let bytes = QueryQuestInfoResponse {
        quest_id: 0x0102_0304,
        allow: true,
        log_title: "Q".to_string(),
        ..QueryQuestInfoResponse::default()
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x0102_0304);
    assert!(pkt.read_bit().unwrap());

    let bytes_before_objectives_size = 17 * 4 // QuestID through RewardBonusMoney.
        + QUEST_REWARD_DISPLAY_SPELL_COUNT * 4
        + 10 * 4 // RewardSpell through FlagsEx2.
        + QUEST_REWARD_ITEM_COUNT * 4 * 4
        + QUEST_REWARD_CHOICES_COUNT * 3 * 4
        + 4 * 4 // POIContinent through POIPriority.
        + 4 * 4 // RewardTitle through RewardNumSkillUps.
        + 4 * 4 // PortraitGiver through PortraitTurnIn.
        + QUEST_REWARD_REPUTATIONS_COUNT * 4 * 4
        + 4 // RewardFactionFlags.
        + QUEST_REWARD_CURRENCY_COUNT * 2 * 4
        + 2 * 4 // AcceptedSoundKitID, CompleteSoundKitID.
        + 4 // AreaGroupID.
        + 8; // TimeAllowed.
    pkt.skip(bytes_before_objectives_size).unwrap();

    assert_eq!(pkt.read_uint32().unwrap(), 0); // Objectives.size()
    assert_eq!(pkt.read_uint64().unwrap(), 0); // AllowableRaces
    assert_eq!(pkt.read_int32().unwrap(), 0); // TreasurePickerID.
    // Local C++ has no TreasurePickerID2 here.
    assert_eq!(pkt.read_int32().unwrap(), 0); // Expansion.
    assert_eq!(pkt.read_int32().unwrap(), 0); // ManagedWorldStateID.
    assert_eq!(pkt.read_int32().unwrap(), 0); // QuestSessionBonus.
    assert_eq!(pkt.read_int32().unwrap(), 0); // QuestGiverCreatureID.
    // String bit lengths follow immediately; no conditional-text counts precede them.
    assert_eq!(pkt.read_bits(9).unwrap(), 1); // LogTitle.size()
    assert_eq!(pkt.read_bits(12).unwrap(), 0); // LogDescription.size()
    assert_eq!(pkt.read_bits(12).unwrap(), 0); // QuestDescription.size()
    assert_eq!(pkt.read_bits(9).unwrap(), 0); // AreaDescription.size()
    assert_eq!(pkt.read_bits(10).unwrap(), 0); // PortraitGiverText.size()
    assert_eq!(pkt.read_bits(8).unwrap(), 0); // PortraitGiverName.size()
    assert_eq!(pkt.read_bits(10).unwrap(), 0); // PortraitTurnInText.size()
    assert_eq!(pkt.read_bits(8).unwrap(), 0); // PortraitTurnInName.size()
    assert_eq!(pkt.read_bits(11).unwrap(), 0); // QuestCompletionLog.size()
    assert!(!pkt.read_bit().unwrap()); // ReadyForTranslation.
}

#[test]
fn quest_giver_status_constants_match_cpp_quest_def_like_cpp() {
    assert_eq!(quest_giver_status::NONE, 0x0000_0000);
    assert_eq!(quest_giver_status::FUTURE, 0x0000_0002);
    assert_eq!(quest_giver_status::TRIVIAL, 0x0000_0004);
    assert_eq!(quest_giver_status::TRIVIAL_REPEATABLE_TURNIN, 0x0000_0008);
    assert_eq!(quest_giver_status::TRIVIAL_DAILY_QUEST, 0x0000_0010);
    assert_eq!(quest_giver_status::REWARD, 0x0000_0020);
    assert_eq!(quest_giver_status::JOURNEY_REWARD, 0x0000_0040);
    assert_eq!(quest_giver_status::COVENANT_CALLING_REWARD, 0x0000_0080);
    assert_eq!(quest_giver_status::REPEATABLE_TURNIN, 0x0000_0100);
    assert_eq!(quest_giver_status::DAILY_QUEST, 0x0000_0200);
    assert_eq!(quest_giver_status::QUEST, 0x0000_0400);
    assert_eq!(quest_giver_status::REWARD_COMPLETE_NO_POI, 0x0000_0800);
    assert_eq!(quest_giver_status::REWARD_COMPLETE_POI, 0x0000_1000);
    assert_eq!(quest_giver_status::LEGENDARY_QUEST, 0x0000_2000);
    assert_eq!(
        quest_giver_status::LEGENDARY_REWARD_COMPLETE_NO_POI,
        0x0000_4000
    );
    assert_eq!(
        quest_giver_status::LEGENDARY_REWARD_COMPLETE_POI,
        0x0000_8000
    );
    assert_eq!(quest_giver_status::JOURNEY_QUEST, 0x0001_0000);
    assert_eq!(
        quest_giver_status::JOURNEY_REWARD_COMPLETE_NO_POI,
        0x0002_0000
    );
    assert_eq!(quest_giver_status::JOURNEY_REWARD_COMPLETE_POI, 0x0004_0000);
    assert_eq!(quest_giver_status::COVENANT_CALLING_QUEST, 0x0008_0000);
    assert_eq!(
        quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_NO_POI,
        0x0010_0000
    );
    assert_eq!(
        quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_POI,
        0x0020_0000
    );
    assert_eq!(quest_giver_status::TRIVIAL_LEGENDARY_QUEST, 0x0040_0000);
    assert_eq!(quest_giver_status::FUTURE_LEGENDARY_QUEST, 0x0080_0000);
    assert_eq!(quest_giver_status::LEGENDARY_REWARD, 0x0100_0000);
    assert_eq!(quest_giver_status::IMPORTANT_QUEST, 0x0200_0000);
    assert_eq!(quest_giver_status::IMPORTANT_REWARD, 0x0400_0000);
    assert_eq!(quest_giver_status::TRIVIAL_IMPORTANT_QUEST, 0x0800_0000);
    assert_eq!(quest_giver_status::FUTURE_IMPORTANT_QUEST, 0x1000_0000);
    assert_eq!(
        quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_NO_POI,
        0x2000_0000
    );
    assert_eq!(
        quest_giver_status::IMPORTANT_QUEST_REWARD_COMPLETE_POI,
        0x4000_0000
    );
    assert_eq!(quest_giver_status::TRIVIAL_JOURNEY_QUEST, 0x8000_0000);
    assert_eq!(quest_giver_status::FUTURE_JOURNEY_QUEST, 0x1_0000_0000);
}

#[test]
fn world_quest_update_response_empty_writes_zero_count_like_cpp() {
    let bytes = WorldQuestUpdateResponse {
        updates: Vec::new(),
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::WorldQuestUpdateResponse as u16
    );
    assert_eq!(&bytes[2..], &[0, 0, 0, 0]);
}

#[test]
fn quest_giver_quest_failed_writes_quest_id_then_reason_like_cpp() {
    let bytes = QuestGiverQuestFailed {
        quest_id: 7008,
        reason: 50,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QuestGiverQuestFailed as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 7008);
    assert_eq!(pkt.read_uint32().unwrap(), 50);
    assert!(pkt.is_empty());
}

#[test]
fn quest_update_add_pvp_credit_writes_cpp_shape() {
    let bytes = QuestUpdateAddPvpCredit {
        quest_id: 12_345,
        count: 7,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QuestUpdateAddPvpCredit as u16
    );
    assert_eq!(&bytes[2..6], &12_345_i32.to_le_bytes());
    assert_eq!(&bytes[6..8], &7_u16.to_le_bytes());
}

#[test]
fn quest_update_add_credit_simple_writes_cpp_shape() {
    let bytes = QuestUpdateAddCreditSimple {
        quest_id: 12_345,
        object_id: -7,
        objective_type: 14,
    }
    .to_bytes();

    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QuestUpdateAddCreditSimple as u16
    );
    assert_eq!(&bytes[2..6], &12_345_i32.to_le_bytes());
    assert_eq!(&bytes[6..10], &(-7_i32).to_le_bytes());
    assert_eq!(bytes[10], 14);
}
