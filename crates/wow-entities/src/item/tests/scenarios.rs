//! Item entity regressions.
//!
//! Moved out of item.rs under #685; every test is unchanged.

use super::*;

#[test]
fn item_constructor_matches_cpp_base_state() {
    let item = Item::new(1234);

    assert_eq!(item.object().type_id(), TypeId::Item);
    assert_eq!(item.object().type_mask(), TypeMask::OBJECT | TypeMask::ITEM);
    assert_eq!(item.slot(), 0);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_0);
    assert_eq!(item.position(), u16::from(INVENTORY_SLOT_BAG_0) << 8);
    assert!(item.is_equipped());
    assert_eq!(item.update_state(), ItemUpdateState::New);
    assert_eq!(item.queue_pos(), -1);
    assert!(!item.is_in_update_queue());
    assert!(!item.loot_generated());
    assert!(!item.is_in_trade());
    assert_eq!(item.last_played_time_update(), 1234);
    assert_eq!(item.refund_recipient(), ObjectGuid::EMPTY);
    assert_eq!(item.paid_money(), 0);
    assert_eq!(item.paid_extended_cost(), 0);
    assert_eq!(item.text(), "");
    assert!(!item.item_data_changes_mask().is_any_set());
}

#[test]
fn bag_slot_and_position_follow_cpp_container_pointer_shape() {
    let mut item = Item::default();
    item.set_slot(3);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_0);
    assert_eq!(item.position(), (u16::from(INVENTORY_SLOT_BAG_0) << 8) | 3);
    assert!(item.is_equipped());

    item.set_container_guid_and_slot(ObjectGuid::create_item(1, 77), 21);
    assert!(item.is_in_bag());
    assert_eq!(item.bag_slot(), 21);
    assert_eq!(item.position(), (21u16 << 8) | 3);
    assert!(!item.is_equipped());
}

#[test]
fn bind_if_visualized_matches_cpp_bonding_subset() {
    for bonding in [
        ItemBondingType::OnEquip,
        ItemBondingType::OnAcquire,
        ItemBondingType::Quest,
    ] {
        let mut item = Item::default();
        item.set_bonding(bonding);
        item.bind_if_visualized();
        assert!(item.is_soul_bound());
    }

    for bonding in [ItemBondingType::None, ItemBondingType::OnUse] {
        let mut item = Item::default();
        item.set_bonding(bonding);
        item.bind_if_visualized();
        assert!(!item.is_soul_bound());
    }
}

#[test]
fn bind_if_stored_matches_cpp_storeitem_bag_position_rule() {
    for bonding in [ItemBondingType::OnAcquire, ItemBondingType::Quest] {
        let mut item = Item::default();
        item.set_bonding(bonding);
        item.bind_if_stored(false);
        assert!(item.is_soul_bound());
    }

    let mut inventory_item = Item::default();
    inventory_item.set_bonding(ItemBondingType::OnEquip);
    inventory_item.bind_if_stored(false);
    assert!(!inventory_item.is_soul_bound());

    let mut bag_item = Item::default();
    bag_item.set_bonding(ItemBondingType::OnEquip);
    bag_item.bind_if_stored(true);
    assert!(bag_item.is_soul_bound());
}

#[test]
fn initialize_created_state_follows_cpp_create_without_template_lookup() {
    let owner = ObjectGuid::create_player(1, 42);
    let guid = ObjectGuid::create_item(1, 99);
    let mut item = Item::default();

    item.initialize_created_state(ItemCreateInfo {
        guid,
        item_id: 6948,
        context: ItemContext::QuestReward,
        owner: Some(owner),
        max_durability: 17,
        expiration: 3600,
        spell_charges: [0, -1, 2, 0, 0],
    });

    assert_eq!(item.object().guid(), guid);
    assert_eq!(item.object().entry(), 6948);
    assert_eq!(item.object().scale(), 1.0);
    assert_eq!(item.data().owner, owner);
    assert_eq!(item.data().contained_in, owner);
    assert_eq!(item.data().stack_count, 1);
    assert_eq!(item.data().max_durability, 17);
    assert_eq!(item.data().durability, 17);
    assert_eq!(item.data().expiration, 3600);
    assert_eq!(item.data().create_played_time, 0);
    assert_eq!(item.data().context, ItemContext::QuestReward as i32);
    assert_eq!(item.data().spell_charges, [0, -1, 2, 0, 0]);
    assert!(item.item_data_changes_mask().is_set(ITEM_DATA_OWNER_BIT));
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_SPELL_CHARGES_PARENT_BIT)
    );
}

