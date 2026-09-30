//! Catalog-query packet contracts, moved intact from the private Character tests.

use std::sync::Arc;

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_data::{
    CreatureQueryCatalogLikeCpp, CreatureQueryDisplayLikeCpp, CreatureQueryTemplateLikeCpp,
    GameObjectQueryCatalogLikeCpp, GameObjectQueryTemplateLikeCpp, GameObjectQuestItemStoreLikeCpp,
    HotfixBlobCache, PageTextCatalogLikeCpp, PageTextLikeCpp, TACTKEY_SIZE, TactKeyEntry,
    TactKeyStore, WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP,
};
use wow_packet::packets::misc::DbQueryBulk;
use wow_packet::packets::query::{
    CreatureDisplayStats, CreatureStats, CreatureXDisplay, GameObjectStats, PageTextInfo,
    QueryCreature, QueryCreatureResponse, QueryGameObject, QueryGameObjectResponse,
    QueryPageText, QueryPageTextResponse,
};
use wow_packet::{ServerPacket, WorldPacket};
use wow_world::session::{ObjectMgrCatalogsLikeCpp, SessionHandlerCatalogsLikeCpp, WorldSession};
use wow_world::test_fixtures::{
    dispatch_db_query_bulk_for_test, handle_creature_query_for_test,
    handle_db_query_bulk_for_test, handle_gameobject_query_for_test,
    handle_page_text_query_for_test, install_object_mgr_catalogs_for_test,
    install_realm_send_channel_for_test, install_tact_key_store_for_test,
    set_equipment_set_guid_generator_for_test,
};

// Preserve the original TactKey table hash used by these wire expectations.
const TACT_KEY_TABLE_HASH_LIKE_CPP: u32 = 0xDF2F_53CF;

fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(16);
    (
        WorldSession::new(
            1,
            "TestAccount".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            pkt_rx,
            send_tx,
        ),
        send_rx,
    )
}

fn make_session_with_realm_send() -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    install_realm_send_channel_for_test(&mut session, realm_tx);
    (session, instance_rx, realm_rx)
}

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    set_equipment_set_guid_generator_for_test(&mut session, Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn install_page_text_catalog_like_cpp(
    session: &mut WorldSession,
    pages: impl IntoIterator<Item = PageTextLikeCpp>,
) {
    install_object_mgr_catalogs_for_test(session, Arc::new(ObjectMgrCatalogsLikeCpp {
        page_text: Arc::new(PageTextCatalogLikeCpp::from_rows_like_cpp(pages, [])),
        ..Default::default()
    }));
}

fn install_creature_query_catalog_like_cpp(
    session: &mut WorldSession,
    creatures: impl IntoIterator<Item = CreatureQueryTemplateLikeCpp>,
) {
    install_object_mgr_catalogs_for_test(session, Arc::new(ObjectMgrCatalogsLikeCpp {
        creature: Arc::new(CreatureQueryCatalogLikeCpp::from_rows_like_cpp(
            creatures,
            [],
        )),
        ..Default::default()
    }));
}

fn creature_query_catalog_row_like_cpp() -> CreatureQueryTemplateLikeCpp {
    CreatureQueryTemplateLikeCpp {
        entry: 42,
        name: "Localized creature".to_owned(),
        subname: "Localized title".to_owned(),
        title_alt: "Localized alternate".to_owned(),
        icon_name: "Directions".to_owned(),
        creature_type: 7,
        creature_family: 8,
        classification: 9,
        kill_credits: [10, 11],
        civilian: true,
        racial_leader: false,
        movement_id: 12,
        required_expansion: 3,
        vignette_id: 13,
        unit_class: 1,
        widget_set_id: 14,
        widget_set_unit_condition_id: 15,
        hp_multi: 1.5,
        energy_multi: 2.5,
        creature_difficulty_id: 16,
        type_flags: [17, 18],
        displays: vec![CreatureQueryDisplayLikeCpp {
            display_id: 19,
            scale: 0.75,
            probability: 0.25,
        }],
    }
}

fn install_gameobject_query_catalog_like_cpp(
    session: &mut WorldSession,
    gameobjects: impl IntoIterator<Item = GameObjectQueryTemplateLikeCpp>,
    quest_item_rows: impl IntoIterator<Item = (u32, u32, u32)>,
) {
    let quest_items =
        GameObjectQuestItemStoreLikeCpp::from_rows_like_cpp(quest_item_rows, |_| true, |_| true)
            .store;
    install_object_mgr_catalogs_for_test(session, Arc::new(ObjectMgrCatalogsLikeCpp {
        gameobject: Arc::new(GameObjectQueryCatalogLikeCpp::from_rows_like_cpp(
            gameobjects,
            [],
        )),
        gameobject_quest_items: Arc::new(quest_items),
        ..Default::default()
    }));
}

fn gameobject_query_catalog_row_like_cpp() -> GameObjectQueryTemplateLikeCpp {
    let mut data = [0_i32; WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP];
    data[0] = 7;
    data[34] = 41;
    GameObjectQueryTemplateLikeCpp {
        entry: 42,
        go_type: 3,
        display_id: 4,
        name: "Localized object".to_owned(),
        icon_name: "Directions".to_owned(),
        cast_bar_caption: "Opening".to_owned(),
        unk_string: "Unknown".to_owned(),
        size: 1.25,
        data,
        content_tuning_id: 42,
        min_money: 0,
        max_money: 0,
    }
}

#[tokio::test]
async fn tact_key_db_query_bulk_miss_returns_invalid_like_cpp_client_cache_fallback() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();

    handle_db_query_bulk_for_test(&mut session, DbQueryBulk {
            table_hash: TACT_KEY_TABLE_HASH_LIKE_CPP,
            queries: vec![3909],
        })
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::DbReply as u16
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
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();
    let key = [0xA5; TACTKEY_SIZE];
    install_tact_key_store_for_test(&mut session, Arc::new(TactKeyStore::from_entries([TactKeyEntry {
        id: 3909,
        key,
    }])));

    handle_db_query_bulk_for_test(&mut session, DbQueryBulk {
            table_hash: TACT_KEY_TABLE_HASH_LIKE_CPP,
            queries: vec![3909],
        })
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::DbReply as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_uint32().unwrap(), TACT_KEY_TABLE_HASH_LIKE_CPP);
    assert_eq!(pkt.read_int32().unwrap(), 3909);
    let _timestamp = pkt.read_int32().unwrap();
    assert_eq!(pkt.read_bits(3).unwrap(), 1);
    assert_eq!(pkt.read_uint32().unwrap(), TACTKEY_SIZE as u32);
    let data = pkt.read_bytes(TACTKEY_SIZE).unwrap();
    assert_eq!(data, key);
}

