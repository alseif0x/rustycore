use super::TACT_KEY_TABLE_HASH_LIKE_CPP;
use crate::handlers::test_support::world::{make_session, make_session_with_realm_send};
use crate::session::{
    ObjectMgrCatalogsLikeCpp, SessionHandlerCatalogsLikeCpp, SessionPlayerController, WorldSession,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_data::{
    HotfixBlobCache, PageTextCatalogLikeCpp, PageTextLikeCpp, TACTKEY_SIZE, TactKeyEntry,
    TactKeyStore,
};
use wow_packet::packets::misc::DbQueryBulk;
use wow_packet::packets::query::{
    NameCacheLookupResult, PageTextInfo, PlayerGuidLookupData, QueryPageText,
    QueryPageTextResponse, QueryPlayerNames, QueryPlayerNamesResponse,
};
use wow_packet::{ServerPacket, WorldPacket};
use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerNameQueryOutcomeLikeCpp, PlayerNameQueryPersistencePortLikeCpp,
    PlayerNameQueryRequestLikeCpp, PlayerNameQueryRowLikeCpp,
};

fn install_page_text_catalog_like_cpp(
    session: &mut WorldSession,
    pages: impl IntoIterator<Item = PageTextLikeCpp>,
) {
    session.set_object_mgr_catalogs_like_cpp(Arc::new(ObjectMgrCatalogsLikeCpp {
        page_text: Arc::new(PageTextCatalogLikeCpp::from_rows_like_cpp(pages, [])),
        ..Default::default()
    }));
}

struct PlayerNameQueryPortFixtureLikeCpp {
    requests: Mutex<Vec<PlayerNameQueryRequestLikeCpp>>,
    outcomes: Mutex<VecDeque<PlayerNameQueryOutcomeLikeCpp>>,
}

impl PlayerNameQueryPortFixtureLikeCpp {
    fn new(outcomes: impl IntoIterator<Item = PlayerNameQueryOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: Mutex::new(Vec::new()),
            outcomes: Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<PlayerNameQueryRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl PlayerNameQueryPersistencePortLikeCpp for PlayerNameQueryPortFixtureLikeCpp {
    fn load_player_name_like_cpp<'a>(
        &'a self,
        request: PlayerNameQueryRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerNameQueryOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one player-name outcome per request");
        Box::pin(async move { outcome })
    }
}

#[tokio::test]
async fn query_player_names_without_port_preserves_failure_order_and_realm_routing() {
    // Rust-only missing persistence capability; C++ resolves this through ObjectAccessor.
    let first = ObjectGuid::create_player(1, 41);
    let second = ObjectGuid::create_player(1, 42);
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();

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
            account_id: 22,
            battlenet_account_id: 77,
            is_deleted: true,
        }),
        PlayerNameQueryOutcomeLikeCpp::Missing,
        // Rust-only synthetic persistence failure; C++ has no port outcome here.
        PlayerNameQueryOutcomeLikeCpp::Failed {
            reason: "character query failed".to_owned(),
        },
    ]);
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();
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

    let account_id = ObjectGuid::new((HighGuid::WowAccount as i64) << 58, 22);
    let bnet_account_id = ObjectGuid::new((HighGuid::BNetAccount as i64) << 58, 77);
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
                        is_deleted: true,
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
async fn query_player_names_connected_target_overlays_live_identity_like_cpp() {
    let found = ObjectGuid::create_player(1, 41);
    let port = PlayerNameQueryPortFixtureLikeCpp::new([PlayerNameQueryOutcomeLikeCpp::Found(
        PlayerNameQueryRowLikeCpp {
            name: "Cached".to_owned(),
            race: 10,
            class: 3,
            sex: 1,
            level: 80,
            account_id: 22,
            battlenet_account_id: 77,
            is_deleted: true,
        },
    )]);
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        found,
        "Connected".to_owned(),
        Position::ZERO,
        571,
        2,
        8,
        55,
        0,
    ));
    session.set_battlenet_account_id(88);
    session.set_player_name_query_persistence_port_like_cpp(port);

    session
        .handle_query_player_names(QueryPlayerNames {
            players: vec![found],
        })
        .await;

    let account_id = ObjectGuid::new((HighGuid::WowAccount as i64) << 58, 1);
    let bnet_account_id = ObjectGuid::new((HighGuid::BNetAccount as i64) << 58, 88);
    assert!(instance_rx.try_recv().is_err());
    assert_eq!(
        realm_rx.try_recv().unwrap(),
        QueryPlayerNamesResponse {
            players: vec![NameCacheLookupResult {
                player: found,
                result: 0,
                data: Some(PlayerGuidLookupData {
                    name: "Connected".to_owned(),
                    race: 2,
                    sex: 0,
                    class: 8,
                    level: 55,
                    guid_actual: found,
                    account_id,
                    bnet_account_id,
                    virtual_realm_address: session.virtual_realm_address(),
                    ..Default::default()
                }),
            }],
        }
        .to_bytes()
    );
}

#[tokio::test]
async fn tact_key_db_query_bulk_miss_returns_invalid_like_cpp_client_cache_fallback() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();

    session
        .handle_db_query_bulk(DbQueryBulk {
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
    session.set_tact_key_store(Arc::new(TactKeyStore::from_entries([TactKeyEntry {
        id: 3909,
        key,
    }])));

    session
        .handle_db_query_bulk(DbQueryBulk {
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
    session
        .dispatch_packet(&catalogs, WorldPacket::from_bytes(request.data()))
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

    session
        .handle_query_page_text(QueryPageText {
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
        let (mut session, send_rx) = make_session();
        install_page_text_catalog_like_cpp(&mut session, expected_pages.clone());

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