#[test]
fn clone_item_for_store_matches_cpp_cloneitem_field_subset() {
    let owner = ObjectGuid::create_player(1, 42);
    let creator = ObjectGuid::create_player(1, 43);
    let gift_creator = ObjectGuid::create_player(1, 44);
    let mut source = Item::default();
    source.initialize_created_state(ItemCreateInfo {
        guid: ObjectGuid::create_item(1, 100),
        item_id: 6948,
        context: ItemContext::QuestReward,
        owner: Some(owner),
        max_durability: 17,
        expiration: 3600,
        spell_charges: [0, -1, 2, 0, 0],
    });
    source.set_creator(creator);
    source.set_gift_creator(gift_creator);
    source.set_item_flag(
        ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE,
    );
    source.set_soulbound_tradeable([ObjectGuid::create_player(1, 45)]);
    source.set_bonding(ItemBondingType::Quest);

    let clone = source.clone_item_for_store(ObjectGuid::create_item(1, 101), Some(owner), 2);

    assert_eq!(clone.object().guid(), ObjectGuid::create_item(1, 101));
    assert_eq!(clone.object().entry(), source.object().entry());
    assert_eq!(clone.owner_guid(), owner);
    assert_eq!(clone.data().contained_in, owner);
    assert_eq!(clone.count(), 2);
    assert_eq!(clone.data().max_durability, source.data().max_durability);
    assert_eq!(clone.data().durability, source.data().max_durability);
    assert_eq!(clone.data().expiration, source.data().expiration);
    assert_eq!(clone.data().context, source.data().context);
    assert_eq!(clone.data().spell_charges, source.data().spell_charges);
    assert_eq!(clone.data().creator, creator);
    assert_eq!(clone.data().gift_creator, gift_creator);
    assert!(clone.is_soul_bound());
    assert!(!clone.is_refundable());
    assert!(!clone.is_bop_tradeable());
    assert!(clone.soulbound_trade_allowed_guids().is_empty());
    assert_eq!(clone.bonding(), ItemBondingType::Quest);
    assert_eq!(clone.update_state(), ItemUpdateState::New);
}

#[test]
fn item_flag_helpers_match_cpp_dynamic_flags() {
    let mut item = Item::default();

    assert!(item.is_locked());
    item.set_item_flag(ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE);
    assert!(item.is_soul_bound());
    assert!(item.is_refundable());
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_DYNAMIC_FLAGS_BIT)
    );

    item.set_item_flag(ItemFieldFlags::UNLOCKED);
    assert!(!item.is_locked());
    item.remove_item_flag(ItemFieldFlags::REFUNDABLE);
    assert!(!item.is_refundable());

    item.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    item.set_refund_recipient(ObjectGuid::create_player(1, 42));
    item.set_paid_money(10);
    item.set_paid_extended_cost(20);
    item.set_not_refundable();
    item.clear_soulbound_tradeable();
    assert!(!item.is_refundable());
    assert!(!item.is_bop_tradeable());
    assert_eq!(item.refund_recipient(), ObjectGuid::EMPTY);
    assert_eq!(item.paid_money(), 0);
    assert_eq!(item.paid_extended_cost(), 0);

    item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
    assert!(item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_DYNAMIC_FLAGS2_BIT)
    );
}

