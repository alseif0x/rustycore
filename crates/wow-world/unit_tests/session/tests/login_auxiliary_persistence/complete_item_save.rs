// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
// C++ anchors: Item.cpp:383-437, 585-725; Player.cpp:18411-18470.

//! Production login hydration keeps canonical Item fields available to full-save capture.

use super::*;
use std::sync::Arc;
use wow_constants::{InventoryType, ItemContext, ItemFieldFlags, ItemUpdateState};
use wow_persistence::{
    PersistenceOutcomeLikeCpp, PlayerBagInventoryLoadRowLikeCpp,
    PlayerEquipmentInventoryLoadRowLikeCpp,
    PlayerInventoryItemLoadPayload, PlayerInventoryItemLoadRowLikeCpp, PlayerItemGemSaveLikeCpp,
    PlayerItemSaveStateLikeCpp, PlayerLoginAuxiliaryLoadOutcomeLikeCpp,
    PlayerLoginAuxiliaryLoadedLikeCpp, PlayerLoginItemRepairActionLikeCpp,
    PlayerLoginItemRepairRequestLikeCpp, PlayerLifecyclePortLikeCpp,
};

fn payload(
    creator_guid: u64,
    gift_creator_guid: u64,
    text: Option<&str>,
) -> PlayerInventoryItemLoadPayload {
    PlayerInventoryItemLoadPayload {
        creator_guid,
        gift_creator_guid,
        text: text.map(str::to_owned),
        battle_pet_species_id: 0,
        battle_pet_breed_data: 0,
        battle_pet_level: 0,
        battle_pet_display_id: 0,
        transmog_appearance: [0; 6],
        transmog_enchant_illusion: [0; 6],
        transmog_secondary_appearance: [0; 6],
        soulbound_trade_allowed_players: None,
    }
}

fn row(item_entry: u32, item_db_guid: u64, count: u32) -> PlayerInventoryItemLoadRowLikeCpp {
    PlayerInventoryItemLoadRowLikeCpp {
        item_entry,
        item_db_guid,
        count,
        durability: 40,
        context: ItemContext::None as u8,
        flags: 0,
        played_time: 0,
        enchantments: String::new(),
        random_properties_id: 0,
        random_properties_seed: 0,
        gems: std::array::from_fn(|_| (0, String::new(), 0)),
        paid_money: None,
        paid_extended_cost: None,
        expiration: 0,
        spell_charges: String::new(),
        complete_payload: Some(payload(0, 0, None)),
    }
}

fn item_flags(flags: ItemFieldFlags) -> u32 {
    flags.bits()
}

fn install_catalogs(
    session: &mut crate::session::WorldSession,
    entries: &[(u32, i8, u8, i32, u32)],
    item_effects_known: bool,
) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records(
        entries.iter().map(|&(id, inventory_type, _, _, _)| wow_data::ItemRecord {
            id,
            class_id: if inventory_type == InventoryType::Bag as i8 {
                1
            } else {
                4
            },
            subclass_id: 0,
            material: 0,
            inventory_type,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        }),
    )));
    session.set_item_stats_store(Arc::new(wow_data::ItemStatsStore::from_sparse_templates(
        entries.iter().map(|&(id, inventory_type, slots, stackable, durability)| {
            (
                id,
                wow_data::ItemSparseTemplateEntry {
                    flags: [0; 4],
                    bag_family: 0,
                    start_quest_id: 0,
                    stackable,
                    max_count: 0,
                    lock_id: 0,
                    required_reputation_rank: 0,
                    sell_price: 0,
                    buy_price: 0,
                    vendor_stack_count: 1,
                    price_variance: 1.0,
                    price_random_value: 0.0,
                    max_durability: durability,
                    other_faction_item_id: 0,
                    content_tuning_id: 0,
                    player_level_to_item_level_curve_id: 0,
                    limit_category: 0,
                    instance_bound: 0,
                    zone_bound: [0; 2],
                    required_reputation_faction: 0,
                    allowable_class: -1,
                    required_expansion: 0,
                    bonding: 0,
                    container_slots: slots,
                    inventory_type,
                },
            )
        }),
    )));
    if item_effects_known {
        session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries(
            std::iter::empty::<wow_data::ItemEffectEntry>(),
        )));
    }
}

