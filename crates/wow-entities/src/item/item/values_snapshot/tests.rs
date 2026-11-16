use std::collections::HashSet;

use wow_constants::{
    EnchantmentSlot, ItemBondingType, ItemFieldFlags, ItemFieldFlags2, ItemModifier,
    ItemUpdateState, TypeId, TypeMask,
};
use wow_core::ObjectGuid;

use crate::{
    ArtifactPower, ITEM_DATA_GEMS_BIT, ITEM_DATA_PARENT_BIT, ITEM_DATA_STACK_COUNT_BIT, Item,
    ItemBonusKey, ItemStorageTemplate, ItemValuesSnapshot, ObjectChangedFields, ObjectNotifyFlags,
    SocketedGem,
};

fn seeded_item() -> Item {
    let owner = ObjectGuid::create_player(1, 77);
    let container = ObjectGuid::create_item(1, 900);
    let looter = ObjectGuid::create_player(1, 78);
    let mut item = Item::new(-31);
    item.object_mut().create(ObjectGuid::create_item(1, 901));
    item.object_mut().bind_map(55, 66);
    item.object_mut()
        .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    item.object_mut().add_to_world();
    item.object_mut().set_entry(u32::MAX);
    item.object_mut().set_scale(f32::from_bits(0x8000_0000));
    item.object_mut().replace_all_dynamic_flags(0x8123_4567);

    item.set_owner_guid(owner);
    item.set_contained_in(container);
    item.set_creator(ObjectGuid::create_player(1, 79));
    item.set_gift_creator(ObjectGuid::create_player(1, 80));
    item.set_count(9);
    item.set_expiration(71);
    item.replace_all_item_flags(
        ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE | ItemFieldFlags::WRAPPED,
    );
    item.replace_all_item_flags2(ItemFieldFlags2::from_bits_retain(0x8000_0001));
    item.set_binding(true);
    item.set_property_seed(-17);
    item.set_random_properties_id(-19);
    item.set_durability(6);
    item.set_max_durability(20);
    item.set_create_played_time(41);
    item.set_context_value(-3);
    item.set_create_time(-99);
    item.set_artifact_xp(0x1234_5678_9ABC_DEF0);
    item.set_appearance_mod_id(7);
    item.set_debug_item_level(344);
    item.set_item_bonus_key(ItemBonusKey {
        item_id: -21,
        bonus_list_ids: vec![8, 13, 21],
    });
    for index in 0..crate::MAX_ITEM_SPELLS {
        item.set_spell_charges(index, (index as i32) - 2);
    }
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 777, 83, -4);
    item.set_artifact_powers(vec![ArtifactPower {
        artifact_power_id: -5,
        purchased_rank: 3,
        current_rank_with_bonus: 4,
    }]);
    item.set_gems(vec![SocketedGem {
        item_id: -31,
        context: 6,
        bonus_list_ids: vec![5, 9],
    }]);
    item.set_modifier(ItemModifier::TransmogAppearanceAllSpecs, 81);
    item.set_slot(12);
    item.set_container_guid_and_slot(container, 4);
    item.set_bonding(ItemBondingType::OnEquip);
    item.force_state(ItemUpdateState::Changed);
    item.set_queue_pos(17);
    item.set_loot_generated(true);
    item.set_in_trade(true);
    item.set_last_played_time_update(-45);
    item.set_refund_recipient(owner);
    item.set_paid_money(0x1122_3344_5566_7788);
    item.set_paid_extended_cost(0xAABB_CCDD);
    item.set_soulbound_tradeable([looter]);
    item.set_text("captured text");
    item
}

fn storage_overlay_item(in_world: bool, already_updated: bool) -> Item {
    let mut item = Item::new(73);
    item.object_mut().create(ObjectGuid::create_item(1, 919));
    item.object_mut().set_entry(10);
    item.set_count(4);
    item.set_slot(2);
    item.set_container_guid_and_slot(ObjectGuid::create_item(1, 920), 5);
    item.set_binding(true);
    item.force_state(ItemUpdateState::Changed);
    item.set_queue_pos(6);
    item.set_loot_generated(true);
    item.set_in_trade(true);
    item.clear_item_data_changes();
    item.object_mut().clear_update_mask(false);
    if in_world {
        item.object_mut().add_to_world();
    }
    if already_updated {
        assert!(in_world);
        item.object_mut().set_scale(1.5);
    }
    item
}

