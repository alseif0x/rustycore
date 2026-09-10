//! Quest giver quest complete tests for the quest module.
//!
//! Separated from quest.rs under #687.

use super::*;

#[test]
fn quest_giver_quest_complete_writes_skill_line_then_skill_ups_like_cpp() {
    let complete = QuestGiverQuestComplete {
        quest_id: 0x0102_0304,
        xp: 0x0506_0708,
        money: 0x0102_0304,
        skill_line_id: 0x0A0B_0C0D,
        skill_points: 0x0E0F_1011,
        use_quest_reward_currency: true,
    };

    let bytes = complete.to_bytes();

    assert_eq!(
        QuestGiverQuestComplete::OPCODE,
        ServerOpcodes::QuestGiverQuestComplete
    );
    assert_eq!(&bytes[2..6], &0x0102_0304u32.to_le_bytes());
    assert_eq!(&bytes[6..10], &0x0506_0708u32.to_le_bytes());
    assert_eq!(&bytes[10..18], &0x0102_0304u64.to_le_bytes());
    assert_eq!(&bytes[18..22], &0x0A0B_0C0Du32.to_le_bytes());
    assert_eq!(&bytes[22..26], &0x0E0F_1011u32.to_le_bytes());
    assert_eq!(bytes[26], 0x80); // UseQuestReward=true, remaining C++ bits false.
    assert_eq!(&bytes[27..31], &0i32.to_le_bytes()); // Default ItemReward ItemID.
    assert_eq!(&bytes[31..35], &0i32.to_le_bytes()); // RandomPropertiesSeed.
    assert_eq!(&bytes[35..39], &0i32.to_le_bytes()); // RandomPropertiesID.
    assert_eq!(bytes[39], 0x00); // Empty ItemReward bonus bit.
    assert_eq!(bytes[40], 0x00); // Empty ItemReward modifications.
    assert_eq!(bytes.len(), 41);
}

#[test]
fn quest_giver_quest_complete_keeps_empty_item_reward_when_flags_are_false() {
    let complete = QuestGiverQuestComplete {
        quest_id: 7,
        xp: 0,
        money: 0,
        skill_line_id: 0,
        skill_points: 0,
        use_quest_reward_currency: false,
    };

    let bytes = complete.to_bytes();

    assert_eq!(bytes[26], 0x00);
    assert_eq!(&bytes[27..39], &[0; 12]);
    assert_eq!(bytes[39], 0x00);
    assert_eq!(bytes[40], 0x00);
    assert_eq!(bytes.len(), 41);
}

#[test]
fn quest_rewards_choice_item_flushes_loot_type_before_item_instance_like_cpp() {
    let mut rewards = QuestRewardsBlock::default();
    rewards.choice_items[0] = (0x0102_0304, 7);
    rewards.choice_item_types[0] = 0b10;

    let mut pkt = WorldPacket::new_empty();
    rewards.write(&mut pkt);
    let bytes = pkt.into_data();

    // Counts + fixed reward slots + money/xp/artifact/honor/title + faction/spell/currency/skill fields.
    let choice_start = 4
        + 4
        + (QUEST_REWARD_ITEM_COUNT * 2 * 4)
        + 4
        + 4
        + 8
        + 4
        + 4
        + 4
        + 4
        + (QUEST_REWARD_REPUTATIONS_COUNT * 4 * 4)
        + (QUEST_REWARD_DISPLAY_SPELL_COUNT * 4)
        + 4
        + (QUEST_REWARD_CURRENCY_COUNT * 2 * 4)
        + 4
        + 4
        + 4;
    assert_eq!(
        bytes[choice_start] >> 6,
        0b10,
        "C++ ByteBuffer::append flushes the 2-bit LootItemType before ItemInstance"
    );
    assert_eq!(
        &bytes[choice_start + 1..choice_start + 5],
        &0x0102_0304_i32.to_le_bytes()
    );
    assert_eq!(
        &bytes[choice_start + 15..choice_start + 19],
        &7_i32.to_le_bytes()
    );
}