fn enchantments_string(id: i32, duration: u32, charges: i16) -> String {
    (0..wow_entities::MAX_ENCHANTMENT_SLOT)
        .map(|slot| {
            if slot == 0 {
                format!("{id} {duration} {charges} ")
            } else {
                "0 0 0 ".to_owned()
            }
        })
        .collect()
}

fn enchantment_entry(id: u32) -> wow_data::SpellItemEnchantmentEntry {
    wow_data::SpellItemEnchantmentEntry {
        id,
        effect_arg: [0; 3],
        effect_points_min: [0; 3],
        effect_scaling_points: [0.0; 3],
        item_visual: 0,
        flags: wow_constants::SpellItemEnchantmentFlags::empty(),
        required_skill_id: 0,
        required_skill_rank: 0,
        item_level: 1,
        charges: 0,
        effect: [0; 3],
        condition_id: 0,
        min_level: 1,
        max_level: 0,
    }
}

async fn load_rows(
    session: &mut crate::session::WorldSession,
    owner: wow_core::ObjectGuid,
    equipment: Vec<PlayerEquipmentInventoryLoadRowLikeCpp>,
    bags: Vec<PlayerBagInventoryLoadRowLikeCpp>,
    repair_outcome: PersistenceOutcomeLikeCpp,
) -> (
    wow_world_character::LoginInventorySnapshotLikeCpp,
    Arc<AuxiliaryLoadPortLikeCpp>,
) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(16))
        .expect("canonical login owner provides backpack capacity input");
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentInventory(equipment),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::BagInventory(bags),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentSets(vec![]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::TransmogOutfits(vec![]),
        ),
    ]);
    port.set_repair_outcome(repair_outcome);
    let lifecycle_port: Arc<dyn PlayerLifecyclePortLikeCpp> = port.clone();
    let snapshot = wow_world_character::load_inventory_for_login_like_cpp(
        session,
        &lifecycle_port,
        owner,
        1,
    )
    .await
    .expect("the production login snapshot remains available");
    (snapshot, port)
}

fn equipment(
    slot: u8,
    item: PlayerInventoryItemLoadRowLikeCpp,
) -> PlayerEquipmentInventoryLoadRowLikeCpp {
    PlayerEquipmentInventoryLoadRowLikeCpp { slot, item }
}

fn bag(
    bag_slot: u8,
    inner_slot: u8,
    item: PlayerInventoryItemLoadRowLikeCpp,
) -> PlayerBagInventoryLoadRowLikeCpp {
    PlayerBagInventoryLoadRowLikeCpp {
        bag_slot,
        inner_slot,
        item,
    }
}