#[test]
fn soulbound_trade_allowed_guids_match_cpp_set_and_clear() {
    let allowed = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 43);
    let mut item = Item::default();

    item.set_soulbound_tradeable([allowed]);
    assert!(item.is_bop_tradeable());
    assert!(item.is_soulbound_trade_allowed_for(allowed));
    assert!(!item.is_soulbound_trade_allowed_for(other));
    assert_eq!(item.soulbound_trade_allowed_guids().len(), 1);

    assert!(item.clear_soulbound_tradeable());
    assert!(!item.is_bop_tradeable());
    assert!(item.soulbound_trade_allowed_guids().is_empty());

    item.set_soulbound_tradeable(std::iter::empty());
    assert!(item.is_bop_tradeable());
    assert!(!item.clear_soulbound_tradeable());
    assert!(!item.is_bop_tradeable());
}

#[test]
fn played_time_and_bop_trade_expiry_match_cpp_thresholds() {
    let mut item = Item::new(1_000);
    item.set_create_played_time(100);

    assert_eq!(item.played_time(1_030), 130);
    assert!(!item.is_refund_expired_at(1_000 + 7_100));
    assert!(item.is_refund_expired_at(1_000 + 7_101));

    assert!(!item.is_soulbound_trade_expired(100 + BOP_TRADEABLE_DURATION_SECS));
    assert!(item.is_soulbound_trade_expired(101 + BOP_TRADEABLE_DURATION_SECS));
}

#[test]
fn is_binded_not_with_matches_cpp_representable_cases() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let template = ItemStorageTemplate::regular_item(6948, 1);
    let mut item = Item::default();

    assert!(!item.is_binded_not_with(player_guid, &template, false));

    item.set_item_flag(ItemFieldFlags::SOULBOUND);
    item.set_owner_guid(player_guid);
    assert!(!item.is_binded_not_with(player_guid, &template, false));

    item.set_owner_guid(other_guid);
    assert!(item.is_binded_not_with(player_guid, &template, false));

    item.set_item_flag(ItemFieldFlags::BOP_TRADEABLE);
    assert!(!item.is_binded_not_with(player_guid, &template, true));
    assert!(item.is_binded_not_with(player_guid, &template, false));

    item.set_soulbound_tradeable([player_guid]);
    assert!(!item.is_binded_not_with_using_allowed_guids(player_guid, &template));

    item.set_soulbound_tradeable([ObjectGuid::create_player(1, 44)]);
    assert!(item.is_binded_not_with_using_allowed_guids(player_guid, &template));

    let account_bound_template = ItemStorageTemplate {
        flags: ItemFlags::IS_BOUND_TO_ACCOUNT,
        ..template
    };
    assert!(!item.is_binded_not_with(player_guid, &account_bound_template, false));
}

#[test]
fn spell_charges_and_enchantments_mark_cpp_array_bits() {
    let mut item = Item::default();

    item.set_spell_charges(2, -3);
    assert_eq!(item.data().spell_charges[2], -3);
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_SPELL_CHARGES_PARENT_BIT)
    );
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_SPELL_CHARGES_FIRST_BIT + 2)
    );

    item.clear_item_data_changes();
    item.set_enchantment(EnchantmentSlot::EnhancementSocket, 777, 120, 5);
    assert_eq!(item.data().enchantments[2].id, 777);
    assert_eq!(item.data().enchantments[2].duration, 120);
    assert_eq!(item.data().enchantments[2].charges, 5);
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_ENCHANTMENT_PARENT_BIT)
    );
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_ENCHANTMENT_FIRST_BIT + 2)
    );
}

#[test]
fn can_be_merged_partly_with_matches_cpp_guards() {
    let mut item = Item::default();
    item.object_mut().set_entry(6948);
    item.set_count(4);

    assert_eq!(
        item.can_be_merged_partly_with(6948, 20),
        InventoryResult::Ok
    );
    assert_eq!(
        item.can_be_merged_partly_with(6949, 20),
        InventoryResult::CantStack
    );

    item.set_count(20);
    assert_eq!(
        item.can_be_merged_partly_with(6948, 20),
        InventoryResult::CantStack
    );

    item.set_count(4);
    item.set_loot_generated(true);
    assert_eq!(
        item.can_be_merged_partly_with(6948, 20),
        InventoryResult::LootGone
    );
}

