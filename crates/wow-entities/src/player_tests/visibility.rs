//! Visibility scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;
use wow_core::ObjectGuid;

fn player_in_world_for_native_flag_tests() -> Player {
    let mut player = Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 0xF1A6));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.clear_data_changes();
    player
}

#[test]
fn player_lifecycle_login_plan_keeps_trinity_phase_ordering() {
    let plan = PlayerLoginLifecyclePlan::trinity_handle_player_login();

    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::LoadFromDb,
        PlayerLoginLifecycleStep::SendInitialPacketsBeforeAddToMap,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::SendInitialPacketsBeforeAddToMap,
        PlayerLoginLifecycleStep::AddPlayerToMap,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::AddPlayerToMap,
        PlayerLoginLifecycleStep::SendInitialPacketsAfterAddToMap,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::SendInitialPacketsAfterAddToMap,
        PlayerLoginLifecycleStep::BootstrapVisibility,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::AddPlayerToMap,
        PlayerLoginLifecycleStep::SendZoneWorldStates,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::SendMovementCompoundState,
        PlayerLoginLifecycleStep::MarkOnline,
    ));
}
#[test]
fn player_lifecycle_world_insertion_state_marks_visibility_after_add() {
    let plan = PlayerLoginLifecyclePlan::trinity_handle_player_login();
    let add_index = plan
        .position_of(PlayerLoginLifecycleStep::AddPlayerToMap)
        .unwrap();
    let after_index = plan
        .position_of(PlayerLoginLifecycleStep::SendInitialPacketsAfterAddToMap)
        .unwrap();
    let add_only = PlayerWorldInsertionState::from_completed_steps(&plan.steps()[..=add_index]);
    let after_add =
        PlayerWorldInsertionState::from_completed_steps(&plan.steps()[..=after_index + 2]);

    assert!(add_only.added_to_map);
    assert!(!add_only.visibility_bootstrapped);
    assert!(!add_only.worldstates_sent);
    assert!(after_add.added_to_map);
    assert!(after_add.object_accessor_registered);
    assert!(after_add.visibility_bootstrapped);
    assert!(after_add.worldstates_sent);
}

#[test]
fn native_player_flag_noops_retain_exact_masks_without_queue_admission() {
    const LOCAL_FLAGS: u32 = 0x0000_8008;
    const OTHER_PLAYER_FLAG: u32 = 0x0000_0040;

    let mut player = player_in_world_for_native_flag_tests();
    player.set_player_local_flags_like_cpp(LOCAL_FLAGS);
    player.set_player_flag(OTHER_PLAYER_FLAG);
    player.clear_data_changes();

    player.set_player_local_flags_like_cpp(LOCAL_FLAGS);
    player.remove_player_local_flags_like_cpp(0x0000_0040);
    player.remove_player_flags_like_cpp(0x0000_0400);

    for bit in 0..ACTIVE_PLAYER_DATA_BITS {
        assert_eq!(
            player.active_player_data_changes_mask().is_set(bit),
            [38, 69].contains(&bit),
            "LocalFlags no-op retains the native ActivePlayerData mask"
        );
    }
    for bit in 0..PLAYER_DATA_BITS {
        assert_eq!(
            player.player_data_changes_mask().is_set(bit),
            [0, 7].contains(&bit),
            "RemovePlayerFlag no-op retains the native PlayerData mask"
        );
    }
    assert!(!player.unit().world().object().is_object_updated());
    let update = player.values_update(true);
    let active = update.active_player_data.expect("LocalFlags payload");
    assert_eq!(active.mask.is_set(69), true);
    assert_eq!(active.local_flags, LOCAL_FLAGS);
    assert!(update.player_data.unwrap().values.player_flags & OTHER_PLAYER_FLAG != 0);

    player.clear_data_changes();
    player.set_player_local_flags_like_cpp(0x0000_0100);
    assert!(player.unit().world().object().is_object_updated());
    let changed = player
        .values_update(true)
        .active_player_data
        .expect("changed LocalFlags payload");
    assert!(changed.mask.is_set(69));
    assert_eq!(changed.local_flags, LOCAL_FLAGS | 0x0000_0100);
}