#[tokio::test]
async fn canonical_login_items_roundtrip_full_payload_through_full_save_ack() {
    let (mut session, _, _) = make_session();
    let owner = install_canonical_player_owner_for_test(&mut session, 571, 0);
    install_catalogs(
        &mut session,
        &[
            (41_001, InventoryType::NonEquip as i8, 0, 20, 40),
            (41_002, InventoryType::Bag as i8, 4, 1, 40),
            (41_003, InventoryType::NonEquip as i8, 0, 20, 40),
        ],
        true,
    );
    session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries([
        wow_data::ItemEffectEntry {
            id: 1,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: 1,
            cooldown_msec: 0,
            category_cooldown_msec: 0,
            spell_category_id: 0,
            spell_id: 0,
            chr_specialization_id: 0,
            parent_item_id: 41_003,
        },
    ])));
    session.set_spell_item_enchantment_store(Arc::new(
        wow_data::SpellItemEnchantmentStore::from_entries([enchantment_entry(990)]),
    ));

    let mut top_payload = payload(60_001, 60_002, Some("backpack inscription"));
    top_payload.battle_pet_species_id = 11;
    top_payload.battle_pet_breed_data = 22;
    top_payload.battle_pet_level = 33;
    top_payload.battle_pet_display_id = 44;
    top_payload.transmog_appearance = [11, 12, 13, 14, 15, 16];
    top_payload.transmog_enchant_illusion = [21, 22, 23, 24, 25, 26];
    top_payload.transmog_secondary_appearance = [31, 32, 33, 34, 35, 36];
    let mut top = row(41_001, 61_001, 3);
    top.durability = 31;
    top.context = ItemContext::Vendor as u8;
    top.played_time = 44;
    top.complete_payload = Some(top_payload);

    let mut bag_item = row(41_002, 61_002, 1);
    bag_item.complete_payload = Some(payload(0, 0, None));
    let mut child_payload = payload(60_003, 60_004, Some("bag inscription"));
    child_payload.battle_pet_species_id = 51;
    child_payload.battle_pet_breed_data = 52;
    child_payload.battle_pet_level = 53;
    child_payload.battle_pet_display_id = 54;
    child_payload.transmog_appearance = [101, 102, 103, 104, 105, 106];
    child_payload.transmog_enchant_illusion = [201, 202, 203, 204, 205, 206];
    child_payload.transmog_secondary_appearance = [301, 302, 303, 304, 305, 306];
    let mut child = row(41_003, 61_003, 2);
    child.durability = 27;
    child.context = ItemContext::Vendor as u8;
    child.played_time = 55;
    child.flags = item_flags(ItemFieldFlags::CHILD);
    child.enchantments = enchantments_string(990, 3_600, 2);
    child.spell_charges = "7".to_owned();
    child.gems[0] = (4_050, "710 711 ".to_owned(), 3);
    child.complete_payload = Some(child_payload);

    let (snapshot, _) = load_rows(
        &mut session,
        owner,
        vec![
            equipment(35, top),
            equipment(wow_entities::INVENTORY_SLOT_BAG_START, bag_item),
        ],
        vec![bag(wow_entities::INVENTORY_SLOT_BAG_START, 2, child)],
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    )
    .await;

    let top_guid = wow_core::ObjectGuid::create_item(1, 61_001);
    let bag_guid = wow_core::ObjectGuid::create_item(1, 61_002);
    let child_guid = wow_core::ObjectGuid::create_item(1, 61_003);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            assert!(player
                .inventory_runtime_like_cpp()
                .equipment_inventory_authority_complete_like_cpp());
            assert_eq!(
                player.get_item_by_pos(wow_entities::INVENTORY_SLOT_BAG_0, 35),
                Some(top_guid)
            );
            assert_eq!(
                player.get_item_by_pos(wow_entities::INVENTORY_SLOT_BAG_START, 2),
                Some(child_guid)
            );
            let items = player.inventory_runtime_like_cpp().item_objects();
            assert!(items[&bag_guid].text().is_empty(), "a proven SQL NULL text hydrates as empty");
            assert_eq!(items[&top_guid].data().creator, wow_core::ObjectGuid::create_player(1, 60_001));
            assert_eq!(items[&child_guid].data().creator, wow_core::ObjectGuid::create_item(1, 60_003));
            assert_eq!(items[&top_guid].data().gift_creator, wow_core::ObjectGuid::create_player(1, 60_002));
            assert_eq!(items[&child_guid].data().gift_creator, wow_core::ObjectGuid::create_player(1, 60_004));
        })
        .unwrap();
    let debit_plan = session
        .core
        .with_owned_player_like_cpp(|player| {
            player.plan_spell_cast_resource_debits_like_cpp(&[(41_001, 1), (41_003, 1)], &[])
        })
        .unwrap()
        .expect("complete production login hydration authorizes the canonical debit plan");
    assert!(session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.apply_spell_cast_resource_debits_like_cpp(&debit_plan)
        })
        .unwrap()
        .is_some());

    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    crate::player_inventory_test_fixture::complete_known_empty_player_currencies_like_cpp(
        &mut session,
    );
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    assert_eq!(
        session.save_current_player_to_db_like_cpp().await,
        crate::session::PlayerSaveOutcomeLikeCpp::Applied
    );

    let requests = save_port.character_save_requests();
    let saved = requests[0]
        .inventory_items
        .as_ref()
        .expect("complete login hydration enables the existing Item save capture");
    assert_eq!(saved.len(), 2);
    let top = saved.iter().find(|item| item.item_guid == 61_001).unwrap();
    assert_eq!(top.state, PlayerItemSaveStateLikeCpp::Changed);
    assert_eq!((top.item_entry, top.owner_guid, top.count, top.durability), (41_001, owner.counter() as u64, 2, 31));
    assert_eq!((top.creator_guid, top.gift_creator_guid), (60_001, 60_002));
    assert_eq!(top.text, "backpack inscription");
    assert_eq!((top.battle_pet_species_id, top.battle_pet_breed_data, top.battle_pet_level, top.battle_pet_display_id), (11, 22, 33, 44));
    assert_eq!(top.transmog, Some([
        11, 12, 13, 14, 15, 16, 21, 22, 23, 24, 25, 26, 31, 32, 33, 34, 35, 36,
    ]));
    assert_eq!((top.bag_guid, top.slot, top.context, top.played_time), (0, 35, ItemContext::Vendor as u8, 44));
    assert_eq!((top.expiration, top.random_properties_id, top.property_seed, top.dynamic_flags, top.wrapped), (0, 0, 0, 0, false));

    let child = saved.iter().find(|item| item.item_guid == 61_003).unwrap();
    assert_eq!(child.state, PlayerItemSaveStateLikeCpp::Changed);
    assert_eq!((child.item_entry, child.owner_guid, child.count, child.durability), (41_003, owner.counter() as u64, 1, 27));
    assert_eq!((child.creator_guid, child.gift_creator_guid), (60_003, 60_004));
    assert_eq!(child.text, "bag inscription");
    assert_eq!((child.battle_pet_species_id, child.battle_pet_breed_data, child.battle_pet_level, child.battle_pet_display_id), (51, 52, 53, 54));
    assert_eq!(child.transmog, Some([
        101, 102, 103, 104, 105, 106, 201, 202, 203, 204, 205, 206, 301, 302, 303, 304, 305, 306,
    ]));
    assert_eq!((child.bag_guid, child.slot, child.context, child.played_time), (bag_guid.counter() as u64, 2, ItemContext::Vendor as u8, 55));
    assert_eq!(child.enchantments, enchantments_string(990, 3_600, 2));
    assert_eq!(child.dynamic_flags, item_flags(ItemFieldFlags::CHILD));
    assert_eq!(child.charges, "7 ");
    assert_eq!(
        child.gems,
        Some([
            PlayerItemGemSaveLikeCpp { item_id: 4_050, bonus_list_ids: "710 711 ".to_owned(), context: 3 },
            PlayerItemGemSaveLikeCpp { item_id: 0, bonus_list_ids: String::new(), context: 0 },
            PlayerItemGemSaveLikeCpp { item_id: 0, bonus_list_ids: String::new(), context: 0 },
        ])
    );
    assert!(snapshot.item_creates.iter().any(|create| create.item_guid == child_guid && create.dynamic_flags == item_flags(ItemFieldFlags::CHILD)));
    session
        .core
        .with_owned_player_like_cpp(|player| {
            assert_eq!(player.inventory_runtime_like_cpp().item_objects()[&top_guid].update_state(), ItemUpdateState::Unchanged);
            assert_eq!(player.inventory_runtime_like_cpp().item_objects()[&child_guid].update_state(), ItemUpdateState::Unchanged);
            assert_eq!(player.get_item_by_pos(wow_entities::INVENTORY_SLOT_BAG_START, 2), Some(child_guid));
            assert_eq!(player.get_item_by_pos(wow_entities::INVENTORY_SLOT_BAG_0, wow_entities::INVENTORY_SLOT_BAG_START), Some(bag_guid));
        })
        .unwrap();
}