#[tokio::test]
async fn db_query_bulk_raw_blob_cache_is_not_sent_as_typed_cpp_storage() {
    // This covers Rust's raw-cache boundary; C++ parity is established by typed WriteRecord above.
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();
    let mut cache = HotfixBlobCache::new();
    cache.insert_blob(0x919B_E54E, 198647, vec![0xAA; 408]);
    let catalogs = SessionHandlerCatalogsLikeCpp {
        hotfixes: Arc::new(cache),
        ..Default::default()
    };
    let mut request = WorldPacket::new_empty();
    request.write_uint16(ClientOpcodes::DbQueryBulk as u16);
    request.write_uint32(0x919B_E54E);
    request.write_bits(1, 13);
    request.flush_bits();
    request.write_int32(198647);
    dispatch_db_query_bulk_for_test(&mut session, &catalogs, WorldPacket::from_bytes(request.data()))
        .await;

    let bytes = realm_rx.try_recv().expect("db reply");
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::DbReply as u16
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
    let (mut session, send_rx) = make_session();

    handle_page_text_query_for_test(&mut session, QueryPageText {
            page_text_id: 123,
            item_guid: ObjectGuid::EMPTY,
        })
        .await;

    let bytes = send_rx.try_recv().expect("query page text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::QueryPageTextResponse as u16
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
    let (mut session, send_rx) = make_session();
    install_page_text_catalog_like_cpp(&mut session, pages.clone());

    handle_page_text_query_for_test(&mut session, QueryPageText {
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
        let (mut session, send_rx) = make_session();
        install_page_text_catalog_like_cpp(&mut session, expected_pages.clone());

        handle_page_text_query_for_test(&mut session, QueryPageText {
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
async fn creature_query_uses_typed_catalog_and_preserves_packet_projection_like_cpp() {
    let row = creature_query_catalog_row_like_cpp();
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    install_creature_query_catalog_like_cpp(&mut session, [row.clone()]);

    handle_creature_query_for_test(&mut session, QueryCreature { creature_id: 42 })
        .await;

    let mut names: [String; 4] = Default::default();
    names[0] = row.name;
    let expected = QueryCreatureResponse {
        creature_id: 42,
        allow: true,
        stats: Some(CreatureStats {
            title: row.subname,
            title_alt: row.title_alt,
            cursor_name: row.icon_name,
            civilian: row.civilian,
            leader: row.racial_leader,
            names,
            name_alts: Default::default(),
            flags: row.type_flags,
            creature_type: row.creature_type,
            creature_family: row.creature_family,
            classification: row.classification,
            proxy_creature_ids: row.kill_credits,
            display: CreatureDisplayStats {
                displays: vec![CreatureXDisplay {
                    creature_display_id: 19,
                    scale: 0.75,
                    probability: 0.25,
                }],
                total_probability: 0.25,
            },
            hp_multi: row.hp_multi,
            energy_multi: row.energy_multi,
            quest_items: Vec::new(),
            creature_movement_info_id: row.movement_id,
            health_scaling_expansion: 0,
            required_expansion: row.required_expansion,
            vignette_id: row.vignette_id,
            unit_class: row.unit_class,
            creature_difficulty_id: row.creature_difficulty_id,
            widget_set_id: row.widget_set_id,
            widget_set_unit_condition_id: row.widget_set_unit_condition_id,
        }),
    };
    assert_eq!(send_rx.try_recv().unwrap(), expected.to_bytes());
}

#[tokio::test]
async fn creature_query_repeats_response_like_cpp() {
    let row = creature_query_catalog_row_like_cpp();
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    install_creature_query_catalog_like_cpp(&mut session, [row]);

    handle_creature_query_for_test(&mut session, QueryCreature { creature_id: 42 })
        .await;
    handle_creature_query_for_test(&mut session, QueryCreature { creature_id: 42 })
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn creature_query_missing_or_failed_catalog_emits_disallowed_response_like_cpp() {
    // An absent Rust catalog capability is synthetic; the empty lookup models C++'s missing template.
    for with_empty_capability in [false, true] {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        if with_empty_capability {
            install_creature_query_catalog_like_cpp(&mut session, []);
        }

        handle_creature_query_for_test(&mut session, QueryCreature { creature_id: 43 })
            .await;

        assert_eq!(
            send_rx.try_recv().unwrap(),
            QueryCreatureResponse {
                creature_id: 43,
                allow: false,
                stats: None,
            }
            .to_bytes()
        );
    }
}

#[tokio::test]
async fn gameobject_query_uses_typed_catalog_and_preserves_packet_projection_like_cpp() {
    let row = gameobject_query_catalog_row_like_cpp();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 0, 571, 0, 42, 99);
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    install_gameobject_query_catalog_like_cpp(
        &mut session,
        [row.clone()],
        [(42, 43, 0), (42, 44, 1)],
    );

    handle_gameobject_query_for_test(&mut session, QueryGameObject {
            game_object_id: 42,
            guid,
        })
        .await;

    let mut names: [String; 4] = Default::default();
    names[0] = row.name;
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QueryGameObjectResponse {
            game_object_id: 42,
            guid,
            allow: true,
            stats: Some(GameObjectStats {
                names,
                icon_name: row.icon_name,
                cast_bar_caption: row.cast_bar_caption,
                unk_string: row.unk_string,
                go_type: row.go_type,
                display_id: row.display_id,
                data: row.data,
                size: row.size,
                quest_items: vec![43, 44],
                content_tuning_id: row.content_tuning_id,
            }),
        }
        .to_bytes()
    );
}

#[tokio::test]
async fn gameobject_query_missing_or_failed_catalog_preserves_guid_and_disallows_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 0, 571, 0, 43, 100);
    // The absent Rust catalog capability is synthetic; only the empty typed lookup models C++ missing-template behavior.
    for with_empty_capability in [false, true] {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        if with_empty_capability {
            install_gameobject_query_catalog_like_cpp(&mut session, [], []);
        }

        handle_gameobject_query_for_test(&mut session, QueryGameObject {
                game_object_id: 43,
                guid,
            })
            .await;

        assert_eq!(
            send_rx.try_recv().unwrap(),
            QueryGameObjectResponse {
                game_object_id: 43,
                guid,
                allow: false,
                stats: None,
            }
            .to_bytes()
        );
    }
}
