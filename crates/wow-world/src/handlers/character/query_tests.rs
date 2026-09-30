use crate::handlers::test_support::world::{make_session, make_session_with_realm_send};
use crate::session::{SessionPlayerController, WorldSession};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_packet::packets::query::{
    NameCacheLookupResult, PlayerGuidLookupData, QueryPlayerNames, QueryPlayerNamesResponse,
};
use wow_packet::{ServerPacket, WorldPacket};
use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerNameQueryOutcomeLikeCpp, PlayerNameQueryPersistencePortLikeCpp,
    PlayerNameQueryRequestLikeCpp, PlayerNameQueryRowLikeCpp,
};

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
    session.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
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
async fn spirit_healer_activate_without_interactable_healer_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let healer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9, 1);
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&healer);

    session.handle_spirit_healer_activate(request).await;

    assert!(send_rx.try_recv().is_err());
}