#[tokio::test]
async fn loaded_ankh_debit_uses_full_save_dto_and_only_applied_ack_clears_dirty_state() {
    for (initial_count, outcome) in [
        (10, PersistenceOutcomeLikeCpp::Applied { rows: 1 }),
        (1, PersistenceOutcomeLikeCpp::Applied { rows: 1 }),
        (10, PersistenceOutcomeLikeCpp::Failed { reason: "controlled failure".into() }),
        (1, PersistenceOutcomeLikeCpp::Failed { reason: "controlled failure".into() }),
        (10, PersistenceOutcomeLikeCpp::Unknown { reason: "controlled unknown commit".into() }),
        (1, PersistenceOutcomeLikeCpp::Unknown { reason: "controlled unknown commit".into() }),
    ] {
        let (mut session, _, _) = make_session();
        let owner = install_canonical_player_owner_for_test(&mut session, 571, 0);
        install_catalogs(
            &mut session,
            &[(17_030, InventoryType::NonEquip as i8, 0, 20, 0)],
            true,
        );
        let (snapshot, _) = load_rows(
            &mut session,
            owner,
            vec![equipment(35, row(17_030, 70_001, initial_count))],
            vec![],
            PersistenceOutcomeLikeCpp::Applied { rows: 0 },
        )
        .await;
        let item_guid = wow_core::ObjectGuid::create_item(1, 70_001);
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                assert!(player
                    .inventory_runtime_like_cpp()
                    .equipment_inventory_authority_complete_like_cpp());
                assert_eq!(
                    player.get_item_by_pos(wow_entities::INVENTORY_SLOT_BAG_0, 35),
                    Some(item_guid)
                );
            })
            .unwrap();
        assert!(snapshot.item_creates.iter().any(|item| item.item_guid == item_guid));
        crate::player_inventory_test_fixture::complete_known_empty_player_currencies_like_cpp(
            &mut session,
        );
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_player_aura_authority_complete_like_cpp(true);
            })
            .unwrap();

        let plan = session
            .core
            .with_owned_player_like_cpp(|player| {
                player.plan_spell_cast_resource_debits_like_cpp(&[(17_030, 1)], &[])
            })
            .unwrap()
            .expect("the loaded Ankh is visible to the canonical reagent planner");
        let applied = session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.apply_spell_cast_resource_debits_like_cpp(&plan)
            })
            .unwrap();
        assert!(applied.is_some());
        assert!(session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.apply_spell_cast_resource_debits_like_cpp(&plan)
            })
            .unwrap()
            .is_none(), "the same immutable plan cannot be applied twice");

        let save_port = crate::session::RecordingPlayerLifecyclePort::new(outcome.clone());
        session.set_player_lifecycle_port_like_cpp(save_port.clone());
        let _ = session.save_current_player_to_db_like_cpp().await;
        let requests = save_port.character_save_requests();
        let saved = requests[0].inventory_items.as_ref().expect("full save captured the loaded Item");
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].item_guid, 70_001);
        assert_eq!(saved[0].count, if initial_count == 10 { 9 } else { 1 });
        assert_eq!(
            saved[0].state,
            if initial_count == 10 { PlayerItemSaveStateLikeCpp::Changed } else { PlayerItemSaveStateLikeCpp::Removed }
        );

        let applied_save = matches!(outcome, PersistenceOutcomeLikeCpp::Applied { .. });
        session
            .core
            .with_owned_player_like_cpp(|player| {
                let items = player.inventory_runtime_like_cpp().item_objects();
                if initial_count == 10 {
                    assert_eq!(items[&item_guid].count(), 9);
                    assert_eq!(items[&item_guid].update_state(), if applied_save { ItemUpdateState::Unchanged } else { ItemUpdateState::Changed });
                } else if applied_save {
                    assert!(!items.contains_key(&item_guid), "applied tombstone acknowledgement retires the Item");
                    assert!(player.get_item_by_pos(wow_entities::INVENTORY_SLOT_BAG_0, 35).is_none());
                } else {
                    assert_eq!(items[&item_guid].update_state(), ItemUpdateState::Removed);
                }
            })
            .unwrap();
    }
}

