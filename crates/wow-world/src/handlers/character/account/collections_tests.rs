use crate::handlers::test_support::world::make_session;
use crate::session::WorldSession;
use crate::test_fixtures::CollectionLoadPortLikeCpp;
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuidGenerator};
use wow_packet::packets::misc::AccountMount;
use wow_packet::WorldPacket;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp, AccountHeirloomLoadRowLikeCpp,
    AccountMaskBlockLikeCpp,
    AccountMountLoadRowLikeCpp, AccountToyLoadRowLikeCpp,
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
    session.set_equipment_set_guid_generator_like_cpp(
        Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(1)),
    );
    (session, send_rx)
}

#[tokio::test]
async fn account_item_appearance_load_preserves_independent_query_failure_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([AccountCollectionLoadOutcomeLikeCpp::Loaded(
        AccountCollectionLoadedLikeCpp::ItemAppearances {
            appearance_blocks: AccountCollectionRowsLikeCpp::Failed {
                reason: "appearance read failed".to_owned(),
            },
            favorite_appearance_ids: AccountCollectionRowsLikeCpp::Loaded(vec![91]),
        },
    )]);
    let (mut session, _) = make_session();
    session.set_battlenet_account_id(77);
    session.set_player_lifecycle_port_like_cpp(port);

    session.load_account_item_appearances_like_cpp().await;

    assert!(
        session
            .account_transmog_active_player_rows_like_cpp()
            .is_empty()
    );
    assert!(!session.set_appearance_is_favorite_like_cpp(91, true));
}

#[tokio::test]
async fn account_collection_empty_and_adapter_failure_clear_represented_rows_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(
            Vec::new(),
        )),
        AccountCollectionLoadOutcomeLikeCpp::Failed {
            reason: "heirloom read failed".to_owned(),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.load_represented_account_toys_like_cpp([(42, true, false)]);
    session.load_represented_account_heirlooms_like_cpp([(43, 2)]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session.load_account_toys_like_cpp().await;
    session.load_account_heirlooms_like_cpp().await;

    assert!(session.account_toy_rows_like_cpp().is_empty());
    assert!(session.account_heirloom_rows_like_cpp().is_empty());
    assert_eq!(
        port.requests(),
        vec![
            AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id: 77
            },
        ]
    );
}

#[test]
fn login_known_spells_include_account_mounts_even_when_use_condition_fails_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_known_spells_like_cpp(vec![635]);
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 42,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 43,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    session.set_account_mounts_like_cpp(vec![
        AccountMount {
            spell_id: 100,
            flags: 0,
        },
        AccountMount {
            spell_id: 101,
            flags: 0,
        },
    ]);

    let login_spells = session.login_known_spells_after_account_collections_like_cpp();
    assert!(login_spells.contains(&635));
    assert!(login_spells.contains(&100));
    assert!(
        login_spells.contains(&101),
        "C++ CollectionMgr::AddMount stores/learns the mount before evaluating PlayerCondition; the condition applies to using it"
    );
}

#[tokio::test]
async fn account_collection_loads_cross_the_typed_port_in_login_order_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(vec![
            AccountToyLoadRowLikeCpp {
                item_id: -1,
                is_favorite: true,
                has_fanfare: true,
            },
            AccountToyLoadRowLikeCpp {
                item_id: 42,
                is_favorite: true,
                has_fanfare: false,
            },
        ])),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Heirlooms(
            vec![
                AccountHeirloomLoadRowLikeCpp {
                    item_id: -1,
                    flags: 1,
                },
                AccountHeirloomLoadRowLikeCpp {
                    item_id: 43,
                    flags: 2,
                },
            ],
        )),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(
            AccountCollectionLoadedLikeCpp::ItemAppearances {
                appearance_blocks: AccountCollectionRowsLikeCpp::Loaded(vec![
                    AccountMaskBlockLikeCpp {
                        block_index: 1,
                        mask: 2,
                    },
                ]),
                favorite_appearance_ids: AccountCollectionRowsLikeCpp::Loaded(vec![9]),
            },
        ),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(
            AccountCollectionLoadedLikeCpp::TransmogIllusions {
                illusion_blocks: vec![AccountMaskBlockLikeCpp {
                    block_index: 2,
                    mask: 4,
                }],
            },
        ),
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Mounts(vec![
            AccountMountLoadRowLikeCpp {
                mount_spell_id: -1,
                flags: 1,
            },
            AccountMountLoadRowLikeCpp {
                mount_spell_id: 123,
                flags: 2,
            },
        ])),
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session.load_account_toys_like_cpp().await;
    session.load_account_heirlooms_like_cpp().await;
    session.load_account_item_appearances_like_cpp().await;
    session.load_account_transmog_illusions_like_cpp().await;
    assert!(session.load_account_mounts_like_cpp().await);

    assert_eq!(session.account_toy_rows_like_cpp(), vec![(42, true, false)]);
    assert_eq!(session.account_heirloom_rows_like_cpp(), vec![(43, 2)]);
    assert_eq!(
        session.account_transmog_active_player_rows_like_cpp(),
        vec![0, 2]
    );
    assert!(!session.set_appearance_is_favorite_like_cpp(9, true));
    assert!(session.has_transmog_illusion_like_cpp(66));
    assert_eq!(
        session.account_mount_rows_like_cpp(),
        vec![AccountMount {
            spell_id: 123,
            flags: 2,
        }]
    );
    assert_eq!(
        port.requests(),
        vec![
            AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::ItemAppearances {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::TransmogIllusions {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Mounts {
                bnet_account_id: 77
            },
        ]
    );
}
