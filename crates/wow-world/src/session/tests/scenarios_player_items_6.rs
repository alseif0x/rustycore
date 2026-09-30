//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn battle_pet_cage_battle_pet_creates_cage_item_removes_and_deletes_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x184);
    let expected_item = RepresentedBattlePetCageItemLikeCpp {
        item_id: BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP,
        species_id: 11,
        breed_data: 44 | (3 << 24),
        level: 17,
        display_id: 33,
    };

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 0,
            flags: 0,
            power: 0,
            health: 100,
            max_health: 100,
            speed: 0,
            quality: 3,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_cage_battle_pet_represented_like_cpp(pet_guid, true, true),
        RepresentedBattlePetCageOutcomeLikeCpp::Caged(expected_item)
    );

    assert_eq!(
        session.represented_battle_pet_cage_items_like_cpp(),
        &[expected_item]
    );
    assert_eq!(
        session
            .represented_battle_pet_like_cpp(pet_guid)
            .expect("removed pet row remains represented")
            .save_info,
        RepresentedBattlePetSaveInfoLikeCpp::Removed
    );
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        None
    );

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetDeleted as u16
    );
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.remaining(), 0);
}
#[test]
fn is_toy_item_uses_toy_db2_item_id_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_toy_store(Arc::new(ToyStore::from_entries([ToyEntry {
        id: 9,
        source_text: "known".to_string(),
        item_id: 30_000,
        flags: 0,
        source_type_enum: 0,
    }])));

    assert!(session.is_toy_item_like_cpp(30_000));
    assert!(!session.is_toy_item_like_cpp(30_001));
}
#[test]
fn item_template_flags_use_item_sparse_flags_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [],
        [
            (100, [ItemFlags::IS_BOUND_TO_ACCOUNT.bits() as u32, 0, 0, 0]),
            (101, [0, 0, 0, 0]),
        ],
    )));

    assert!(
        session
            .item_template_flags(100)
            .is_some_and(|flags| flags.contains(ItemFlags::IS_BOUND_TO_ACCOUNT))
    );
    assert!(session.is_item_bound_account_wide(100));
    assert!(!session.is_item_bound_account_wide(101));
    assert_eq!(session.item_template_flags(102), None);
}
#[test]
fn item_durability_repair_cost_uses_cpp_db2_cost_and_quality() {
    let (mut session, _, _) = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            100,
            InventoryType::Shield,
            ItemClass::Armor,
            ItemSubClassArmor::Shield as u8,
        ),
        represented_test_item_record_like_cpp(
            101,
            InventoryType::Chest,
            ItemClass::Armor,
            4,
        ),
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_random_property_templates([
        (
            100,
            ItemRandomPropertyTemplateEntry {
                item_level: 57,
                quality: ItemQuality::Rare as i8,
                inventory_type: InventoryType::Shield as i8,
            },
        ),
        (
            101,
            ItemRandomPropertyTemplateEntry {
                item_level: 57,
                quality: ItemQuality::Rare as i8,
                inventory_type: InventoryType::Chest as i8,
            },
        ),
    ])));
    session.set_durability_costs_store(Arc::new(DurabilityCostsStore::from_entries([
        DurabilityCostsEntry {
            id: 57,
            weapon_sub_class_cost: [0; 21],
            armor_sub_class_cost: std::array::from_fn(|i| {
                if i == ItemSubClassArmor::Shield as usize {
                    13
                } else if i == 4 {
                    5
                } else {
                    0
                }
            }),
        },
    ])));
    session.set_durability_quality_store(Arc::new(DurabilityQualityStore::from_entries([
        DurabilityQualityEntry {
            id: (ItemQuality::Rare as u32 + 1) * 2,
            data: 1.25,
        },
    ])));

    assert_eq!(
        session.item_durability_repair_cost_like_cpp(100, 40, 50, 0.8, 2.0),
        260
    );
    assert_eq!(
        session.item_durability_repair_cost_like_cpp(101, 10, 13, 1.0, 1.0),
        19
    );
    assert_eq!(
        session.item_durability_repair_cost_like_cpp(100, 50, 50, 1.0, 1.0),
        0
    );
}