#[tokio::test]
async fn incomplete_item_save_inputs_keep_coherent_carried_load_but_reject_reagents() {
    #[derive(Clone, Copy)]
    enum InputCase {
        MissingPayload,
        MissingEffectCatalog,
        MissingItemTemplate,
        InvalidContext,
        HighBitGem,
        MissingRawEnchantStore,
        MissingRandomPropertyEnchantStore,
        MissingRandomSuffixEnchantStore,
        FailedRepair,
        UnknownRepair,
        KnownAbsentRandomProperty,
    }

    let cases = [
        ("missing complete payload", InputCase::MissingPayload, false),
        ("missing ItemEffect catalog", InputCase::MissingEffectCatalog, false),
        ("missing ItemSparse template", InputCase::MissingItemTemplate, false),
        ("unrepresentable ItemContext", InputCase::InvalidContext, false),
        ("unsigned high-bit gem id", InputCase::HighBitGem, false),
        ("missing raw enchantment catalog", InputCase::MissingRawEnchantStore, false),
        ("random-property effective enchantment without enchantment catalog", InputCase::MissingRandomPropertyEnchantStore, false),
        ("random-suffix effective enchantment without enchantment catalog", InputCase::MissingRandomSuffixEnchantStore, false),
        ("failed required refund repair", InputCase::FailedRepair, false),
        ("unknown required refund repair", InputCase::UnknownRepair, false),
        ("known-absent random-property row", InputCase::KnownAbsentRandomProperty, true),
    ];
    for (label, case, expected_authority) in cases {
        let (mut session, _, _) = make_session();
        let owner = install_canonical_player_owner_for_test(&mut session, 571, 0);
        install_catalogs(
            &mut session,
            &[
                (42_001, InventoryType::NonEquip as i8, 0, 20, 40),
            ],
            !matches!(case, InputCase::MissingEffectCatalog),
        );
        if matches!(case, InputCase::MissingItemTemplate) {
            session.set_item_stats_store(Arc::new(
                wow_data::ItemStatsStore::from_sparse_templates(std::iter::empty::<(
                    u32,
                    wow_data::ItemSparseTemplateEntry,
                )>()),
            ));
        }
        let mut loaded = row(42_001, 72_001, 1);
        match case {
            InputCase::MissingPayload => loaded.complete_payload = None,
            InputCase::InvalidContext => loaded.context = u8::MAX,
            InputCase::HighBitGem => loaded.gems[0] = (i32::MIN, String::new(), 0),
            InputCase::MissingRawEnchantStore => loaded.enchantments = enchantments_string(991, 0, 0),
            InputCase::MissingRandomPropertyEnchantStore => {
                loaded.random_properties_id = 701;
                session.set_item_random_properties_store(Arc::new(
                    wow_data::ItemRandomPropertiesStore::from_entries([
                        wow_data::ItemRandomPropertiesEntry { id: 701, enchantments: [991, 0, 0, 0, 0] },
                    ]),
                ));
            }
            InputCase::MissingRandomSuffixEnchantStore => {
                loaded.random_properties_id = -702;
                session.set_item_random_suffix_store(Arc::new(
                    wow_data::ItemRandomSuffixStore::from_entries([
                        wow_data::ItemRandomSuffixEntry { id: 702, enchantments: [992, 0, 0, 0, 0], allocation_pct: [0; 5] },
                    ]),
                ));
            }
            InputCase::FailedRepair | InputCase::UnknownRepair => {
                loaded.flags = item_flags(ItemFieldFlags::REFUNDABLE);
                loaded.played_time = 7_201;
            }
            InputCase::KnownAbsentRandomProperty => {
                loaded.random_properties_id = 703;
                session.set_item_random_properties_store(Arc::new(
                    wow_data::ItemRandomPropertiesStore::from_entries(std::iter::empty::<
                        wow_data::ItemRandomPropertiesEntry,
                    >()),
                ));
            }
            InputCase::MissingEffectCatalog | InputCase::MissingItemTemplate => {}
        }
        let repair_outcome = match case {
            InputCase::FailedRepair => PersistenceOutcomeLikeCpp::Failed { reason: "repair failed".into() },
            InputCase::UnknownRepair => PersistenceOutcomeLikeCpp::Unknown { reason: "repair commit unknown".into() },
            _ => PersistenceOutcomeLikeCpp::Applied { rows: 0 },
        };
        let item_guid = wow_core::ObjectGuid::create_item(1, 72_001);
        let (_, port) = load_rows(
            &mut session,
            owner,
            vec![equipment(35, loaded)],
            vec![],
            repair_outcome,
        )
        .await;
        let (carried, complete, plan_available, random_id) = session
            .core
            .with_owned_player_like_cpp(|player| {
                (
                    player.inventory_runtime_like_cpp().carried_inventory_loaded_like_cpp(),
                    player.inventory_runtime_like_cpp().equipment_inventory_authority_complete_like_cpp(),
                    player.plan_spell_cast_resource_debits_like_cpp(&[(42_001, 1)], &[]).is_some(),
                    player.inventory_runtime_like_cpp().item_objects().get(&item_guid).map(|item| item.data().random_properties_id),
                )
            })
            .unwrap();
        assert!(carried, "{label}: coherent carried topology remains loaded");
        assert_eq!(complete, expected_authority, "{label}");
        assert_eq!(plan_available, expected_authority, "{label}: planner must follow full-save authority");
        if matches!(case, InputCase::KnownAbsentRandomProperty) {
            assert_eq!(random_id, Some(0), "a row absent from a complete random-property catalog keeps native zero/default behavior");
        }
        if matches!(case, InputCase::FailedRepair | InputCase::UnknownRepair) {
            assert_eq!(port.repair_requests().len(), 1, "{label}: the required repair was submitted");
        }
    }
}