#[test]
fn storage_query_overlay_matches_ordered_live_setters_and_preserves_other_facts() {
    for (entry, count, changes_entry, changes_count) in [
        (20, 9, true, true),
        (20, 4, true, false),
        (10, 9, false, true),
        (10, 4, false, false),
    ] {
        for (in_world, already_updated) in [(false, false), (true, false), (true, true)] {
            let source = storage_overlay_item(in_world, already_updated);
            let mut live = storage_overlay_item(in_world, already_updated);

            let projected =
                ItemValuesSnapshot::capture(&source).for_storage_query_overlay(entry, count, 7);
            live.object_mut().set_entry(entry);
            live.set_count(count);
            live.set_slot(7);
            let expected = ItemValuesSnapshot::capture(&live);

            assert_eq!(projected, expected);
            assert_eq!(projected.object().entry(), entry);
            assert_eq!(projected.data().stack_count, count);
            assert_eq!(
                projected
                    .item_data_changes_mask()
                    .is_set(ITEM_DATA_PARENT_BIT),
                changes_count
            );
            assert_eq!(
                projected
                    .item_data_changes_mask()
                    .is_set(ITEM_DATA_STACK_COUNT_BIT),
                changes_count
            );
            assert_eq!(
                projected
                    .object()
                    .changed_fields()
                    .contains(ObjectChangedFields::ENTRY_ID),
                changes_entry
            );
            assert_eq!(projected.container_guid(), ObjectGuid::create_item(1, 920));
            assert_eq!(projected.container_slot(), 5);
            assert_eq!(projected.update_state(), ItemUpdateState::Changed);
            assert_eq!(projected.queue_pos(), 6);
            assert!(projected.loot_generated());
            assert!(projected.is_in_trade());
            assert_eq!(
                projected.object().state().object_updated,
                already_updated || (in_world && changes_entry)
            );
        }
    }
}

