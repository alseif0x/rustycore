use crate::session::{ObjectMgrCatalogsLikeCpp, WorldSession};
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_data::{
    GameObjectQueryCatalogLikeCpp, GameObjectQueryTemplateLikeCpp, GameObjectQuestItemStoreLikeCpp,
    WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP,
};
use wow_packet::packets::query::{GameObjectStats, QueryGameObject, QueryGameObjectResponse};
use wow_packet::{ServerPacket, WorldPacket};

fn install_gameobject_query_catalog_like_cpp(
    session: &mut WorldSession,
    gameobjects: impl IntoIterator<Item = GameObjectQueryTemplateLikeCpp>,
    quest_item_rows: impl IntoIterator<Item = (u32, u32, u32)>,
) {
    let quest_items =
        GameObjectQuestItemStoreLikeCpp::from_rows_like_cpp(quest_item_rows, |_| true, |_| true)
            .store;
    session.set_object_mgr_catalogs_like_cpp(Arc::new(ObjectMgrCatalogsLikeCpp {
        gameobject: Arc::new(GameObjectQueryCatalogLikeCpp::from_rows_like_cpp(
            gameobjects,
            [],
        )),
        gameobject_quest_items: Arc::new(quest_items),
        ..Default::default()
    }));
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
    session.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
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
async fn gameobject_query_uses_typed_catalog_and_preserves_packet_projection_like_cpp() {
    let row = gameobject_query_catalog_row_like_cpp();
    let guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 0, 571, 0, 42, 99);
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    install_gameobject_query_catalog_like_cpp(
        &mut session,
        [row.clone()],
        [(42, 43, 0), (42, 44, 1)],
    );

    session
        .handle_query_game_object(QueryGameObject {
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

        session
            .handle_query_game_object(QueryGameObject {
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
