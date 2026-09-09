//! Loot scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn player_flags_and_loot_guid_mark_playerdata_bits() {
    let mut player = Player::new(None, false);

    player.set_player_flag(0x20);
    player.set_player_flag_ex(0x04);
    player.set_loot_guid(ObjectGuid::new(9, 3));
    player.set_bank_bag_slot_count(6);
    player.set_primary_specialization(62);
    player.set_honor_level_like_cpp(3);

    assert!(player.has_player_flag(0x20));
    assert!(player.has_player_flag_ex(0x04));
    assert_eq!(player.data().loot_target_guid, ObjectGuid::new(9, 3));
    assert_eq!(player.data().num_bank_slots, 6);
    assert_eq!(player.data().current_spec_id, 62);
    assert_eq!(player.data().honor_level, 3);
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_FLAGS_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_FLAGS_EX_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_LOOT_TARGET_GUID_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_NUM_BANK_SLOTS_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_CURRENT_SPEC_ID_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_HONOR_LEVEL_BIT)
    );

    player.remove_player_flag(0x20);
    player.remove_player_flag_ex(0x04);
    assert!(!player.has_player_flag(0x20));
    assert!(!player.has_player_flag_ex(0x04));
}
#[test]
fn money_matches_cpp_modify_clamps_and_active_playerdata_coinage_bit() {
    let mut player = Player::new(None, false);

    player.set_money(100);
    assert_eq!(player.active_data().coinage, 100);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_COINAGE_BIT)
    );

    assert!(player.modify_money(-150));
    assert_eq!(player.active_data().coinage, 0);

    player.set_money(MAX_MONEY_AMOUNT - 1);
    assert!(!player.modify_money(2));
    assert_eq!(player.active_data().coinage, MAX_MONEY_AMOUNT - 1);
    assert!(!player.modify_money(i64::MAX));
    assert_eq!(player.active_data().coinage, MAX_MONEY_AMOUNT - 1);

    assert!(player.modify_money(1));
    assert_eq!(player.active_data().coinage, MAX_MONEY_AMOUNT);
}