#[test]
fn snapshot_owns_full_item_and_object_facts_after_source_mutation() {
    let mut item = seeded_item();
    let snapshot = ItemValuesSnapshot::capture(&item);

    assert_eq!(snapshot.object().guid(), item.object().guid());
    assert_eq!(snapshot.object().type_id(), TypeId::Item);
    assert_eq!(
        snapshot.object().type_mask(),
        TypeMask::OBJECT | TypeMask::ITEM
    );
    assert_eq!(snapshot.object().entry(), u32::MAX);
    assert_eq!(
        snapshot.object().scale().to_bits(),
        item.object().scale().to_bits()
    );
    assert_eq!(snapshot.object().dynamic_flags(), 0x8123_4567);
    assert_eq!(snapshot.object().map_id(), Some(55));
    assert_eq!(snapshot.object().instance_id(), Some(66));
    assert_eq!(
        snapshot.object().notify_flags(),
        ObjectNotifyFlags::VISIBILITY_CHANGED
    );
    assert_eq!(snapshot.data(), item.data());
    assert_eq!(
        snapshot.item_data_changes_mask(),
        item.item_data_changes_mask()
    );
    assert_eq!(snapshot.slot(), 12);
    assert_eq!(snapshot.container_guid(), ObjectGuid::create_item(1, 900));
    assert_eq!(snapshot.container_slot(), 4);
    assert_eq!(snapshot.bonding(), ItemBondingType::OnEquip);
    assert_eq!(snapshot.update_state(), ItemUpdateState::Changed);
    assert_eq!(snapshot.queue_pos(), 17);
    assert!(snapshot.loot_generated());
    assert!(snapshot.is_in_trade());
    assert_eq!(snapshot.last_played_time_update(), -45);
    assert_eq!(
        snapshot.refund_recipient(),
        ObjectGuid::create_player(1, 77)
    );
    assert_eq!(snapshot.paid_money(), 0x1122_3344_5566_7788);
    assert_eq!(snapshot.paid_extended_cost(), 0xAABB_CCDD);
    assert_eq!(
        snapshot.soulbound_trade_allowed_guids(),
        &HashSet::from([ObjectGuid::create_player(1, 78)])
    );
    assert_eq!(snapshot.text(), "captured text");

    let view = snapshot.read_view();
    assert_eq!(view.object().entry(), u32::MAX);
    assert_eq!(view.data(), snapshot.data());
    assert_eq!(view.text(), "captured text");

    item.object_mut().set_entry(123);
    item.object_mut().set_scale(f32::NAN);
    item.object_mut().replace_all_dynamic_flags(3);
    item.object_mut().reset_all_notifies();
    item.object_mut().clear_map_binding();
    item.set_owner_guid(ObjectGuid::EMPTY);
    item.set_count(1);
    item.set_gems(Vec::new());
    item.set_slot(0);
    item.set_container_guid(ObjectGuid::EMPTY);
    item.set_bonding(ItemBondingType::None);
    item.force_state(ItemUpdateState::Removed);
    item.set_queue_pos(-1);
    item.set_loot_generated(false);
    item.set_in_trade(false);
    item.set_text("new text");
    item.clear_item_data_changes();
    item.object_mut().clear_update_mask(false);

    assert_eq!(snapshot.object().entry(), u32::MAX);
    assert_eq!(snapshot.object().scale().to_bits(), 0x8000_0000);
    assert_eq!(snapshot.object().dynamic_flags(), 0x8123_4567);
    assert_eq!(snapshot.data().owner, ObjectGuid::create_player(1, 77));
    assert_eq!(snapshot.data().stack_count, 9);
    assert_eq!(snapshot.data().gems[0].item_id, -31);
    assert_eq!(snapshot.data().gems[0].bonus_list_ids, [5, 9]);
    assert_eq!(snapshot.slot(), 12);
    assert_eq!(snapshot.update_state(), ItemUpdateState::Changed);
    assert_eq!(snapshot.queue_pos(), 17);
    assert_eq!(snapshot.text(), "captured text");
    assert!(snapshot.item_data_changes_mask().is_any_set());
    assert_eq!(
        snapshot.object().notify_flags(),
        ObjectNotifyFlags::VISIBILITY_CHANGED
    );
    assert_eq!(snapshot.object().map_id(), Some(55));
    assert_eq!(snapshot.object().instance_id(), Some(66));
}

