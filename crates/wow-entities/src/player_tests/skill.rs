//! Skill scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn scaling_player_level_delta_marks_cpp_parent_and_field_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.set_scaling_player_level_delta_like_cpp(-1);

    assert_eq!(player.active_data().scaling_player_level_delta, -1);
    let mask = player.active_player_data_changes_mask();
    assert_eq!(mask.get_block(0), 1 << ACTIVE_PLAYER_DATA_PARENT_BIT);
    assert_eq!(mask.get_block(1), 0);
    assert_eq!(
        mask.get_block(2),
        (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT - 64))
            | (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT - 64))
    );
    assert!(mask.blocks()[3..].iter().all(|block| *block == 0));

    player.clear_data_changes();
    player.set_scaling_player_level_delta_like_cpp(-1);
    assert!(!player.active_player_data_changes_mask().is_any_set());
    player.mark_scaling_player_level_delta_changed_like_cpp();
    let mask = player.active_player_data_changes_mask();
    assert_eq!(mask.get_block(0), 1 << ACTIVE_PLAYER_DATA_PARENT_BIT);
    assert_eq!(
        mask.get_block(2),
        (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT - 64))
            | (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT - 64))
    );
}
#[test]
fn canonical_player_retains_the_shared_level_xp_catalog_like_cpp() {
    let mut player = Player::new(None, false);
    let table = std::sync::Arc::new(vec![0, 400, 900]);

    player.install_player_xp_table_like_cpp(std::sync::Arc::clone(&table));

    assert_eq!(player.player_xp_for_level_like_cpp(1), Some(400));
    assert_eq!(player.player_xp_for_level_like_cpp(2), Some(900));
    assert_eq!(player.player_xp_for_level_like_cpp(3), None);
    assert_eq!(std::sync::Arc::strong_count(&table), 2);
}
#[test]
fn add_honor_xp_matches_cpp_level_gate_threshold_and_max_shape() {
    let mut low_level = Player::new(None, false);
    assert!(!low_level.add_honor_xp_like_cpp(100, PLAYER_LEVEL_MIN_HONOR_LIKE_CPP - 1));
    assert_eq!(low_level.active_data().honor, 0);
    assert!(!low_level.active_player_data_changes_mask().is_any_set());

    let mut player = Player::new(None, false);
    assert!(player.add_honor_xp_like_cpp(PLAYER_HONOR_NEXT_LEVEL_XP_LIKE_CPP as u32 + 25, 10));
    assert_eq!(player.data().honor_level, 1);
    assert_eq!(player.active_data().honor, 25);
    assert_eq!(
        player.active_data().honor_next_level,
        PLAYER_HONOR_NEXT_LEVEL_XP_LIKE_CPP
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_HONOR_LEVEL_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT)
    );

    player.clear_data_changes();
    player.set_honor_level_like_cpp(PLAYER_MAX_HONOR_LEVEL_LIKE_CPP - 1);
    player.set_honor_like_cpp(PLAYER_HONOR_NEXT_LEVEL_XP_LIKE_CPP - 1);
    player.clear_data_changes();

    assert!(player.add_honor_xp_like_cpp(1, 80));
    assert_eq!(player.data().honor_level, PLAYER_MAX_HONOR_LEVEL_LIKE_CPP);
    assert_eq!(player.active_data().honor, 0);
}
#[test]
fn add_explored_zones_ors_mask_and_marks_cpp_parent_and_child_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert!(player.add_explored_zones_like_cpp(7, 0x0f));
    assert_eq!(player.explored_zones_block_like_cpp(7), Some(0x0f));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT + 7)
    );

    player.clear_data_changes();
    assert!(player.add_explored_zones_like_cpp(7, 0xf0));
    assert_eq!(player.explored_zones_block_like_cpp(7), Some(0xff));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT + 7)
    );
}
#[test]
fn add_explored_zones_repeated_and_out_of_range_are_noops_like_cpp() {
    let mut player = Player::new(None, false);
    assert!(player.add_explored_zones_like_cpp(0, u64::MAX));
    player.clear_data_changes();

    assert!(!player.add_explored_zones_like_cpp(0, 1));
    assert_eq!(player.explored_zones_block_like_cpp(0), Some(u64::MAX));
    assert!(!player.active_player_data_changes_mask().is_any_set());

    assert!(!player.add_explored_zones_like_cpp(PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP, u64::MAX));
    assert!(!player.active_player_data_changes_mask().is_any_set());
}
#[test]
fn explored_zones_db_string_parser_matches_cpp_low_high_words() {
    let blocks = parse_explored_zones_db_string_like_cpp("1 2 bad 4 5");

    assert_eq!(blocks[0], 0x0000_0002_0000_0001);
    assert_eq!(blocks[1], 0x0000_0004_0000_0000);
    assert_eq!(blocks[2], 5);
    assert!(blocks[3..].iter().all(|value| *value == 0));
}
#[test]
fn explored_zones_db_string_parser_ignores_tokens_past_cpp_array() {
    let input = std::iter::repeat_n("1", PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP * 2 + 6)
        .collect::<Vec<_>>()
        .join(" ");
    let blocks = parse_explored_zones_db_string_like_cpp(&input);

    assert!(blocks.iter().all(|value| *value == 0x0000_0001_0000_0001));
}
#[test]
fn explored_zones_db_string_serializer_matches_cpp_low_high_order_and_trailing_space() {
    let mut blocks = [0u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP];
    blocks[0] = 0x0000_0002_0000_0001;
    blocks[1] = 0xFFFF_FFFF_8000_0000;

    let serialized = explored_zones_db_string_from_blocks_like_cpp(&blocks);

    assert!(serialized.starts_with("1 2 2147483648 4294967295 0 0 "));
    assert!(serialized.ends_with(' '));
    assert_eq!(
        serialized.split_whitespace().count(),
        PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP * 2
    );
}
#[test]
fn explicit_markers_force_default_value_deltas_like_cpp_live_object_masks() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.mark_inv_slot_changed(0);
    player.mark_visible_item_slot_changed(0);
    player.mark_buyback_price_changed(0);
    player.mark_buyback_timestamp_changed(0);

    assert_eq!(player.active_data().inv_slots[0], ObjectGuid::EMPTY);
    assert_eq!(player.data().visible_items[0], VisibleItemValues::default());
    assert_eq!(player.active_data().buyback_price[0], 0);
    assert_eq!(player.active_data().buyback_timestamp[0], 0);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT)
    );
}
#[test]
fn apply_enchantment_effect_actions_expand_cpp_stat_switch_special_cases() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12494, 7468);
    item.set_slot(EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    11,
                    ItemModType::HitRating as u32,
                ),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    12,
                    ItemModType::CritRating as u32,
                ),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    13,
                    ItemModType::HasteRating as u32,
                ),
            ],
        ),
        vec![
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitMelee,
                amount: 11,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitRanged,
                amount: 11,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitSpell,
                amount: 11,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::CritMelee,
                amount: 12,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::CritRanged,
                amount: 12,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::CritSpell,
                amount: 12,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteMelee,
                amount: 13,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteRanged,
                amount: 13,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteSpell,
                amount: 13,
                apply: true,
            },
        ]
    );

    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            false,
            &[
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    20,
                    ItemModType::AttackPower as u32,
                ),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    21,
                    ItemModType::SpellPower as u32,
                ),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    22,
                    ItemModType::BlockValue as u32,
                ),
            ],
        ),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::AttackPower,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 20,
                apply: false,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::AttackPowerRanged,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 20,
                apply: false,
            },
            ApplyEnchantmentEffectAction::SpellPowerBonus {
                amount: 21,
                apply: false,
            },
            ApplyEnchantmentEffectAction::BaseModFlatValue {
                base_mod: ApplyEnchantmentBaseMod::ShieldBlockValue,
                amount: 22,
                apply: false,
            },
        ]
    );
}
