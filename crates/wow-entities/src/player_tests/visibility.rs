//! Visibility scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