#[tokio::test]
async fn login_refund_repair_and_refundable_before_bop_precedence_apply_to_bag_children() {
    let (mut session, _, _) = make_session();
    let owner = install_canonical_player_owner_for_test(&mut session, 571, 0);
    install_catalogs(
        &mut session,
        &[
            (43_001, InventoryType::NonEquip as i8, 0, 1, 40),
            (43_002, InventoryType::Bag as i8, 4, 1, 40),
            (43_003, InventoryType::NonEquip as i8, 0, 1, 40),
        ],
        true,
    );
    let refundable_trade_flags = ItemFieldFlags::SOULBOUND
        | ItemFieldFlags::REFUNDABLE
        | ItemFieldFlags::BOP_TRADEABLE;
    let mut valid_refund = row(43_001, 73_001, 1);
    valid_refund.flags = item_flags(refundable_trade_flags);
    valid_refund.played_time = 3;
    valid_refund.paid_money = Some(700);
    valid_refund.paid_extended_cost = Some(8);
    let bag_item = row(43_002, 73_002, 1);
    let mut expired_child = row(43_003, 73_003, 1);
    expired_child.flags = item_flags(refundable_trade_flags);
    expired_child.played_time = 7_201;
    expired_child.complete_payload.as_mut().unwrap().soulbound_trade_allowed_players =
        Some("80 81".to_owned());
    let (snapshot, port) = load_rows(
        &mut session,
        owner,
        vec![
            equipment(35, valid_refund),
            equipment(wow_entities::INVENTORY_SLOT_BAG_START, bag_item),
        ],
        vec![bag(wow_entities::INVENTORY_SLOT_BAG_START, 1, expired_child)],
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    )
    .await;
    let refund_guid = wow_core::ObjectGuid::create_item(1, 73_001);
    let child_guid = wow_core::ObjectGuid::create_item(1, 73_003);
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let items = player.inventory_runtime_like_cpp().item_objects();
            let valid = &items[&refund_guid];
            assert!(valid.is_refundable());
            assert_eq!(valid.refund_recipient(), owner);
            assert_eq!((valid.paid_money(), valid.paid_extended_cost()), (700, 8));
            assert!(valid.soulbound_trade_allowed_guids().is_empty());
            assert!(!player.soulbound_tradeable_items().contains(&refund_guid));

            let expired = &items[&child_guid];
            assert!(!expired.has_item_flag(ItemFieldFlags::REFUNDABLE));
            assert!(expired.has_item_flag(ItemFieldFlags::BOP_TRADEABLE));
            assert!(expired.soulbound_trade_allowed_guids().is_empty());
            assert!(!player.soulbound_tradeable_items().contains(&child_guid));
            assert!(player
                .inventory_runtime_like_cpp()
                .equipment_inventory_authority_complete_like_cpp());
        })
        .unwrap();
    assert_eq!(
        port.repair_requests(),
        vec![PlayerLoginItemRepairRequestLikeCpp {
            actions: vec![PlayerLoginItemRepairActionLikeCpp::ClearRefundable {
                item_guid: 73_003,
                new_flags: item_flags(ItemFieldFlags::SOULBOUND | ItemFieldFlags::BOP_TRADEABLE),
            }],
        }]
    );
    assert!(snapshot.item_creates.iter().any(|item| item.item_guid == child_guid && item.dynamic_flags == item_flags(ItemFieldFlags::SOULBOUND | ItemFieldFlags::BOP_TRADEABLE)));
}

