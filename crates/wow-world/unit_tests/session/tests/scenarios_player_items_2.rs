//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;
use wow_constants::ItemFieldFlags2;

#[path = "scenarios_player_items_2/committed_relocation.rs"]
mod committed_relocation;
#[path = "scenarios_player_items_2/committed_swap.rs"]
mod committed_swap;
#[path = "scenarios_player_items_2/enchantment_persistence.rs"]
mod enchantment_persistence;
#[path = "scenarios_player_items_2/item_publication.rs"]
mod item_publication;
#[path = "scenarios_player_items_2/position_publication.rs"]
mod position_publication;

#[test]
fn inventory_swap_effects_non_equipment_remove_duration_and_tradeable_refs() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 30_180);
    let item_guid = ObjectGuid::create_item(1, 30_180);
    let registry = Arc::new(PlayerRegistry::default());
    bind_canonical_test_player_to_registry_like_cpp(
        &mut session,
        &registry,
        guid,
        Position::ZERO,
        571,
    );
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        item_guid,
        30_180,
        InventoryType::NonEquip,
    );
    session.update_inventory_item_object_like_cpp(item_guid, |item| {
        item.set_expiration(300);
        item.set_soulbound_tradeable([guid]);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 905, 12_000, 0);
    });
    let mut item = session
        .resolved_inventory_item_object_like_cpp(item_guid)
        .unwrap();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.add_tradeable_item(&item);
        let _ = player.add_item_durations(&item);
        let _ = player.add_enchantment_duration(
            &mut item,
            EnchantmentSlot::EnhancementTemporary,
            7_000,
        );
    });
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .soulbound_tradeable_items()
            .contains(&item_guid)),
        Some(true)
    );
    let _ = drain_server_opcodes(&send_rx);
    assert!(!session.apply_inventory_item_remove_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        item_guid,
        &[]
    ));
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| (
            player.soulbound_tradeable_items().contains(&item_guid),
            player.item_durations().is_empty(),
            player.enchant_durations().is_empty(),
        )),
        Some((false, true, true))
    );
    assert_eq!(
        session
            .resolved_inventory_item_object_like_cpp(item_guid)
            .unwrap()
            .data()
            .enchantments[EnchantmentSlot::EnhancementTemporary as usize]
            .duration,
        7_000
    );
    assert!(
        session
            .represented_item_mod_reapply_events_like_cpp()
            .is_empty()
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
    assert!(!session.apply_inventory_item_store_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        item_guid
    ));
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| (
            player.item_durations().len(),
            player.enchant_durations().len()
        )),
        Some((1, 1))
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::ItemTimeUpdate,
            ServerOpcodes::ItemEnchantTimeUpdate
        ]
    );
}

#[test]
fn inventory_swap_effects_equipment_records_mods_and_clears_enchantment() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_item(1, 30_181);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 30_181)));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        guid,
        30_181,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(guid, |item| {
        item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 905, 12_000, 0);
    });
    let _ = session.apply_inventory_item_remove_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        guid,
        &[EnchantmentSlot::EnhancementTemporary],
    );
    let item = session
        .resolved_inventory_item_object_like_cpp(guid)
        .unwrap();
    assert!(!item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id,
        0
    );
    let _ = session.apply_inventory_item_store_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        guid,
    );
    assert!(
        session
            .resolved_inventory_item_object_like_cpp(guid)
            .unwrap()
            .has_item_flag2(ItemFieldFlags2::EQUIPPED)
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[
            RepresentedItemModsReapplyEventLikeCpp {
                item_guid: guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                apply: false
            },
            RepresentedItemModsReapplyEventLikeCpp {
                item_guid: guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                apply: true
            },
        ]
    );
}

#[test]
fn inventory_swap_effects_broken_item_skips_mods_but_sets_equipped() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_item(1, 30_182);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 30_182)));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 718,
        name: "Broken swap set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 30_182 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 38,
            chr_spec_id: 0,
            spell_id: 9038,
            threshold: 1,
            item_set_id: 718,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        guid,
        30_182,
        InventoryType::Chest,
    );
    session.update_inventory_item_object_like_cpp(guid, |item| {
        item.set_max_durability(40);
        item.set_durability(0);
    });
    assert!(!session.apply_inventory_item_store_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_CHEST,
        guid
    ));
    assert!(
        session
            .resolved_inventory_item_object_like_cpp(guid)
            .unwrap()
            .has_item_flag2(ItemFieldFlags2::EQUIPPED)
    );
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 718,
            spell_entry_id: 38,
            spell_id: 9038,
            threshold: 1,
            apply: true
        },]
    );
    assert!(!session.apply_inventory_item_remove_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_CHEST,
        guid,
        &[]
    ));
    assert!(
        session
            .represented_item_mod_reapply_events_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session
            .represented_item_set_spell_events_like_cpp()
            .last()
            .unwrap()
            .apply,
        false
    );
}

