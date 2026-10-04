// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

fn entry(id: u32, flags: wow_constants::SpellItemEnchantmentFlags) -> wow_data::SpellItemEnchantmentEntry {
    wow_data::SpellItemEnchantmentEntry {
        id, flags, effect_arg: [0; 3], effect_points_min: [0; 3],
        effect_scaling_points: [0.0; 3], item_visual: 0,
        required_skill_id: 0, required_skill_rank: 0, item_level: 1,
        charges: 0, effect: [0; 3], condition_id: 0, min_level: 1, max_level: 0,
    }
}

#[test]
fn enchantment_persistence_missing_catalog_zeros_fields_without_clearing_live_item() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 30_220)));
    let guid = ObjectGuid::create_item(1, 30_220);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, guid, 30_220, InventoryType::NonEquip);
    session.update_inventory_item_object_like_cpp(guid, |item| {
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 940, 3_000, 2);
    });
    let before = session.resolved_inventory_item_object_like_cpp(guid).unwrap().data().enchantments.clone();
    let _ = drain_server_opcodes(&send_rx);
    let (persisted, cleared) = session.inventory_remove_enchantment_persistence_like_cpp(guid, true).unwrap();
    assert_eq!(persisted, "0 0 0 ".repeat(before.len()));
    assert!(cleared.is_empty());
    assert_eq!(session.resolved_inventory_item_object_like_cpp(guid).unwrap().data().enchantments, before);
    assert!(drain_server_opcodes(&send_rx).is_empty());
    assert!(session.inventory_remove_enchantment_persistence_like_cpp(ObjectGuid::create_item(1, 30_221), false).is_none());
}

#[test]
fn enchantment_persistence_selected_reads_canonical_duration_and_item_fallback_without_mutation() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_222);
    let registry = Arc::new(PlayerRegistry::default());
    bind_canonical_test_player_to_registry_like_cpp(&mut session, &registry, owner, Position::ZERO, 571);
    session.set_player_guid(Some(owner));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    let guid = ObjectGuid::create_item(1, 30_222);
    equip_represented_test_item_like_cpp(&mut session, EQUIPMENT_SLOT_MAINHAND, guid, 30_222, InventoryType::Weapon);
    session.set_spell_item_enchantment_store(Arc::new(wow_data::SpellItemEnchantmentStore::from_entries([
        entry(940, wow_constants::SpellItemEnchantmentFlags::MAINHAND_ONLY),
        entry(941, wow_constants::SpellItemEnchantmentFlags::empty()),
        entry(942, wow_constants::SpellItemEnchantmentFlags::DO_NOT_SAVE_TO_DB),
        entry(943, wow_constants::SpellItemEnchantmentFlags::empty()),
    ])));
    session.update_inventory_item_object_like_cpp(guid, |item| {
        item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 940, 4_000, 2);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 941, 3_000, 1);
        item.set_enchantment(EnchantmentSlot::Property0, 942, 2_000, 3);
        item.set_enchantment(EnchantmentSlot::Property1, 943, 1_000, 4);
    });
    let mut timed = session.resolved_inventory_item_object_like_cpp(guid).unwrap();
    session.mutate_canonical_player_like_cpp(|player| {
        player.add_enchantment_duration(&mut timed, EnchantmentSlot::EnhancementTemporary, 1_500)
    }).unwrap();
    let before = session.resolved_inventory_item_object_like_cpp(guid).unwrap().data().enchantments.clone();
    let _ = drain_server_opcodes(&send_rx);
    let (persisted, cleared) = session.inventory_remove_enchantment_persistence_like_cpp(guid, true).unwrap();
    let fields: Vec<_> = persisted.split_whitespace().collect();
    assert_eq!(cleared, vec![EnchantmentSlot::EnhancementPermanent]);
    assert_eq!(&fields[0..3], &["0", "0", "0"]);
    assert_eq!(&fields[3..6], &["941", "1500", "1"]);
    assert_eq!(&fields[24..27], &["0", "0", "0"]);
    assert_eq!(&fields[27..30], &["943", "1000", "4"]);
    assert!(persisted.ends_with(' '));
    let (not_cleared, slots) = session.inventory_remove_enchantment_persistence_like_cpp(guid, false).unwrap();
    assert!(not_cleared.starts_with("940 4000 2 "));
    assert!(slots.is_empty());
    assert_eq!(session.resolved_inventory_item_object_like_cpp(guid).unwrap().data().enchantments, before);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn enchantment_persistence_stale_same_guid_owner_does_not_read_available_fixture_item() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_223);
    session.set_player_guid(Some(owner));
    let guid = ObjectGuid::create_item(1, 30_223);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, guid, 30_223, InventoryType::NonEquip);
    assert!(session.inventory_remove_enchantment_persistence_like_cpp(guid, false).is_some());
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(owner, "StaleEnchantPersistence".to_string(), Position::ZERO, 571, 1, 1, 20, 0));
    session.ensure_canonical_world_map_for_current_player_like_cpp().unwrap();
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, guid, 30_223, InventoryType::NonEquip);
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(owner);
    let handle = canonical.lock().unwrap().install_detached_player_like_cpp(replacement).unwrap();
    let _ = drain_server_opcodes(&send_rx);
    assert!(session.inventory_remove_enchantment_persistence_like_cpp(guid, false).is_none());
    assert!(session.inventory_item_objects_like_cpp().contains_key(&guid));
    assert_eq!(canonical.lock().unwrap().with_player_like_cpp(handle, |player| player.inventory_runtime_like_cpp().item_objects().is_empty()), Some(true));
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