#[test]
fn item_can_go_into_bag_matches_cpp_container_family_rules() {
    let regular_bag = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 16,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let herb_bag = ItemStorageTemplate {
        subclass_id: ItemSubClassContainer::HerbContainer as u32,
        ..regular_bag
    };
    let reagent_bag = ItemStorageTemplate {
        subclass_id: ItemSubClassContainer::ReagentContainer as u32,
        ..regular_bag
    };
    let quiver = ItemStorageTemplate {
        class_id: ItemClass::Quiver,
        subclass_id: ItemSubClassQuiver::Quiver as u32,
        ..regular_bag
    };
    let ammo_pouch = ItemStorageTemplate {
        class_id: ItemClass::Quiver,
        subclass_id: ItemSubClassQuiver::AmmoPouch as u32,
        ..regular_bag
    };
    let herb = ItemStorageTemplate {
        bag_family: BagFamilyMask::HERBS,
        ..ItemStorageTemplate::regular_item(2447, 20)
    };
    let arrow = ItemStorageTemplate {
        bag_family: BagFamilyMask::ARROWS,
        ..ItemStorageTemplate::regular_item(2512, 200)
    };
    let bullet = ItemStorageTemplate {
        bag_family: BagFamilyMask::BULLETS,
        ..ItemStorageTemplate::regular_item(2516, 200)
    };
    let reagent = ItemStorageTemplate {
        is_crafting_reagent: true,
        ..ItemStorageTemplate::regular_item(3371, 20)
    };
    let misc = ItemStorageTemplate::regular_item(6948, 1);

    assert!(item_can_go_into_bag(&misc, &regular_bag));
    assert!(item_can_go_into_bag(&herb, &herb_bag));
    assert!(!item_can_go_into_bag(&misc, &herb_bag));
    assert!(item_can_go_into_bag(&reagent, &reagent_bag));
    assert!(!item_can_go_into_bag(&misc, &reagent_bag));
    assert!(item_can_go_into_bag(&arrow, &quiver));
    assert!(!item_can_go_into_bag(&bullet, &quiver));
    assert!(item_can_go_into_bag(&bullet, &ammo_pouch));
    assert!(!item_can_go_into_bag(&arrow, &ammo_pouch));
    assert!(!item_can_go_into_bag(&misc, &misc));
}

#[test]
fn can_change_equip_state_in_combat_matches_cpp_template_helper() {
    let shield = ItemStorageTemplate {
        inventory_type: InventoryType::Shield,
        ..ItemStorageTemplate::regular_item(1, 1)
    };
    let holdable = ItemStorageTemplate {
        inventory_type: InventoryType::Holdable,
        ..ItemStorageTemplate::regular_item(2, 1)
    };
    let relic = ItemStorageTemplate {
        inventory_type: InventoryType::Relic,
        ..ItemStorageTemplate::regular_item(3, 1)
    };
    let weapon = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        inventory_type: InventoryType::Head,
        ..ItemStorageTemplate::regular_item(4, 1)
    };
    let projectile = ItemStorageTemplate {
        class_id: ItemClass::Projectile,
        inventory_type: InventoryType::NonEquip,
        ..ItemStorageTemplate::regular_item(5, 1)
    };
    let armor = ItemStorageTemplate {
        class_id: ItemClass::Armor,
        inventory_type: InventoryType::Chest,
        ..ItemStorageTemplate::regular_item(6, 1)
    };

    assert!(shield.can_change_equip_state_in_combat());
    assert!(holdable.can_change_equip_state_in_combat());
    assert!(relic.can_change_equip_state_in_combat());
    assert!(weapon.can_change_equip_state_in_combat());
    assert!(projectile.can_change_equip_state_in_combat());
    assert!(!armor.can_change_equip_state_in_combat());
}

