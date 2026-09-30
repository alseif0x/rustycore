use super::*;

#[test]
fn auto_unequip_offhand_records_titan_grip_penalty_check_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mainhand_item_id = 30_012_u32;
    let offhand_item_id = 30_013_u32;
    let mainhand_guid = ObjectGuid::create_item(1, 30_012);
    let offhand_guid = ObjectGuid::create_item(1, 30_013);
    let player_guid = ObjectGuid::create_player(1, 162);
    let penalty_spell_id = 49_152_u32;
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TitanGripPenaltyRemoveItem".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
        player.set_can_titan_grip(true, penalty_spell_id);
    });
    session
        .apply_aura(penalty_spell_id as i32, player_guid, 0, 0)
        .expect("represented Titan Grip penalty aura");
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (
                mainhand_item_id,
                InventoryType::Weapon2Hand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe2 as u8,
            ),
            (
                offhand_item_id,
                InventoryType::Weapon2Hand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe2 as u8,
            ),
        ],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        mainhand_guid,
        mainhand_item_id,
        InventoryType::Weapon2Hand,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::Weapon2Hand,
    );

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(true));

    assert_eq!(
        session.represented_titan_grip_penalty_actions_like_cpp(),
        &[TitanGripPenaltyAction::Remove(penalty_spell_id)],
        "C++ RemoveItem calls CheckTitanGripPenalty after clearing the offhand slot; once only the main-hand 2H remains, the Titan Grip penalty aura is removed"
    );
}
#[test]
fn auto_unequip_offhand_records_average_equipped_item_level_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mainhand_item_id = 30_014_u32;
    let offhand_item_id = 30_015_u32;
    let mainhand_guid = ObjectGuid::create_item(1, 30_014);
    let offhand_guid = ObjectGuid::create_item(1, 30_015);
    let player_guid = ObjectGuid::create_player(1, 163);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AverageItemLevelRemoveItem".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(true);
        player.set_can_titan_grip(false, 0);
    });
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            mainhand_item_id,
            InventoryType::Weapon2Hand,
            ItemClass::Weapon,
            ItemSubClassWeapon::Axe2 as u8,
        ),
        represented_test_item_record_like_cpp(
            offhand_item_id,
            InventoryType::Weapon2Hand,
            ItemClass::Weapon,
            ItemSubClassWeapon::Axe2 as u8,
        ),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    mainhand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon2Hand, 0),
                ),
                (
                    offhand_item_id,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon2Hand, 0),
                ),
            ],
            [
                (
                    mainhand_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 200,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Weapon2Hand as i8,
                    },
                ),
                (
                    offhand_item_id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Weapon2Hand as i8,
                    },
                ),
            ],
        ),
    ));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_MAINHAND,
        mainhand_guid,
        mainhand_item_id,
        InventoryType::Weapon2Hand,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::Weapon2Hand,
    );

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(true));

    assert_eq!(
        session.represented_avg_equipped_item_level_updates_like_cpp(),
        &[25.0],
        "C++ UpdateAverageItemLevelEquipped divides equipped item-level sum by 16 and counts a remaining main-hand 2H twice without Titan Grip"
    );
    let owned = session
        .represented_player_condition_context_like_cpp()
        .expect("fixture canonical inventory owner");
    let context = owned
        .as_context(&session)
        .expect("test Player condition owner");
    assert_eq!(
        context.avg_equipped_item_level, 25.0,
        "C++ PlayerCondition uses PlayerData::AvgItemLevel[1], so the represented context must use the same equipped-average formula"
    );
}
