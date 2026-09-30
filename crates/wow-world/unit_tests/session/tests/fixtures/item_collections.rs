//! Collection, toy, battle-pet, and open-item fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn grant_learned_weapon_proficiency_like_cpp(
    session: &mut WorldSession,
    subclass_mask: u32,
) {
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.add_weapon_proficiency_like_cpp(subclass_mask);
            })
            .is_some(),
        "the canonical Player must own the learned weapon proficiency"
    );
}

pub(in crate::session::tests) fn install_transmog_can_add_test_item(
    session: &mut WorldSession,
    item_id: u32,
    class_id: ItemClass,
    subclass_id: u8,
    inventory_type: InventoryType,
    quality: ItemQuality,
    flags: [u32; 4],
    allowable_class: i16,
) {
    install_transmog_can_add_test_items(
        session,
        [(
            item_id,
            class_id,
            subclass_id,
            inventory_type,
            quality,
            flags,
            allowable_class,
        )],
    );
}

pub(in crate::session::tests) fn install_transmog_can_add_test_items<const N: usize>(
    session: &mut WorldSession,
    items: [(
        u32,
        ItemClass,
        u8,
        InventoryType,
        ItemQuality,
        [u32; 4],
        i16,
    ); N],
) {
    session.set_item_store(Arc::new(ItemStore::from_records(
        items.iter().copied().map(
            |(item_id, class_id, subclass_id, inventory_type, _, _, _)| ItemRecord {
                id: item_id,
                class_id: class_id as u8,
                subclass_id,
                material: 0,
                inventory_type: inventory_type as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
                scaling_stat_distribution_id: 0,
                scaling_stat_value: 0,
            },
        ),
    )));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries(
        items
            .iter()
            .copied()
            .map(
                |(item_id, _, _, _, quality, flags, allowable_class)| ItemSearchNameEntry {
                    id: item_id,
                    allowable_race: 0,
                    display: String::new(),
                    overall_quality_id: quality as u8,
                    expansion_id: 0,
                    min_faction_id: 0,
                    min_reputation: 0,
                    allowable_class: i32::from(allowable_class),
                    required_level: 0,
                    required_skill: 0,
                    required_skill_rank: 0,
                    required_ability: 0,
                    item_level: 1,
                    flags: flags.map(|flag| flag as i32),
                },
            ),
    )));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            items.iter().copied().map(
                |(item_id, _, _, inventory_type, _, flags, allowable_class)| {
                    (
                        item_id,
                        ItemSparseTemplateEntry {
                            flags,
                            bag_family: 0,
                            start_quest_id: 0,
                            stackable: 1,
                            max_count: 0,
                            lock_id: 0,
                            required_reputation_rank: 0,
                            sell_price: 0,
                            buy_price: 0,
                            vendor_stack_count: 1,
                            price_variance: 0.0,
                            price_random_value: 0.0,
                            max_durability: 0,
                            other_faction_item_id: 0,
                            content_tuning_id: 0,
                            player_level_to_item_level_curve_id: 0,
                            limit_category: 0,
                            instance_bound: 0,
                            zone_bound: [0, 0],
                            required_reputation_faction: 0,
                            allowable_class,
                            required_expansion: 0,
                            bonding: ItemBondingType::None as u8,
                            container_slots: 0,
                            inventory_type: inventory_type as i8,
                        },
                    )
                },
            ),
            items
                .iter()
                .copied()
                .map(|(item_id, _, _, inventory_type, quality, _, _)| {
                    (
                        item_id,
                        ItemRandomPropertyTemplateEntry {
                            item_level: 1,
                            quality: quality as i8,
                            inventory_type: inventory_type as i8,
                        },
                    )
                }),
        ),
    ));
}

pub(in crate::session::tests) fn install_represented_battle_pet_stat_stores_like_cpp(
    session: &mut WorldSession,
) {
    session.set_battle_pet_breed_state_store(Arc::new(BattlePetBreedStateStore::from_entries([
        wow_data::BattlePetBreedStateEntry {
            id: 1,
            battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
            value: 500,
            battle_pet_breed_id: 7,
        },
        wow_data::BattlePetBreedStateEntry {
            id: 2,
            battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
            value: 300,
            battle_pet_breed_id: 7,
        },
        wow_data::BattlePetBreedStateEntry {
            id: 3,
            battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
            value: 200,
            battle_pet_breed_id: 7,
        },
    ])));
    session.set_battle_pet_species_state_store(Arc::new(BattlePetSpeciesStateStore::from_entries(
        [
            wow_data::BattlePetSpeciesStateEntry {
                id: 10,
                battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 100,
                battle_pet_species_id: 11,
            },
            wow_data::BattlePetSpeciesStateEntry {
                id: 11,
                battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
                value: 50,
                battle_pet_species_id: 11,
            },
            wow_data::BattlePetSpeciesStateEntry {
                id: 12,
                battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
                value: 25,
                battle_pet_species_id: 11,
            },
        ],
    )));
    session.set_battle_pet_breed_quality_store(Arc::new(BattlePetBreedQualityStore::from_entries(
        [wow_data::BattlePetBreedQualityEntry {
            id: 20,
            state_multiplier: 1.5,
            quality_enum: 3,
        }],
    )));
}

pub(in crate::session::tests) fn install_represented_battle_pet_species_flags_like_cpp(
    session: &mut WorldSession,
    species: u32,
    flags: i32,
) {
    install_represented_battle_pet_species_like_cpp(session, species, 0, flags);
}