#[test]
fn live_and_captured_views_share_item_rules_and_external_inputs() {
    let mut item = seeded_item();
    item.set_count(3);
    let snapshot = ItemValuesSnapshot::capture(&item);
    let live = item.read_view();
    let captured = snapshot.read_view();

    assert_eq!(item.is_in_bag(), captured.is_in_bag());
    assert_eq!(item.bag_slot(), captured.bag_slot());
    assert_eq!(item.position(), captured.position());
    assert_eq!(item.is_equipped(), captured.is_equipped());
    assert_eq!(
        item.can_be_merged_partly_with(u32::MAX, 10),
        captured.can_be_merged_partly_with(u32::MAX, 10)
    );
    assert_eq!(
        live.has_item_flag(ItemFieldFlags::SOULBOUND),
        captured.has_item_flag(ItemFieldFlags::SOULBOUND)
    );
    assert_eq!(
        live.has_item_flag2(ItemFieldFlags2::from_bits_retain(1)),
        captured.has_item_flag2(ItemFieldFlags2::from_bits_retain(1))
    );
    assert_eq!(live.is_refundable(), captured.is_refundable());
    assert_eq!(live.is_bop_tradeable(), captured.is_bop_tradeable());
    assert_eq!(
        live.is_binded_not_with_using_allowed_guids(
            ObjectGuid::create_player(1, 81),
            &ItemStorageTemplate::regular_item(1, 20)
        ),
        captured.is_binded_not_with_using_allowed_guids(
            ObjectGuid::create_player(1, 81),
            &ItemStorageTemplate::regular_item(1, 20)
        )
    );
    assert_eq!(item.played_time(100), captured.played_time(100));
    assert_eq!(
        item.is_refund_expired_at(100),
        captured.is_refund_expired_at(100)
    );
    assert_eq!(
        item.is_soulbound_trade_expired(200),
        captured.is_soulbound_trade_expired(200)
    );
    assert_eq!(item.is_wrapped(), captured.is_wrapped());
    assert_eq!(item.is_locked(), captured.is_locked());
    assert_eq!(item.is_broken(), captured.is_broken());
    assert_eq!(
        item.get_modifier(ItemModifier::TransmogAppearanceAllSpecs),
        captured.get_modifier(ItemModifier::TransmogAppearanceAllSpecs)
    );
    assert_eq!(
        item.visible_enchantment_id(0),
        captured.visible_enchantment_id(0)
    );
    assert_eq!(
        item.visible_secondary_modified_appearance_id(0),
        captured.visible_secondary_modified_appearance_id(0)
    );
    assert_eq!(
        snapshot.changed_object_type_mask(),
        item.changed_object_type_mask()
    );
    assert_eq!(snapshot.values_update(), item.values_update());

    let calls = std::cell::RefCell::new(Vec::new());
    let display = captured.display_id_like_cpp(
        0,
        |modifier| {
            calls.borrow_mut().push(("modified", modifier));
            None
        },
        |entry, appearance| {
            calls.borrow_mut().push(("physical", entry));
            calls
                .borrow_mut()
                .push(("appearance", u32::from(appearance)));
            Some(999)
        },
    );
    assert_eq!(display, Some(999));
    assert_eq!(
        calls.into_inner(),
        vec![("modified", 81), ("physical", u32::MAX), ("appearance", 7)]
    );

    let fallback_calls = std::cell::Cell::new(0);
    let display = captured.display_id_like_cpp(
        0,
        |_| Some(555),
        |_, _| {
            fallback_calls.set(fallback_calls.get() + 1);
            None
        },
    );
    assert_eq!(display, Some(555));
    assert_eq!(fallback_calls.get(), 0, "physical fallback remains lazy");

    let view = snapshot.read_view();
    assert_eq!(view.values_update(), snapshot.values_update());
    assert_eq!(view.data().gems[0].item_id, -31);
    assert_eq!(view.object().entry(), u32::MAX);
    assert_eq!(view.text(), "captured text");
}

#[test]
fn read_views_compare_item_data_referents_not_guid_or_value_equality() {
    let first = seeded_item();
    let equal_but_distinct = seeded_item();
    let first_view = first.read_view();

    assert_eq!(first.object().guid(), equal_but_distinct.object().guid());
    assert_eq!(first.data(), equal_but_distinct.data());
    assert!(first_view.shares_read_source(&first.read_view()));
    assert!(!first_view.shares_read_source(&equal_but_distinct.read_view()));

    let snapshot = ItemValuesSnapshot::capture(&first);
    assert!(
        snapshot
            .read_view()
            .shares_read_source(&snapshot.read_view())
    );
    assert!(!first_view.shares_read_source(&snapshot.read_view()));

    let cloned_snapshot = snapshot.clone();
    assert_eq!(snapshot.data(), cloned_snapshot.data());
    assert!(
        !snapshot
            .read_view()
            .shares_read_source(&cloned_snapshot.read_view())
    );
}

#[test]
fn item_projection_keeps_empty_masks_and_raw_values_separate() {
    let mut item = seeded_item();
    item.clear_item_data_changes();
    item.object_mut().clear_update_mask(false);
    let snapshot = ItemValuesSnapshot::capture(&item);

    assert!(!snapshot.item_data_changes_mask().is_any_set());
    assert_eq!(snapshot.data().gems.len(), 1);
    assert_eq!(snapshot.data().stack_count, 9);
    assert_eq!(snapshot.object().entry(), u32::MAX);
    assert_eq!(snapshot.values_update().item_data, None);
    assert_eq!(snapshot.values_update().object_data, None);

    let mut changed = Item::new(0);
    changed.object_mut().create(ObjectGuid::create_item(1, 902));
    changed.object_mut().set_entry(5);
    changed.set_gems(vec![SocketedGem {
        item_id: 77,
        ..SocketedGem::default()
    }]);
    let values = changed.values_snapshot().values_update();
    assert!(values.changed_object_type_mask != 0);
    assert!(values.object_data.is_some());
    assert!(
        values
            .item_data
            .as_ref()
            .unwrap()
            .mask
            .is_set(ITEM_DATA_GEMS_BIT)
    );
}
