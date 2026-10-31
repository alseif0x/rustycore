//! Ordered self-res spell dynamic-field behavior on the canonical Player.

use super::*;

#[test]
fn local_flags_keep_one_canonical_value_and_mark_native_section_on_noops() {
    let mut player = Player::new(Some(1), false);
    player.gameplay_state_mut().active_local_flags = 0x8000;
    for enable in [true, true, false, false] {
        player.clear_active_player_data_changes();
        if enable {
            player.set_player_local_flags_like_cpp(8);
        } else {
            player.remove_player_local_flags_like_cpp(8);
        }
        let flags = if enable { 0x8008 } else { 0x8000 };
        assert_eq!(player.gameplay_state().active_local_flags, flags);
        let update = player.values_update(true).active_player_data.unwrap();
        assert_eq!(update.local_flags, flags);
        assert!(update.mask.is_set(38) && update.mask.is_set(69));
        assert!(player.values_update(false).active_player_data.is_none());
    }
}

#[test]
fn self_res_spells_append_in_order_and_remove_only_the_first_duplicate() {
    let mut player = Player::new(Some(1), false);
    player.add_self_res_spell(3026);
    player.add_self_res_spell(21169);
    player.add_self_res_spell(3026);

    assert_eq!(player.self_res_spells(), &[3026, 21169, 3026]);
    assert!(player.remove_self_res_spell(3026));
    assert_eq!(player.self_res_spells(), &[21169, 3026]);
    assert!(player.remove_self_res_spell(3026));
    assert_eq!(player.self_res_spells(), &[21169]);
}

#[test]
fn self_res_spell_remove_marks_shifted_indices_across_mask_blocks() {
    let mut player = Player::new(Some(1), false);
    for spell_id in 0..34 {
        player.add_self_res_spell(spell_id);
    }
    player.clear_active_player_data_changes();
    player.add_self_res_spell(i32::MIN);

    assert!(player.remove_self_res_spell(30));
    assert_eq!(player.self_res_spells().len(), 34);
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        Some(vec![0xC000_0000, 0x0000_0003])
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT)
    );

    player.clear_active_player_data_changes();
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        Some(vec![0, 0])
    );
    player.add_self_res_spell(i32::MIN);
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        Some(vec![0, 0x0000_0004])
    );
    assert_eq!(player.self_res_spells().last(), Some(&i32::MIN));
}

#[test]
fn self_res_spell_missing_remove_is_clean_and_empty_clear_is_dirty() {
    let mut player = Player::new(Some(1), false);
    player.clear_active_player_data_changes();
    assert!(!player.remove_self_res_spell(0));
    assert!(!player.active_player_data_changes_mask().is_any_set());
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        Some(vec![])
    );

    player.clear_self_res_spells();
    assert!(player.self_res_spells().is_empty());
    assert!(player.active_player_data_changes_mask().is_set(0));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT)
    );
    assert_eq!(
        player.active_data().self_res_spells_update_mask,
        Some(vec![])
    );

    player.clear_active_player_data_changes();
    player.clear_self_res_spells();
    assert!(player.active_player_data_changes_mask().is_set(0));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT)
    );
}