pub(in crate::session::tests) fn install_represented_battle_pet_species_like_cpp(
    session: &mut WorldSession,
    species: u32,
    creature_id: i32,
    flags: i32,
) {
    session.set_battle_pet_species_store(Arc::new(wow_data::BattlePetSpeciesStore::from_entries(
        [wow_data::BattlePetSpeciesEntry {
            id: species,
            description: String::new(),
            source_text: String::new(),
            creature_id,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        }],
    )));
}

pub(in crate::session::tests) fn write_minimal_use_toy_packet_like_cpp(
    item_id: u32,
    spell_id: i32,
    cast_id: ObjectGuid,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::UseToy as u16);
    pkt.write_packed_guid(&cast_id);
    pkt.write_int32(i32::try_from(item_id).unwrap());
    pkt.write_int32(0);
    pkt.write_int32(spell_id);
    wow_packet::packets::spell::SpellCastVisual::default().write(&mut pkt);
    pkt.write_float(0.0);
    pkt.write_float(0.0);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_bits(0, 5);
    pkt.write_bit(false);
    pkt.write_bits(0, 2);
    pkt.write_bit(false);
    pkt.flush_bits();
    SpellTargetData::default().write(&mut pkt);
    pkt
}

pub(in crate::session::tests) fn instant_toy_spell_info_like_cpp(
    spell_id: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

pub(in crate::session::tests) fn insert_open_item_bag_with_child(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    bag_slot: u8,
    inner_slot: u8,
) -> (ObjectGuid, ObjectGuid) {
    let bag_guid = ObjectGuid::create_item(1, 1001);
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            bag_slot,
            InventoryItem {
                guid: bag_guid,
                entry_id: 101,
                db_guid: 1001,
                inventory_type: Some(InventoryType::Bag as u8),
            },
        );
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        101,
        player_guid,
        1,
        0,
        ItemContext::None,
        bag_slot,
    );
    session.insert_inventory_item_object(bag_item);

    let child_guid = ObjectGuid::create_item(1, 1002);
    let mut child = session.make_inventory_item_object(
        child_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        inner_slot,
    );
    child.set_container_guid_and_slot(bag_guid, bag_slot);
    session.insert_inventory_item_object(child);

    (bag_guid, child_guid)
}

pub(in crate::session::tests) fn install_open_item_has_loot_template(
    session: &mut WorldSession,
    entry: u32,
) {
    install_open_item_has_loot_template_with_lock(session, entry, 0);
}

pub(in crate::session::tests) fn install_open_item_template_with_flags(
    session: &mut WorldSession,
    entry: u32,
    flags: ItemFlags,
    lock_id: u16,
) {
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry,
        ItemSparseTemplateEntry {
            flags: [flags.bits() as u32, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 1,
            max_count: 0,
            lock_id,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

pub(in crate::session::tests) fn install_open_item_has_loot_template_with_lock(
    session: &mut WorldSession,
    entry: u32,
    lock_id: u16,
) {
    install_open_item_template_with_flags(session, entry, ItemFlags::HAS_LOOT, lock_id);
}

pub(in crate::session::tests) fn install_lock_store(session: &mut WorldSession, lock_id: u32) {
    session.set_lock_store(Arc::new(LockStore::from_entries([LockEntry {
        id: lock_id,
        index: [0; 8],
        skill: [0; 8],
        lock_type: [0; 8],
        action: [0; 8],
    }])));
}

pub(in crate::session::tests) fn insert_open_item_top_level(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    slot: u8,
    item_guid: ObjectGuid,
    entry: u32,
    unlocked: bool,
) {
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id: entry,
                db_guid: item_guid.counter() as u64,
                inventory_type: None,
            },
        );
    let mut item = session.make_inventory_item_object(
        item_guid,
        entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    if unlocked {
        item.set_item_flag(ItemFieldFlags::UNLOCKED);
    }
    session.insert_inventory_item_object(item);
}

pub(in crate::session::tests) async fn assert_open_item_nested_has_loot_opens_without_internal_bag_error(
    bag_slot: u8,
) {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    install_open_item_has_loot_template(&mut session, 700);
    let (_, child_guid) = insert_open_item_bag_with_child(&mut session, player_guid, bag_slot, 5);

    session
        .handle_open_item(WorldPacket::from_bytes(&[bag_slot, 5]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::LootResponse as u16);
    assert_ne!(opcode, ServerOpcodes::InventoryChangeFailure as u16);
    assert!(session.loot_table.contains_key(&child_guid));
    assert!(
        session
            .inventory_item_objects
            .get(&child_guid)
            .is_some_and(|item| item.loot_generated())
    );
}

pub(in crate::session::tests) fn assert_open_item_release_destroy_nested_item_leaves_container_in_place(
    bag_slot: u8,
) {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (bag_guid, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, bag_slot, 5);
    let child = session.inventory_item_objects.get(&child_guid).unwrap();
    let child_bag = child.bag_slot();
    let child_slot = child.slot();

    let inv = session.get_inventory_item_by_pos(child_bag, child_slot);
    assert!(inv.is_some());
    assert_eq!(inv.unwrap().guid, child_guid);

    session.remove_fully_looted_runtime_item(child_bag, child_slot, child_guid);
    assert!(
        session
            .get_inventory_item_by_pos(child_bag, child_slot)
            .is_none()
    );
    assert!(!session.inventory_item_objects.contains_key(&child_guid));
    assert!(
        session
            .player_item_test_fixture_like_cpp
            .inventory_items
            .contains_key(&bag_slot)
    );
    assert_eq!(
        session.player_item_test_fixture_like_cpp.inventory_items[&bag_slot].guid,
        bag_guid
    );
}