#[tokio::test]
async fn login_bop_allowlist_registers_only_with_known_template_and_complete_owner() {
    let (mut session, _, _) = make_session();
    let owner = install_canonical_player_owner_for_test(&mut session, 571, 0);
    install_catalogs(
        &mut session,
        &[(44_001, InventoryType::NonEquip as i8, 0, 1, 40)],
        true,
    );
    let flags = item_flags(ItemFieldFlags::SOULBOUND | ItemFieldFlags::BOP_TRADEABLE);
    let mut valid = row(44_001, 74_001, 1);
    valid.flags = flags;
    valid.complete_payload.as_mut().unwrap().soulbound_trade_allowed_players =
        Some("5001 5002 5001".to_owned());
    let mut unknown = row(44_002, 74_002, 1);
    unknown.flags = flags;
    unknown.complete_payload.as_mut().unwrap().soulbound_trade_allowed_players =
        Some("6001 6002".to_owned());
    let (snapshot, port) = load_rows(
        &mut session,
        owner,
        vec![equipment(35, valid), equipment(36, unknown)],
        vec![],
        PersistenceOutcomeLikeCpp::Applied { rows: 0 },
    )
    .await;
    let valid_guid = wow_core::ObjectGuid::create_item(1, 74_001);
    let unknown_guid = wow_core::ObjectGuid::create_item(1, 74_002);
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let items = player.inventory_runtime_like_cpp().item_objects();
            assert!(items[&valid_guid].is_bop_tradeable());
            assert_eq!(
                items[&valid_guid].soulbound_trade_allowed_guids(),
                &std::collections::HashSet::from([
                    wow_core::ObjectGuid::create_player(1, 5_001),
                    wow_core::ObjectGuid::create_player(1, 5_002),
                ])
            );
            assert!(player.soulbound_tradeable_items().contains(&valid_guid));
            assert!(items[&unknown_guid].is_bop_tradeable(), "unknown template preserves the persisted flag");
            assert!(items[&unknown_guid].soulbound_trade_allowed_guids().is_empty());
            assert!(!player.soulbound_tradeable_items().contains(&unknown_guid));
            assert!(player.inventory_runtime_like_cpp().carried_inventory_loaded_like_cpp());
            assert!(!player.inventory_runtime_like_cpp().equipment_inventory_authority_complete_like_cpp());
        })
        .unwrap();
    assert!(snapshot.item_creates.iter().any(|item| item.item_guid == valid_guid && item.dynamic_flags & flags == flags));
    assert!(snapshot.item_creates.iter().any(|item| item.item_guid == unknown_guid && item.dynamic_flags & flags == flags));
    assert!(port.repair_requests().is_empty());
}
