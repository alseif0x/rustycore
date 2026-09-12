//! #769 — the canonical Player owns its item-modifier runtime and its
//! invariants.
//!
//! C++ holds one `ItemSetEffect` (`Entities/Item/Item.h:41`) per active set in
//! `Player::ItemSetEff` and changes it through the two functions it declares on
//! the Player boundary (`Player.h:3160-3161`): `AddItemsSetItem`
//! (`Entities/Item/Item.cpp:57`) and `RemoveItemsSetItem` (`:146`), the second
//! deleting an effect once its last equipped item is gone (`:192`).

use crate::{PlayerItemLevelCapsLikeCpp, PlayerItemModifierRuntimeStateLikeCpp};

fn item(counter: i64) -> wow_core::ObjectGuid {
    wow_core::ObjectGuid::create_item(1, counter)
}

#[test]
fn a_fresh_item_modifier_runtime_is_empty_like_cpp() {
    let runtime = PlayerItemModifierRuntimeStateLikeCpp::default();

    assert!(runtime.item_set_effects_like_cpp().is_empty());
    assert_eq!(runtime.item_set_effect_like_cpp(700), None);
    assert_eq!(runtime.bonuses_like_cpp().shield_block_value, 0);
    assert_eq!(runtime.item_level_caps_like_cpp().max_item_level, 0);
}

#[test]
fn the_first_equipped_item_creates_its_set_effect_like_cpp_add_items_set_item() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();

    assert_eq!(runtime.add_item_set_item_like_cpp(700, item(1)), 1);

    let effect = runtime.item_set_effect_like_cpp(700).expect("set effect");
    assert_eq!(effect.item_set_id, 700);
    assert!(effect.equipped_items.contains(&item(1)));
    assert!(effect.set_bonuses.is_empty());
}

#[test]
fn equipping_the_same_item_twice_does_not_grow_the_count_like_cpp() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();

    assert_eq!(runtime.add_item_set_item_like_cpp(700, item(1)), 1);
    assert_eq!(runtime.add_item_set_item_like_cpp(700, item(1)), 1);
    assert_eq!(runtime.add_item_set_item_like_cpp(700, item(2)), 2);
}

#[test]
fn two_sets_keep_independent_effects_like_cpp() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();

    runtime.add_item_set_item_like_cpp(700, item(1));
    runtime.add_item_set_item_like_cpp(800, item(2));

    assert_eq!(runtime.item_set_effects_like_cpp().len(), 2);
    assert_eq!(
        runtime
            .item_set_effect_like_cpp(700)
            .map(|effect| effect.equipped_items.len()),
        Some(1)
    );
}

#[test]
fn a_set_bonus_is_added_once_and_needs_an_existing_effect_like_cpp() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();

    assert!(!runtime.add_item_set_bonus_like_cpp(700, 35));

    runtime.add_item_set_item_like_cpp(700, item(1));
    assert!(runtime.add_item_set_bonus_like_cpp(700, 35));
    assert!(!runtime.add_item_set_bonus_like_cpp(700, 35));
    assert_eq!(
        runtime
            .item_set_effect_like_cpp(700)
            .map(|effect| effect.set_bonuses.len()),
        Some(1)
    );
}

#[test]
fn removing_an_item_reports_the_remaining_count_like_cpp_remove_items_set_item() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();
    runtime.add_item_set_item_like_cpp(700, item(1));
    runtime.add_item_set_item_like_cpp(700, item(2));

    assert_eq!(runtime.remove_item_set_item_like_cpp(700, item(1)), Some(1));
    assert_eq!(runtime.remove_item_set_item_like_cpp(700, item(1)), Some(1));
    assert_eq!(runtime.remove_item_set_item_like_cpp(700, item(2)), Some(0));
}

#[test]
fn removing_an_item_of_a_set_without_an_effect_is_the_cpp_early_return() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();

    assert_eq!(runtime.remove_item_set_item_like_cpp(700, item(1)), None);
    assert!(runtime.item_set_effects_like_cpp().is_empty());
}

