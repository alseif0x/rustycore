//! Misc scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn tact_key_db_query_bulk_miss_returns_invalid_like_cpp_client_cache_fallback() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);

    session
        .handle_db_query_bulk(wow_packet::packets::misc::DbQueryBulk {
            table_hash: TACT_KEY_TABLE_HASH_LIKE_CPP,
            queries: vec![3909],
        })
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::DbReply as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), TACT_KEY_TABLE_HASH_LIKE_CPP);
    assert_eq!(pkt.read_int32().unwrap(), 3909);
    let _timestamp = pkt.read_int32().unwrap();
    assert_eq!(pkt.read_bits(3).unwrap(), 3);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
}
#[tokio::test]
async fn tact_key_db_query_bulk_hit_returns_typed_valid_write_record_like_cpp() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);
    let key = [0xA5; wow_data::TACTKEY_SIZE];
    session.set_tact_key_store(Arc::new(wow_data::TactKeyStore::from_entries([
        wow_data::TactKeyEntry { id: 3909, key },
    ])));

    session
        .handle_db_query_bulk(wow_packet::packets::misc::DbQueryBulk {
            table_hash: TACT_KEY_TABLE_HASH_LIKE_CPP,
            queries: vec![3909],
        })
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::DbReply as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), TACT_KEY_TABLE_HASH_LIKE_CPP);
    assert_eq!(pkt.read_int32().unwrap(), 3909);
    let _timestamp = pkt.read_int32().unwrap();
    assert_eq!(pkt.read_bits(3).unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), wow_data::TACTKEY_SIZE as u32);
    let data = pkt.read_bytes(wow_data::TACTKEY_SIZE).unwrap();
    assert_eq!(data, key);
}
#[tokio::test]
async fn db_query_bulk_raw_blob_cache_is_not_sent_as_typed_cpp_storage() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);
    let mut cache = wow_data::HotfixBlobCache::new();
    cache.insert_blob(0x919B_E54E, 198647, vec![0xAA; 408]);
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp {
        hotfixes: Arc::new(cache),
        ..Default::default()
    };
    let mut request = WorldPacket::new_empty();
    request.write_uint16(wow_constants::ClientOpcodes::DbQueryBulk as u16);
    request.write_uint32(0x919B_E54E);
    request.write_bits(1, 13);
    request.flush_bits();
    request.write_int32(198647);
    session
        .dispatch_packet(&catalogs, WorldPacket::from_bytes(request.data()))
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::DbReply as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), 0x919B_E54E);
    assert_eq!(pkt.read_int32().unwrap(), 198647);
    let _timestamp = pkt.read_int32().unwrap();
    assert_eq!(pkt.read_bits(3).unwrap(), 3);
    assert_eq!(pkt.read_uint32().unwrap(), 0);
    assert!(realm_rx.try_recv().is_err());
}
#[tokio::test]
async fn query_page_text_without_catalog_capability_sends_cpp_deny_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);

    session
        .handle_query_page_text(QueryPageText {
            page_text_id: 123,
            item_guid: ObjectGuid::EMPTY,
        })
        .await;

    let bytes = send_rx.try_recv().expect("query page text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryPageTextResponse as u16
    );
    assert_eq!(&bytes[2..6], &123_u32.to_le_bytes());
    assert_eq!(bytes[6], 0x00);
    assert_eq!(bytes.len(), 7);
}
#[tokio::test]
async fn query_page_text_uses_typed_catalog_and_preserves_exact_chain_packet_like_cpp() {
    let pages = vec![
        PageTextLikeCpp {
            id: 123,
            next_page_id: 124,
            player_condition_id: -7,
            flags: 3,
            text: "Primera página".to_owned(),
        },
        PageTextLikeCpp {
            id: 124,
            next_page_id: 0,
            player_condition_id: 9,
            flags: 5,
            text: "Segunda página".to_owned(),
        },
    ];
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    install_world_query_catalogs_like_cpp(&mut session, [], [], pages.clone(), []);

    session
        .handle_query_page_text(QueryPageText {
            page_text_id: 123,
            item_guid: ObjectGuid::EMPTY,
        })
        .await;

    assert_eq!(
        send_rx.try_recv().unwrap(),
        QueryPageTextResponse {
            page_text_id: 123,
            allow: true,
            pages: pages
                .into_iter()
                .map(|page| PageTextInfo {
                    id: page.id,
                    next_page_id: page.next_page_id,
                    player_condition_id: page.player_condition_id,
                    flags: page.flags,
                    text: page.text,
                })
                .collect(),
        }
        .to_bytes()
    );
}
#[tokio::test]
async fn query_page_text_preserves_partial_chain_and_empty_failure_shapes_like_cpp() {
    for expected_pages in [
        vec![PageTextLikeCpp {
            id: 123,
            next_page_id: 124,
            player_condition_id: 0,
            flags: 0,
            text: "partial".to_owned(),
        }],
        Vec::new(),
    ] {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        install_world_query_catalogs_like_cpp(&mut session, [], [], expected_pages.clone(), []);

        session
            .handle_query_page_text(QueryPageText {
                page_text_id: 123,
                item_guid: ObjectGuid::EMPTY,
            })
            .await;

        assert_eq!(
            send_rx.try_recv().unwrap(),
            QueryPageTextResponse {
                page_text_id: 123,
                allow: !expected_pages.is_empty(),
                pages: expected_pages
                    .into_iter()
                    .map(|page| PageTextInfo {
                        id: page.id,
                        next_page_id: page.next_page_id,
                        player_condition_id: page.player_condition_id,
                        flags: page.flags,
                        text: page.text,
                    })
                    .collect(),
            }
            .to_bytes()
        );
    }
}
#[tokio::test]
async fn query_player_names_without_port_preserves_failure_order_and_realm_routing() {
    let first = ObjectGuid::create_player(1, 41);
    let second = ObjectGuid::create_player(1, 42);
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);

    session
        .handle_query_player_names(QueryPlayerNames {
            players: vec![first, second],
        })
        .await;

    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        QueryPlayerNamesResponse {
            players: vec![
                NameCacheLookupResult {
                    player: first,
                    result: 1,
                    data: None,
                },
                NameCacheLookupResult {
                    player: second,
                    result: 1,
                    data: None,
                },
            ],
        }
        .to_bytes()
    );
}
#[tokio::test]
async fn query_player_names_uses_typed_port_and_preserves_exact_mixed_packet_like_cpp() {
    let found = ObjectGuid::create_player(1, 41);
    let missing = ObjectGuid::create_player(1, 42);
    let failed = ObjectGuid::create_player(1, 43);
    let port = PlayerNameQueryPortFixtureLikeCpp::new([
        PlayerNameQueryOutcomeLikeCpp::Found(PlayerNameQueryRowLikeCpp {
            name: "Target".to_owned(),
            race: 10,
            class: 3,
            sex: 1,
            level: 80,
        }),
        PlayerNameQueryOutcomeLikeCpp::Missing,
        PlayerNameQueryOutcomeLikeCpp::Failed {
            reason: "character query failed".to_owned(),
        },
    ]);
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(1);
    session.set_player_name_query_persistence_port_like_cpp(port.clone());

    session
        .handle_query_player_names(QueryPlayerNames {
            players: vec![found, missing, failed],
        })
        .await;

    assert_eq!(
        port.requests(),
        vec![
            PlayerNameQueryRequestLikeCpp {
                player_guid_counter: 41,
            },
            PlayerNameQueryRequestLikeCpp {
                player_guid_counter: 42,
            },
            PlayerNameQueryRequestLikeCpp {
                player_guid_counter: 43,
            },
        ]
    );
    assert!(instance_rx.try_recv().is_err());

    let account_id = ObjectGuid::new((HighGuid::WowAccount as i64) << 58, 1);
    let bnet_account_id = ObjectGuid::new((HighGuid::BNetAccount as i64) << 58, 1);
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        QueryPlayerNamesResponse {
            players: vec![
                NameCacheLookupResult {
                    player: found,
                    result: 0,
                    data: Some(PlayerGuidLookupData {
                        name: "Target".to_owned(),
                        race: 10,
                        sex: 1,
                        class: 3,
                        level: 80,
                        guid_actual: found,
                        account_id,
                        bnet_account_id,
                        virtual_realm_address: session.virtual_realm_address(),
                        ..Default::default()
                    }),
                },
                NameCacheLookupResult {
                    player: missing,
                    result: 1,
                    data: None,
                },
                NameCacheLookupResult {
                    player: failed,
                    result: 1,
                    data: None,
                },
            ],
        }
        .to_bytes()
    );
}
#[tokio::test]
async fn query_corpse_location_without_runtime_corpse_sends_cpp_invalid_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);

    session
        .handle_query_corpse_location(QueryCorpseLocationFromClient { player })
        .await;

    let bytes = send_rx.try_recv().expect("corpse location response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::CorpseLocation as u16
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(&bytes[3..19], &player.to_raw_bytes());
    assert_eq!(bytes.len(), 55);
}
#[tokio::test]
async fn query_corpse_transport_without_runtime_corpse_sends_cpp_default_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player = ObjectGuid::create_player(1, 0xAABB_CCDD);
    let transport = ObjectGuid::create_world_object(HighGuid::Transport, 0, 1, 571, 0, 77, 42);

    session
        .handle_query_corpse_transport(QueryCorpseTransport { player, transport })
        .await;

    let bytes = send_rx.try_recv().expect("corpse transport response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::CorpseTransportQuery as u16
    );
    assert_eq!(&bytes[2..18], &player.to_raw_bytes());
    assert_eq!(&bytes[18..22], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[22..26], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[26..30], &0.0_f32.to_le_bytes());
    assert_eq!(&bytes[30..34], &0.0_f32.to_le_bytes());
    assert_eq!(bytes.len(), 34);
}
#[test]
fn start_zones_are_valid() {
    for race in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
        let zone = start_zone(race);
        assert!(zone > 0, "Race {race} has invalid zone");
    }
}
#[test]
fn vendor_buy_price_uses_cpp_buy_count_unit_price() {
    assert_eq!(vendor_buy_quantity_and_price(500, 5, 1), (1, 100));
    assert_eq!(vendor_buy_quantity_and_price(500, 5, 3), (3, 300));
    assert_eq!(vendor_buy_quantity_and_price(500, 0, 2), (2, 1000));
    assert_eq!(vendor_buy_quantity_and_price(0, 5, 3), (3, 0));
    assert_eq!(vendor_buy_quantity_and_price(1, 5, 1), (1, 1));
}
#[test]
fn vendor_buy_zero_gold_price_does_not_dirty_coinage_like_cpp() {
    assert_eq!(vendor_buy_coinage_update_like_cpp(0, 12_345), None);
    assert_eq!(vendor_buy_coinage_update_like_cpp(1, 12_344), Some(12_344));
}
#[test]
fn vendor_buy_packet_quantity_uses_cpp_uint8_count_conversion() {
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(0), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(1), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(256), 1);
    assert_eq!(vendor_buy_packet_quantity_to_cpp_count(-1), 255);
}
#[test]
fn vendor_buy_currency_preflight_matches_cpp_quantity_guards() {
    assert_eq!(vendor_buy_currency_packet_quantity_to_cpp_count(0), 1);
    assert_eq!(vendor_buy_currency_packet_quantity_to_cpp_count(5), 5);
    assert_eq!(
        vendor_buy_currency_quantity_block_result(5, 3),
        Some(InventoryResult::CantBuyQuantity)
    );
    assert_eq!(vendor_buy_currency_quantity_block_result(5, 10), None);
    assert_eq!(
        vendor_buy_currency_quantity_block_result(0, 10),
        Some(InventoryResult::CantBuyQuantity)
    );
}
#[test]
fn vendor_buy_muid_uses_cpp_one_based_uint32_slot_conversion() {
    assert_eq!(vendor_buy_muid_to_cpp_slot(0), None);
    assert_eq!(vendor_buy_muid_to_cpp_slot(1), Some(0));
    assert_eq!(vendor_buy_muid_to_cpp_slot(2), Some(1));
    assert_eq!(vendor_buy_muid_to_cpp_slot(-1), Some(u32::MAX - 1));
}
#[test]
fn vendor_list_currency_rows_match_cpp_basic_guards() {
    let store = CurrencyTypesStore::from_entries([wow_data::CurrencyTypesEntry {
        id: 395,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }]);
    assert!(vendor_list_should_skip_currency_row(Some(&store), 395, 0,));
    assert!(!vendor_list_should_skip_currency_row(Some(&store), 395, 10,));
    assert!(vendor_list_should_skip_currency_row(
        Some(&store),
        999_999,
        10
    ));
    assert!(vendor_list_should_skip_currency_row(None, 395, 10));
}
#[test]
fn vendor_player_condition_id_evaluates_player_condition_store_like_cpp() {
    let store = PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 0,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ]);
    let context = PlayerConditionContextLikeCpp {
        class_mask: 1,
        ..Default::default()
    };

    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(0, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(42, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(43, Some(&store), Some(context)),
        43
    );
    assert_eq!(
        vendor_player_condition_failed_id_like_cpp(999, Some(&store), Some(context)),
        0
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(42, Some(&store), Some(context)),
        None
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(43, Some(&store), Some(context)),
        Some(InventoryResult::ItemLocked)
    );
    assert_eq!(
        vendor_buy_player_condition_block_result_like_cpp(42, None, Some(context)),
        Some(InventoryResult::ItemLocked)
    );
}
#[test]
fn vendor_condition_presence_fails_closed_until_condition_mgr_exists() {
    assert_eq!(vendor_conditions_block_result(false), None);
    assert_eq!(
        vendor_conditions_block_result(true),
        Some(BuyResult::CantFindItem)
    );
}
#[test]
fn vendor_buy_extended_cost_fails_closed_like_cpp_preflight() {
    let currency_store = CurrencyTypesStore::from_entries([wow_data::CurrencyTypesEntry {
        id: 395,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }]);
    let extended_cost_store =
        ItemExtendedCostStore::from_entries([wow_data::ItemExtendedCostEntry {
            id: 12,
            required_arena_rating: 0,
            arena_bracket: 0,
            flags: wow_constants::ItemExtendedCostFlags::empty(),
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
            item_count: [0; wow_data::MAX_ITEM_EXT_COST_ITEMS],
            currency_id: [395, 0, 0, 0, 0],
            currency_count: [10, 0, 0, 0, 0],
        }]);

    assert_eq!(
        vendor_buy_extended_cost_block_result(
            None,
            None,
            |_, _| false,
            |_, _| false,
            false,
            0,
            5,
            3
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| false,
            |_, _| false,
            false,
            12,
            5,
            3
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantBuyQuantity
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| currency_id == 395 && amount >= 20,
            false,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| currency_id == 395 && amount >= 20,
            true,
            12,
            5,
            10
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_currency_costs(Some(&extended_cost_store), 12, 5, 10),
        vec![(395, 20)]
    );
    let item_turnin_store =
        ItemExtendedCostStore::from_entries([wow_data::ItemExtendedCostEntry {
            id: 13,
            required_arena_rating: 0,
            arena_bracket: 0,
            flags: wow_constants::ItemExtendedCostFlags::empty(),
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [700, 0, 0, 0, 0],
            item_count: [3, 0, 0, 0, 0],
            currency_id: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
            currency_count: [0; wow_data::MAX_ITEM_EXT_COST_CURRENCIES],
        }]);
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&item_turnin_store),
            Some(&currency_store),
            |item_id, amount| item_id == 700 && amount == 6,
            |_, _| true,
            true,
            13,
            5,
            10
        ),
        None
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&item_turnin_store),
            Some(&currency_store),
            |_, _| false,
            |_, _| true,
            true,
            13,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert_eq!(
        vendor_buy_extended_cost_item_costs(Some(&item_turnin_store), 13, 5, 10),
        vec![(700, 6)]
    );
    let checked_currency_amount = std::cell::Cell::new(false);
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |currency_id, amount| {
                checked_currency_amount.set(true);
                assert_eq!(currency_id, 395);
                assert_eq!(amount, 20);
                false
            },
            true,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins
        ))
    );
    assert!(checked_currency_amount.get());
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            None,
            |_, _| true,
            |_, _| true,
            true,
            12,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Buy(BuyResult::CantFindItem))
    );
    assert_eq!(
        vendor_buy_extended_cost_block_result(
            Some(&extended_cost_store),
            Some(&currency_store),
            |_, _| true,
            |_, _| true,
            true,
            99,
            5,
            10
        ),
        Some(VendorExtendedCostBlock::Silent)
    );
}
#[test]
fn vendor_buy_direct_store_preflight_matches_cpp_store_branch() {
    assert_eq!(
        vendor_buy_direct_store_block_result(NULL_BAG, NULL_SLOT, 1),
        None
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(INVENTORY_SLOT_BAG_0, 35, 1),
        None
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(NULL_BAG, 35, 1),
        Some(InventoryResult::WrongSlot)
    );
    assert_eq!(
        vendor_buy_direct_store_block_result(INVENTORY_SLOT_BAG_0, 0, 1),
        Some(InventoryResult::NotEquippable)
    );
}
#[test]
fn vendor_buy_stock_refill_matches_cpp_increment_and_full_reset() {
    assert_eq!(vendor_buy_stock_refill_count(2, 20, 10, 5, 20), (12, false));
    assert_eq!(vendor_buy_stock_refill_count(18, 10, 10, 5, 20), (20, true));
    assert_eq!(vendor_buy_stock_refill_count(2, 9, 10, 5, 20), (2, false));
}
#[tokio::test]
async fn cancel_temp_enchantment_ignores_missing_enchant_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 15, 0);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 15 })
        .await;

    let item = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].duration,
        12_000
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn vendor_list_sold_out_filter_matches_cpp_gm_branch() {
    assert!(vendor_list_should_skip_sold_out(5, 0, false));
    assert!(!vendor_list_should_skip_sold_out(5, 0, true));
    assert!(!vendor_list_should_skip_sold_out(5, 1, false));
    assert!(!vendor_list_should_skip_sold_out(0, 0, false));
}
#[test]
fn vendor_list_refundable_flag_matches_cpp_template_guard() {
    assert!(vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(1),
        42
    ));
    assert!(!vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(2),
        42
    ));
    assert!(!vendor_list_item_refundable(
        Some(ItemFlags::ITEM_PURCHASE_RECORD),
        Some(1),
        0
    ));
    assert!(!vendor_list_item_refundable(None, Some(1), 42));
}
#[test]
fn vendor_list_allowed_class_filter_matches_cpp_bind_on_acquire_branch() {
    let warrior_mask = 1i16 << (1 - 1);
    let mage_mask = 1i16 << (8 - 1);

    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        1,
        false,
    ));
    assert!(vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnEquip as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        true,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(warrior_mask | mage_mask),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
    assert!(!vendor_list_should_skip_allowed_class(
        Some(-1),
        Some(ItemBondingType::OnAcquire as u8),
        8,
        false,
    ));
}
#[test]
fn vendor_list_faction_filter_matches_cpp_team_branch() {
    assert_eq!(player_team_for_race_cpp(1), Team::Alliance);
    assert_eq!(player_team_for_race_cpp(2), Team::Horde);
    assert_eq!(player_team_for_race_cpp(11), Team::Alliance);
    assert_eq!(player_team_for_race_cpp(10), Team::Horde);

    assert!(vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionHorde as u32),
        Team::Alliance,
        false,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionHorde as u32),
        Team::Horde,
        false,
    ));
    assert!(vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionAlliance as u32),
        Team::Horde,
        false,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        Some(ItemFlags2::FactionAlliance as u32),
        Team::Horde,
        true,
    ));
    assert!(!vendor_list_should_skip_faction_flags(
        None,
        Team::Alliance,
        false
    ));
}
#[test]
fn vendor_buy_template_gates_match_cpp_error_shapes() {
    let warrior_mask = 1i16 << (1 - 1);

    assert_eq!(
        vendor_buy_template_block_result(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            None,
            8,
            1,
            false,
        ),
        Some(VendorBuyTemplateBlock::BuyError(BuyResult::CantFindItem))
    );
    assert_eq!(
        vendor_buy_template_block_result(
            Some(warrior_mask),
            Some(ItemBondingType::OnAcquire as u8),
            None,
            8,
            1,
            true,
        ),
        None
    );
    assert_eq!(
        vendor_buy_template_block_result(
            None,
            None,
            Some(ItemFlags2::FactionHorde as u32),
            1,
            1,
            false,
        ),
        Some(VendorBuyTemplateBlock::Silent)
    );
    assert_eq!(
        vendor_buy_template_block_result(
            None,
            None,
            Some(ItemFlags2::FactionHorde as u32),
            1,
            2,
            false,
        ),
        None
    );
}
#[test]
fn vendor_buy_destination_uses_cpp_uint8_slot_conversion() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: 256,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        Some((INVENTORY_SLOT_BAG_0, 0))
    );
}
