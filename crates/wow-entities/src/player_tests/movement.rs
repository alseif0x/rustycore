//! Movement scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn enchantment_durations_match_cpp_add_replace_remove_and_tick() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1220, 7200);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 500, 5_000, 0);

    assert_eq!(
        player.add_enchantment_durations(&mut item),
        vec![PlayerEnchantTimeUpdate {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
            duration_secs: 5,
        }]
    );
    assert_eq!(
        player.enchant_durations(),
        &[PlayerEnchantDuration {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
            left_duration_ms: 5_000,
        }]
    );

    assert_eq!(
        player.add_enchantment_duration(&mut item, EnchantmentSlot::EnhancementTemporary, 8_000,),
        Some(PlayerEnchantTimeUpdate {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
            duration_secs: 8,
        })
    );
    assert_eq!(item.data().enchantments[1].duration, 5_000);
    assert_eq!(player.enchant_durations()[0].left_duration_ms, 8_000);

    assert!(
        player
            .update_enchant_time(
                &[PlayerEnchantDurationItemRef::new(
                    item.object().guid(),
                    EnchantmentSlot::EnhancementTemporary,
                    500,
                )],
                3_000,
            )
            .is_empty()
    );
    assert_eq!(player.enchant_durations()[0].left_duration_ms, 5_000);
    assert_eq!(
        player.update_enchant_time(
            &[PlayerEnchantDurationItemRef::new(
                item.object().guid(),
                EnchantmentSlot::EnhancementTemporary,
                500,
            )],
            5_000,
        ),
        vec![UpdateEnchantTimeAction::ClearExpired {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
        }]
    );
    assert!(player.enchant_durations().is_empty());
}
#[test]
fn enchantment_duration_remove_saves_left_duration_unlike_reference_cleanup() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1230, 7300);
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 600, 9_000, 0);
    player.add_enchantment_duration(&mut item, EnchantmentSlot::EnhancementPermanent, 9_000);
    player.update_enchant_time(
        &[PlayerEnchantDurationItemRef::new(
            item.object().guid(),
            EnchantmentSlot::EnhancementPermanent,
            600,
        )],
        4_000,
    );

    let removed = player.remove_enchantment_durations(&mut item);
    assert_eq!(
        removed,
        vec![PlayerEnchantDuration {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementPermanent,
            left_duration_ms: 5_000,
        }]
    );
    assert_eq!(item.data().enchantments[0].duration, 5_000);
    assert!(player.enchant_durations().is_empty());

    item.set_enchantment_duration(EnchantmentSlot::EnhancementPermanent, 9_000);
    player.add_enchantment_duration(&mut item, EnchantmentSlot::EnhancementPermanent, 7_000);
    let removed_refs = player.remove_enchantment_duration_references(&item);
    assert_eq!(removed_refs[0].left_duration_ms, 7_000);
    assert_eq!(item.data().enchantments[0].duration, 9_000);
    assert!(player.enchant_durations().is_empty());
}
#[test]
fn remove_arena_enchantments_cleans_duration_list_like_cpp() {
    let mut player = Player::new(None, false);
    let mut allowed = item_with_guid_entry(1250, 7500);
    let mut blocked = item_with_guid_entry(1251, 7501);
    let mut zero = item_with_guid_entry(1252, 7502);

    player.add_enchantment_duration(&mut allowed, EnchantmentSlot::EnhancementTemporary, 10_000);
    player.add_enchantment_duration(&mut blocked, EnchantmentSlot::EnhancementTemporary, 11_000);
    player.add_enchantment_duration(&mut zero, EnchantmentSlot::EnhancementTemporary, 12_000);
    player.add_enchantment_duration(&mut blocked, EnchantmentSlot::EnhancementPermanent, 1_000);

    let actions = player.remove_arena_enchantments(
        EnchantmentSlot::EnhancementTemporary,
        &[
            ArenaEnchantmentItemRef::new(
                allowed.object().guid(),
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                10,
                true,
            ),
            ArenaEnchantmentItemRef::new(
                blocked.object().guid(),
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_OFFHAND,
                20,
                false,
            ),
            ArenaEnchantmentItemRef::new(
                zero.object().guid(),
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_BACK,
                0,
                false,
            ),
        ],
    );

    assert_eq!(
        actions,
        vec![
            RemoveArenaEnchantmentAction::ClearEquippedEnchantment {
                item_guid: blocked.object().guid(),
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
            RemoveArenaEnchantmentAction::RemoveDurationReference {
                item_guid: zero.object().guid(),
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
        ]
    );
    assert_eq!(
        player.enchant_durations(),
        &[
            PlayerEnchantDuration {
                item_guid: allowed.object().guid(),
                slot: EnchantmentSlot::EnhancementTemporary,
                left_duration_ms: 10_000,
            },
            PlayerEnchantDuration {
                item_guid: blocked.object().guid(),
                slot: EnchantmentSlot::EnhancementPermanent,
                left_duration_ms: 1_000,
            },
        ]
    );
}