#[test]
fn item_modifiers_mark_cpp_modifiers_bit() {
    let mut item = Item::default();
    item.clear_item_data_changes();

    item.set_modifier(ItemModifier::TransmogAppearanceSpec2, 1234);

    assert_eq!(
        item.get_modifier(ItemModifier::TransmogAppearanceSpec2),
        1234
    );
    assert!(
        item.item_data_changes_mask()
            .is_set(ITEM_DATA_MODIFIERS_BIT)
    );
}

#[test]
fn visible_entry_and_appearance_follow_cpp_transmog_precedence() {
    let mut item = Item::default();
    item.object_mut().set_entry(19019);
    item.set_appearance_mod_id(7);
    item.set_modifier(ItemModifier::TransmogAppearanceAllSpecs, 10);

    let lookup = |id| match id {
        10 => Some((25_000, 3)),
        20 => Some((26_000, 4)),
        _ => None,
    };

    assert_eq!(item.visible_entry(1, lookup), 25_000);
    assert_eq!(item.visible_appearance_mod_id(1, lookup), 3);

    item.set_modifier(ItemModifier::TransmogAppearanceSpec2, 20);
    assert_eq!(item.visible_entry(1, lookup), 26_000);
    assert_eq!(item.visible_appearance_mod_id(1, lookup), 4);

    item.set_modifier(ItemModifier::TransmogAppearanceSpec2, 999);
    assert_eq!(item.visible_entry(1, lookup), 19019);
    assert_eq!(item.visible_appearance_mod_id(1, lookup), 7);
}

#[test]
fn visible_item_visual_follows_cpp_illusion_then_permanent_enchant() {
    let mut item = Item::default();
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 500, 0, 0);

    let enchant_visual = |id| match id {
        500 => Some(12),
        700 => Some(34),
        800 => Some(56),
        _ => None,
    };

    assert_eq!(item.visible_enchantment_id(0), 500);
    assert_eq!(item.visible_item_visual(0, enchant_visual), 12);

    item.set_modifier(ItemModifier::EnchantIllusionAllSpecs, 700);
    assert_eq!(item.visible_enchantment_id(1), 700);
    assert_eq!(item.visible_item_visual(1, enchant_visual), 34);

    item.set_modifier(ItemModifier::EnchantIllusionSpec2, 800);
    assert_eq!(item.visible_enchantment_id(1), 800);
    assert_eq!(item.visible_item_visual(1, enchant_visual), 56);
}

#[test]
fn visible_secondary_modified_appearance_uses_spec_then_all_specs() {
    let mut item = Item::default();

    item.set_modifier(ItemModifier::TransmogSecondaryAppearanceAllSpecs, 11);
    assert_eq!(item.visible_secondary_modified_appearance_id(2), 11);

    item.set_modifier(ItemModifier::TransmogSecondaryAppearanceSpec3, 22);
    assert_eq!(item.visible_secondary_modified_appearance_id(2), 22);
}

#[test]
fn state_transition_preserves_new_until_saved_like_cpp() {
    let mut item = Item::default();

    assert_eq!(
        item.set_state(ItemUpdateState::Changed),
        ItemStateTransition::Updated
    );
    assert_eq!(item.update_state(), ItemUpdateState::New);
    assert_eq!(
        item.set_state(ItemUpdateState::Removed),
        ItemStateTransition::PretendNeverExisted
    );

    item.force_state(ItemUpdateState::Unchanged);
    item.set_queue_pos(7);
    assert_eq!(
        item.set_state(ItemUpdateState::Changed),
        ItemStateTransition::Updated
    );
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert!(item.is_in_update_queue());

    item.set_state(ItemUpdateState::Unchanged);
    assert_eq!(item.update_state(), ItemUpdateState::Unchanged);
    assert!(!item.is_in_update_queue());
}

#[test]
fn values_update_sets_item_type_bit() {
    let mut item = Item::default();

    item.set_count(3);
    let update = item.values_update();

    assert!(update.has_data());
    assert_eq!(
        update.changed_object_type_mask & (1 << TYPEID_ITEM),
        1 << TYPEID_ITEM
    );
    assert!(update.object_data.is_none());
    assert!(update.item_data.is_some());
}