#[test]
fn dropping_a_set_bonus_reports_whether_it_was_held_like_cpp() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();
    runtime.add_item_set_item_like_cpp(700, item(1));
    runtime.add_item_set_bonus_like_cpp(700, 35);

    assert!(runtime.remove_item_set_bonus_like_cpp(700, 35));
    assert!(!runtime.remove_item_set_bonus_like_cpp(700, 35));
    assert!(!runtime.remove_item_set_bonus_like_cpp(800, 35));
}

#[test]
fn an_effect_is_deleted_only_once_its_last_item_is_gone_like_cpp() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();
    runtime.add_item_set_item_like_cpp(700, item(1));
    runtime.add_item_set_item_like_cpp(700, item(2));
    runtime.add_item_set_bonus_like_cpp(700, 35);

    runtime.remove_item_set_item_like_cpp(700, item(1));
    assert!(!runtime.drop_empty_item_set_effect_like_cpp(700));
    assert!(runtime.item_set_effect_like_cpp(700).is_some());

    runtime.remove_item_set_item_like_cpp(700, item(2));
    assert!(runtime.drop_empty_item_set_effect_like_cpp(700));
    assert_eq!(runtime.item_set_effect_like_cpp(700), None);
    assert!(!runtime.drop_empty_item_set_effect_like_cpp(700));
}

#[test]
fn a_deleted_effect_takes_its_remaining_bonuses_with_it_like_cpp() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();
    runtime.add_item_set_item_like_cpp(700, item(1));
    runtime.add_item_set_bonus_like_cpp(700, 35);
    runtime.remove_item_set_item_like_cpp(700, item(1));

    assert!(runtime.drop_empty_item_set_effect_like_cpp(700));

    assert!(runtime.item_set_effects_like_cpp().is_empty());
    assert!(!runtime.add_item_set_bonus_like_cpp(700, 35));
}

#[test]
fn item_level_caps_are_installed_together() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();

    runtime.set_item_level_caps_like_cpp(PlayerItemLevelCapsLikeCpp {
        min_item_level_cutoff: 10,
        min_item_level: 20,
        max_item_level: 200,
    });

    let caps = runtime.item_level_caps_like_cpp();
    assert_eq!(caps.min_item_level_cutoff, 10);
    assert_eq!(caps.min_item_level, 20);
    assert_eq!(caps.max_item_level, 200);
}

#[test]
fn resetting_the_bonuses_leaves_set_effects_and_caps_alone() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();
    runtime.add_item_set_item_like_cpp(700, item(1));
    runtime.set_item_level_caps_like_cpp(PlayerItemLevelCapsLikeCpp {
        min_item_level_cutoff: 0,
        min_item_level: 0,
        max_item_level: 200,
    });
    runtime.with_bonuses_mut_like_cpp(|bonuses| bonuses.shield_block_value = 29);

    runtime.reset_bonuses_like_cpp();

    assert_eq!(runtime.bonuses_like_cpp().shield_block_value, 0);
    assert!(runtime.item_set_effect_like_cpp(700).is_some());
    assert_eq!(runtime.item_level_caps_like_cpp().max_item_level, 200);
}

#[test]
fn the_retained_bonus_borrow_writes_the_owner_and_the_snapshot_copies_it() {
    let mut runtime = PlayerItemModifierRuntimeStateLikeCpp::default();

    let returned = runtime.with_bonuses_mut_like_cpp(|bonuses| {
        bonuses.attack_power_total = 10;
        bonuses.stats_base[1] = 4;
        bonuses.attack_power_total
    });

    assert_eq!(returned, 10);
    assert_eq!(runtime.bonuses_like_cpp().attack_power_total, 10);
    let mut snapshot = runtime.bonuses_snapshot_like_cpp();
    snapshot.attack_power_total = 99;
    assert_eq!(runtime.bonuses_like_cpp().attack_power_total, 10);
    assert_eq!(runtime.bonuses_like_cpp().stats_base[1], 4);
}
