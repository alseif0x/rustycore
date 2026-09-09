//! Combat scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_skill_replacement_keeps_deleted_markers_without_authorizing_malformed_rows() {
    let mut player = Player::new(None, false);
    player.replace_skill_records_like_cpp(vec![], true, true, Some(7), BTreeSet::from([333, 755]));
    let row = |id, state, value| PlayerSkillRecord {
        skill_line_id: id,
        current_value: value,
        max_value: value,
        step: 0,
        profession_slot: -1,
        state,
    };
    player.replace_represented_skill_records_like_cpp(
        vec![
            (333, row(333, PlayerSkillLoadState::Deleted, 1)),
            (755, row(755, PlayerSkillLoadState::Unchanged, 0)),
        ],
        true,
        true,
    );
    assert!(!player.skill_records_complete_like_cpp());
    assert!(player.skill_records_loaded_like_cpp());
    assert_eq!(player.occupied_skill_slots_like_cpp(), None);
    assert_eq!(
        player.non_durable_skill_tombstones_like_cpp(),
        &BTreeSet::from([333, 755])
    );
    player.replace_represented_skill_records_like_cpp(
        vec![
            (333, row(333, PlayerSkillLoadState::Unchanged, 0)),
            (755, row(755, PlayerSkillLoadState::Changed, 20)),
        ],
        true,
        true,
    );
    assert!(player.skill_records_complete_like_cpp());
    assert_eq!(
        player.non_durable_skill_tombstones_like_cpp(),
        &BTreeSet::from([333])
    );
}
#[test]
fn skill_lifecycle_finalization_normalizes_in_place_and_preserves_last_duplicate() {
    let mut player = Player::new(None, false);
    player.replace_skill_records_like_cpp(
        [
            (333, PlayerSkillLoadState::Deleted, 0),
            (333, PlayerSkillLoadState::Changed, 99),
            (999, PlayerSkillLoadState::Deleted, 0),
            (70000, PlayerSkillLoadState::Deleted, 0),
        ]
        .into_iter()
        .map(|(id, state, value)| PlayerSkillRecord {
            skill_line_id: id,
            current_value: value,
            max_value: value,
            step: 0,
            profession_slot: -1,
            state,
        })
        .collect(),
        true,
        true,
        Some(2),
        BTreeSet::from([755]),
    );
    let records_ptr = player.skill_records_like_cpp().as_ptr();
    player.mark_skill_records_saved_like_cpp();
    assert_eq!(player.skill_records_like_cpp().len(), 2);
    assert_eq!(player.skill_records_like_cpp()[0].current_value, 99);
    assert!(
        player
            .skill_records_like_cpp()
            .iter()
            .all(|r| r.state == PlayerSkillLoadState::Unchanged)
    );
    assert_eq!(
        player.non_durable_skill_tombstones_like_cpp(),
        &BTreeSet::from([755, 999])
    );
    assert_eq!(player.occupied_skill_slots_like_cpp(), Some(2));
    assert_eq!(player.skill_records_like_cpp().as_ptr(), records_ptr);
    player.clear_skill_tombstones_for_identity_change_like_cpp();
    assert!(player.non_durable_skill_tombstones_like_cpp().is_empty());
    assert_eq!(player.skill_records_like_cpp().len(), 2);
    assert_eq!(player.skill_records_like_cpp().as_ptr(), records_ptr);
}
#[test]
fn occupied_skill_slot_authority_clears_invalid_proof_without_replacing_records() {
    let mut player = Player::new(None, false);
    player.replace_skill_records_like_cpp(
        vec![PlayerSkillRecord {
            skill_line_id: 333,
            current_value: 0,
            max_value: 0,
            step: 0,
            profession_slot: -1,
            state: PlayerSkillLoadState::Deleted,
        }],
        true,
        true,
        None,
        BTreeSet::from([333]),
    );
    let records = player.skill_records_like_cpp().to_vec();
    let records_ptr = player.skill_records_like_cpp().as_ptr();
    for (slots, accepted) in [(1, true), (0, false), (257, false), (1, true)] {
        assert_eq!(
            player.authorize_occupied_skill_slots_like_cpp(slots),
            accepted
        );
        assert_eq!(
            player.occupied_skill_slots_like_cpp(),
            accepted.then_some(slots)
        );
        assert_eq!(player.skill_records_like_cpp(), records);
        assert_eq!(player.skill_records_like_cpp().as_ptr(), records_ptr);
        assert_eq!(
            player.non_durable_skill_tombstones_like_cpp(),
            &BTreeSet::from([333])
        );
        assert!(player.skill_records_loaded_like_cpp());
        assert!(player.skill_records_complete_like_cpp());
    }
}
#[test]
fn apply_enchantment_effect_actions_match_cpp_damage_and_totem_attack_slot_rules() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12492, 7465);
    let weapon = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(7465, 1)
    };
    let ranged = ItemStorageTemplate {
        inventory_type: InventoryType::RangedRight,
        ..ItemStorageTemplate::regular_item(7466, 1)
    };

    item.set_slot(EQUIPMENT_SLOT_MAINHAND);
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            Some(&weapon),
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Damage,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
            attack_type: WeaponAttackType::BaseAttack,
            modifier_slot: -1,
        }]
    );
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            Some(&ranged),
            EnchantmentSlot::EnhancementTemporary,
            false,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Totem,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
            attack_type: WeaponAttackType::RangedAttack,
            modifier_slot: EnchantmentSlot::EnhancementTemporary as i16,
        }]
    );

    item.set_slot(EQUIPMENT_SLOT_OFFHAND);
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            Some(&weapon),
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Damage,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
            attack_type: WeaponAttackType::OffAttack,
            modifier_slot: -1,
        }]
    );

    item.set_slot(EQUIPMENT_SLOT_CHEST);
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            Some(&weapon),
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Damage,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::Noop]
    );
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Damage,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::MissingItemTemplateForAttack {
            effect_kind: ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::Damage),
        }]
    );
}