#[test]
fn inventory_swap_effects_stale_owner_does_not_mutate_fixture_or_replacement() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 30_183);
    let item_guid = ObjectGuid::create_item(1, 30_183);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "StaleSwapEffects".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("world map");
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        item_guid,
        30_183,
        InventoryType::Weapon,
    );
    session.update_inventory_item_object_like_cpp(item_guid, |item| {
        item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
        item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 905, 12_000, 0);
    });
    assert!(
        session
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some()
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    let _ = drain_server_opcodes(&send_rx);
    assert!(!session.apply_inventory_item_remove_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        item_guid,
        &[EnchantmentSlot::EnhancementTemporary]
    ));
    assert!(!session.apply_inventory_item_store_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        item_guid
    ));
    assert!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .has_item_flag2(ItemFieldFlags2::EQUIPPED)
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .enchantments[EnchantmentSlot::EnhancementTemporary as usize]
            .id,
        905
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .is_empty(),
                player.item_durations().is_empty(),
                player.enchant_durations().is_empty(),
            )),
        Some((true, true, true))
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn represented_item_level_area_scaling_raw_fallback_requires_publication_and_targets() {
    let (mut session, _, send_rx) = make_session();
    let item_id = 30_166;
    let item_guid = ObjectGuid::create_item(1, 30_166);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 30_166)));
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 10);
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        item_id,
        InventoryType::Chest,
    );
    assert!(session.catalogs.player_stats.is_none());
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(489, wow_data::map::MAP_BATTLEGROUND, 0),
        represented_item_level_area_map_like_cpp(571, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(489, Position::ZERO);
    let _ = drain_server_opcodes(&send_rx);
    assert_eq!(
        session.update_represented_item_level_area_based_scaling_with_publication_like_cpp(true),
        Some(true),
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );

    session.set_player_map_position_like_cpp(571, Position::ZERO);
    let _ = drain_server_opcodes(&send_rx);
    assert_eq!(
        session.update_represented_item_level_area_based_scaling_with_publication_like_cpp(false),
        Some(true),
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());

    let (mut empty_session, _, empty_rx) = make_session();
    empty_session.set_player_guid(Some(ObjectGuid::create_player(1, 30_167)));
    install_represented_pvp_item_level_fixture_like_cpp(&mut empty_session, item_id, 10);
    assert!(empty_session.catalogs.player_stats.is_none());
    empty_session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(489, wow_data::map::MAP_BATTLEGROUND, 0),
    ])));
    empty_session.set_player_map_position_like_cpp(489, Position::ZERO);
    let _ = drain_server_opcodes(&empty_rx);
    assert_eq!(
        empty_session
            .update_represented_item_level_area_based_scaling_with_publication_like_cpp(true),
        Some(true),
    );
    assert!(
        empty_session
            .represented_item_mod_reapply_events_like_cpp()
            .is_empty()
    );
    assert!(drain_server_opcodes(&empty_rx).is_empty());
}