#[test]
fn native_player_flag_changes_queue_and_noops_preserve_prior_queue_and_bits() {
    const LOCAL_FLAGS: u32 = 0x0000_8808;

    let mut player = player_in_world_for_native_flag_tests();
    player.set_player_local_flags_like_cpp(LOCAL_FLAGS);
    player.set_player_flag(0x40);
    player.clear_data_changes();

    player.remove_player_local_flags_like_cpp(0x0000_0800);
    player.remove_player_flags_like_cpp(0x40);

    assert!(player.unit().world().object().is_object_updated());
    assert!(player.active_player_data_changes_mask().is_set(69));
    assert!(player.player_data_changes_mask().is_set(7));
    let changed = player.values_update(true);
    assert_eq!(
        changed.active_player_data.unwrap().local_flags,
        LOCAL_FLAGS & !0x0000_0800
    );
    assert_eq!(changed.player_data.unwrap().values.player_flags, 0);

    let mut queued = player_in_world_for_native_flag_tests();
    queued.set_inebriation_like_cpp(1);
    assert!(queued.unit().world().object().is_object_updated());
    queued.set_player_local_flags_like_cpp(0);
    queued.remove_player_flags_like_cpp(0x0000_0400);
    assert!(queued.unit().world().object().is_object_updated());
    assert!(
        queued
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_INEBRIATION_BIT)
    );
    assert!(queued.active_player_data_changes_mask().is_set(69));
    assert!(queued.player_data_changes_mask().is_set(7));
}
#[test]
fn apply_enchantment_plan_updates_duration_and_visible_shape_like_cpp() {
    let mut player = Player::new(None, false);
    player.unit_mut().set_level(80);
    let mut item = item_with_guid_entry(1249, 7463);
    item.set_slot(EQUIPMENT_SLOT_MAINHAND);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);

    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementTemporary,
            Some(ApplyEnchantmentTemplateRef::new(903)),
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Applied {
                item_guid: item.object().guid(),
                slot: EnchantmentSlot::EnhancementTemporary,
                enchantment_id: 903,
                apply: true,
                effects_allowed: true,
                update_permanent_visible_item: false,
                duration_action: Some(ApplyEnchantmentDurationAction::Added(
                    PlayerEnchantTimeUpdate {
                        item_guid: item.object().guid(),
                        slot: EnchantmentSlot::EnhancementTemporary,
                        duration_secs: 6,
                    },
                )),
            },
        }
    );

    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementTemporary,
            Some(ApplyEnchantmentTemplateRef::new(903)),
            ApplyEnchantmentArgs::remove(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Applied {
                item_guid: item.object().guid(),
                slot: EnchantmentSlot::EnhancementTemporary,
                enchantment_id: 903,
                apply: false,
                effects_allowed: true,
                update_permanent_visible_item: false,
                duration_action: Some(ApplyEnchantmentDurationAction::Removed {
                    item_guid: item.object().guid(),
                    slot: EnchantmentSlot::EnhancementTemporary,
                }),
            },
        }
    );
    assert!(player.enchant_durations().is_empty());

    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 904, 0, 0);
    item.set_max_durability(100);
    item.set_durability(0);
    assert_eq!(
        player
            .apply_enchantment_plan(
                Some(&mut item),
                EnchantmentSlot::EnhancementPermanent,
                Some(ApplyEnchantmentTemplateRef::new(904)),
                ApplyEnchantmentArgs::apply(),
            )
            .result,
        ApplyEnchantmentResult::Applied {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementPermanent,
            enchantment_id: 904,
            apply: true,
            effects_allowed: false,
            update_permanent_visible_item: true,
            duration_action: None,
        }
    );
}