#[test]
fn represented_item_level_area_scaling_rejects_stale_owner_without_mutation_or_publication() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 30_164);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "StaleScalingOwner".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("world map");
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(
            571,
            wow_data::map::MAP_COMMON,
            wow_data::map::MAP_FLAG2_ACTIVATES_PVP_ITEM_LEVELS_LIKE_CPP,
        ),
    ])));
    let _ = drain_server_opcodes(&send_rx);
    let before = canonical
        .lock()
        .unwrap()
        .with_player_like_cpp(replacement_handle, |player| {
            (
                player.gameplay_state().using_pvp_item_levels,
                player.unit().data().health,
                player.unit().data().max_health,
            )
        })
        .expect("replacement state");

    assert_eq!(
        session.update_represented_item_level_area_based_scaling_with_publication_like_cpp(true),
        None,
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                (
                    player.gameplay_state().using_pvp_item_levels,
                    player.unit().data().health,
                    player.unit().data().max_health,
                )
            }),
        Some(before)
    );
    assert!(
        session
            .represented_item_mod_reapply_events_like_cpp()
            .is_empty()
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn represented_item_level_area_scaling_without_publication_still_reapplies_and_restores_health() {
    let (mut session, _, send_rx) = make_session();
    let item_id = 30_163;
    let item_guid = ObjectGuid::create_item(1, 30_163);
    let guid = ObjectGuid::create_player(1, 30_163);
    let registry = Arc::new(PlayerRegistry::default());
    bind_canonical_test_player_to_registry_like_cpp(
        &mut session,
        &registry,
        guid,
        Position::ZERO,
        571,
    );
    session.set_player_guid(Some(guid));
    session.set_loaded_player_name_like_cpp("ScalingRegistryOwner".to_string());
    session.set_player_registry(Arc::clone(&registry));
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 10);
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        item_id,
        InventoryType::Chest,
    );
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(571, wow_data::map::MAP_BATTLEGROUND, 0),
    ])));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_health_like_cpp(1, 3);
    session.register_in_player_registry();
    assert!(
        !registry
            .vehicle_interaction_snapshot(guid)
            .unwrap()
            .has_vehicle_kit
    );
    session.fixtures.vehicles.player_mount_vehicle_kit_like_cpp = Some(
        represented_vehicle_kit_with_passenger_like_cpp(guid, test_creature_guid(30_165), true),
    );
    let _ = drain_server_opcodes(&send_rx);

    assert_eq!(
        session.update_represented_item_level_area_based_scaling_with_publication_like_cpp(false),
        Some(true),
    );
    assert!(session.represented_using_pvp_item_levels_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 1);
    assert!(
        registry
            .vehicle_interaction_snapshot(guid)
            .unwrap()
            .has_vehicle_kit
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[
            RepresentedItemModsReapplyEventLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                apply: false
            },
            RepresentedItemModsReapplyEventLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                apply: true
            },
        ]
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
    assert_eq!(
        session.update_represented_item_level_area_based_scaling_with_publication_like_cpp(true),
        Some(false),
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp().len(),
        2
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn represented_item_level_area_scaling_activates_on_map_flag_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_155_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 15);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(
            30_155,
            wow_data::map::MAP_COMMON,
            wow_data::map::MAP_FLAG2_ACTIVATES_PVP_ITEM_LEVELS_LIKE_CPP,
        ),
    ])));
    session.set_player_map_position_like_cpp(30_155, Position::ZERO);

    assert!(session.update_represented_item_level_area_based_scaling_like_cpp());
    assert!(session.represented_using_pvp_item_levels_like_cpp());
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(115)
    );
}
#[test]
fn represented_item_level_area_scaling_deactivates_without_activity_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_157_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 20);
    let _ = session.set_represented_using_pvp_item_levels_like_cpp(true);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(30_157, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(30_157, Position::ZERO);

    assert!(session.update_represented_item_level_area_based_scaling_like_cpp());
    assert!(!session.represented_using_pvp_item_levels_like_cpp());
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(100)
    );
}
#[test]
fn represented_item_level_area_scaling_reapplies_top_level_item_mods_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 30_160);
    let item_id = 30_160_u32;
    let item_guid = ObjectGuid::create_item(1, 30_160);
    session.set_player_guid(Some(player_guid));
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 10);
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        item_id,
        InventoryType::Chest,
    );
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(489, wow_data::map::MAP_BATTLEGROUND, 0),
    ])));
    session.set_player_map_position_like_cpp(489, Position::ZERO);
    session.set_player_health_like_cpp(35, 80);

    assert!(session.update_represented_item_level_area_based_scaling_like_cpp());

    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[
            RepresentedItemModsReapplyEventLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                apply: false,
            },
            RepresentedItemModsReapplyEventLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                apply: true,
            },
        ],
        "C++ Player::UpdateItemLevelAreaBasedScaling wraps ActivatePvpItemLevels with _RemoveAllItemMods/_ApplyAllItemMods"
    );
    assert_eq!(
        session.player_health_like_cpp(),
        35,
        "C++ restores health with CalculatePct(GetMaxHealth(), previous GetHealthPct()) after item mods are reapplied"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject],
        "C++ item-mod setters mark update fields; this represented path emits the current-session VALUES delta after the reapply sequence"
    );
}
#[test]
fn represented_item_level_area_scaling_noop_does_not_reapply_item_mods_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_161_u32;
    let item_guid = ObjectGuid::create_item(1, 30_161);
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        item_id,
        InventoryType::Chest,
    );
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(30_161, wow_data::map::MAP_COMMON, 0),
    ])));
    session.set_player_map_position_like_cpp(30_161, Position::ZERO);

    assert!(!session.update_represented_item_level_area_based_scaling_like_cpp());
    assert!(
        session
            .represented_item_mod_reapply_events_like_cpp()
            .is_empty(),
        "C++ only removes/reapplies item mods when _usePvpItemLevels changes"
    );
}
#[test]
fn represented_item_level_area_scaling_skips_broken_item_mods_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_162_u32;
    let item_guid = ObjectGuid::create_item(1, 30_162);
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        item_id,
        InventoryType::Chest,
    );
    session.update_inventory_item_object_like_cpp(item_guid, |item| {
        item.set_max_durability(40);
        item.set_durability(0);
    });
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(489, wow_data::map::MAP_BATTLEGROUND, 0),
    ])));
    session.set_player_map_position_like_cpp(489, Position::ZERO);

    assert!(session.update_represented_item_level_area_based_scaling_like_cpp());
    assert!(
        session
            .represented_item_mod_reapply_events_like_cpp()
            .is_empty(),
        "C++ _RemoveAllItemMods/_ApplyAllItemMods skip broken items for represented mod removal/reapplication"
    );
}
#[test]
fn represented_item_level_applies_pvp_item_level_bonus_when_active_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_054_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 25);

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(100),
        "C++ only adds GetPvpItemLevelBonus when Player::IsUsingPvpItemLevels is true"
    );

    let _ = session.set_represented_using_pvp_item_levels_like_cpp(true);
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(125),
        "C++ Item::GetItemLevel(owner) adds DB2Manager::GetPvpItemLevelBonus after BonusData::ItemLevelBonus"
    );
}
#[test]
fn represented_item_level_applies_max_cap_after_pvp_bonus_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_055_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_pvp_item_store(Arc::new(PvpItemStore::from_entries([PvpItemEntry {
        id: 1,
        item_id: item_id as i32,
        item_level_delta: 75,
    }])));
    let _ = session.set_represented_using_pvp_item_levels_like_cpp(true);
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        max_item_level: 150,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(150),
        "C++ applies MaxItemLevel after adding the PvP item-level bonus"
    );
}
#[test]
fn represented_item_level_min_cutoff_uses_pre_pvp_item_level_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_056_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_pvp_item_store(Arc::new(PvpItemStore::from_entries([PvpItemEntry {
        id: 1,
        item_id: item_id as i32,
        item_level_delta: 50,
    }])));
    let _ = session.set_represented_using_pvp_item_levels_like_cpp(true);
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        min_item_level_cutoff: 120,
        min_item_level: 200,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(150),
        "C++ stores itemLevelBeforeUpgrades before PvP bonus and checks MinItemLevelCutoff against that value"
    );
}
#[test]
fn represented_item_level_does_not_apply_caps_to_non_equip_items_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_053_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::NonEquip, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::NonEquip as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        min_item_level: 150,
        max_item_level: 80,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(100),
        "C++ skips MinItemLevel/MaxItemLevel caps for INVTYPE_NON_EQUIP"
    );
}
#[test]
fn represented_condition_total_avg_item_level_skips_can_use_rejected_candidates_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_031_u32;
    let rejected_chest_item_id = 30_032_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_031);
    let rejected_chest_guid = ObjectGuid::create_item(1, 30_032);
    let player_guid = ObjectGuid::create_player(1, 171);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelCanUseRejected".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            equipped_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
        represented_test_item_record_like_cpp(
            rejected_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    equipped_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
                (
                    rejected_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
            ],
            [
                (
                    equipped_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
                (
                    rejected_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 300,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
            ],
        ),
    ));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: equipped_chest_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 100,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: rejected_chest_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 1 << 1,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 300,
            flags: [0; 4],
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        equipped_chest_guid,
        equipped_chest_item_id,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        rejected_chest_guid,
        rejected_chest_item_id,
        InventoryType::Chest,
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 6.25,
        "C++ UpdateAverageItemLevelTotal calls CanEquipItem for non-equipped candidates, and CanEquipItem rejects CanUseItem class/race failures before replacing the best slot"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped still uses only equipped items"
    );
}
#[test]
fn represented_condition_total_avg_item_level_skips_unique_limit_candidates_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_033_u32;
    let rejected_chest_item_id = 30_034_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_033);
    let rejected_chest_guid = ObjectGuid::create_item(1, 30_034);
    let limit_category_id = 44_u32;
    let player_guid = ObjectGuid::create_player(1, 172);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelUniqueLimitRejected".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            equipped_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
        represented_test_item_record_like_cpp(
            rejected_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
    ])));

    let mut equipped_sparse = sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0);
    equipped_sparse.limit_category = limit_category_id as u16;
    let mut rejected_sparse = sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0);
    rejected_sparse.limit_category = limit_category_id as u16;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (equipped_chest_item_id, equipped_sparse),
                (rejected_chest_item_id, rejected_sparse),
            ],
            [
                (
                    equipped_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
                (
                    rejected_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 300,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
            ],
        ),
    ));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: equipped_chest_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 100,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: rejected_chest_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 300,
            flags: [0; 4],
        },
    ])));
    session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
        ItemLimitCategoryEntry {
            id: limit_category_id,
            name: String::new(),
            quantity: 1,
            flags: wow_entities::ITEM_LIMIT_CATEGORY_MODE_EQUIP,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        equipped_chest_guid,
        equipped_chest_item_id,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        rejected_chest_guid,
        rejected_chest_item_id,
        InventoryType::Chest,
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 6.25,
        "C++ CanEquipItem calls CanEquipUniqueItem for non-equipped candidates, so an equip-limit category candidate cannot replace an already equipped item in the same limited category"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped still uses only equipped items"
    );
}
#[test]
fn represented_condition_total_avg_item_level_skips_socketed_gem_limit_candidates_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let equipped_chest_item_id = 30_035_u32;
    let rejected_chest_item_id = 30_036_u32;
    let equipped_gem_item_id = 30_037_u32;
    let candidate_gem_item_id = 30_038_u32;
    let equipped_chest_guid = ObjectGuid::create_item(1, 30_035);
    let rejected_chest_guid = ObjectGuid::create_item(1, 30_036);
    let gem_limit_category_id = 45_u32;
    let player_guid = ObjectGuid::create_player(1, 173);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelSocketedGemLimitRejected".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            equipped_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
        represented_test_item_record_like_cpp(
            rejected_chest_item_id,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
        represented_test_item_record_like_cpp(
            equipped_gem_item_id,
            InventoryType::NonEquip,
            ItemClass::Gem,
            0,
        ),
        represented_test_item_record_like_cpp(
            candidate_gem_item_id,
            InventoryType::NonEquip,
            ItemClass::Gem,
            0,
        ),
    ])));

    let mut equipped_gem_sparse =
        sparse_template_for_inventory_type_like_cpp(InventoryType::NonEquip, 0);
    equipped_gem_sparse.limit_category = gem_limit_category_id as u16;
    let mut candidate_gem_sparse =
        sparse_template_for_inventory_type_like_cpp(InventoryType::NonEquip, 0);
    candidate_gem_sparse.limit_category = gem_limit_category_id as u16;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    equipped_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
                (
                    rejected_chest_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
                (equipped_gem_item_id, equipped_gem_sparse),
                (candidate_gem_item_id, candidate_gem_sparse),
            ],
            [
                (
                    equipped_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
                (
                    rejected_chest_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 300,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
            ],
        ),
    ));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: equipped_chest_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 100,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: rejected_chest_item_id,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Epic as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 300,
            flags: [0; 4],
        },
    ])));
    session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
        ItemLimitCategoryEntry {
            id: gem_limit_category_id,
            name: String::new(),
            quantity: 1,
            flags: wow_entities::ITEM_LIMIT_CATEGORY_MODE_EQUIP,
        },
    ])));
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut equipped_chest = session.make_inventory_item_object(
        equipped_chest_guid,
        equipped_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    equipped_chest.set_gems(vec![wow_entities::SocketedGem {
        item_id: equipped_gem_item_id as i32,
        context: 0,
        bonus_list_ids: Vec::new(),
    }]);
    session.insert_inventory_item_object(equipped_chest);
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: equipped_chest_guid,
            entry_id: equipped_chest_item_id,
            db_guid: equipped_chest_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );

    let mut candidate_chest = session.make_inventory_item_object(
        rejected_chest_guid,
        rejected_chest_item_id,
        owner,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    candidate_chest.set_gems(vec![wow_entities::SocketedGem {
        item_id: candidate_gem_item_id as i32,
        context: 0,
        bonus_list_ids: Vec::new(),
    }]);
    session.insert_inventory_item_object(candidate_chest);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: rejected_chest_guid,
            entry_id: rejected_chest_item_id,
            db_guid: rejected_chest_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );

    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");

    assert_eq!(
        context.avg_item_level, 6.25,
        "C++ CanEquipUniqueItem checks socketed gems after the item template, so a candidate with a gem from an already-equipped limited category cannot replace the best slot"
    );
    assert_eq!(
        context.avg_equipped_item_level, 6.25,
        "C++ UpdateAverageItemLevelEquipped still uses only equipped items"
    );
}
