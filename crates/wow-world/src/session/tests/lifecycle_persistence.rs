// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Session publishes offline state through the typed lifecycle port.
//!
//! These drive the real production methods against a recording port, so they
//! pin which marks are requested, against which logical database, and that
//! every outcome class is handled without panicking.

use super::*;

use std::sync::Mutex;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionSaveLikeCpp, AccountMaskBlockLikeCpp, LogicalDatabaseLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerBuybackClearRequestLikeCpp,
    PlayerCharacterSaveRequestLikeCpp, PlayerCharacterSaveResultLikeCpp,
    PlayerHomebindPersistenceRequestLikeCpp, PlayerLifecyclePortLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerOfflineMarkLikeCpp, PlayerRealmCharacterCountRefreshRequestLikeCpp,
};

struct RecordingPortLikeCpp {
    seen: Mutex<Vec<PlayerOfflineMarkLikeCpp>>,
    collection_loads: Mutex<Vec<AccountCollectionLoadRequestLikeCpp>>,
    collections: Mutex<Vec<AccountCollectionSaveLikeCpp>>,
    character_saves: Mutex<Vec<PlayerCharacterSaveRequestLikeCpp>>,
    buyback_clears: Mutex<Vec<PlayerBuybackClearRequestLikeCpp>>,
    realm_character_count_refreshes: Mutex<Vec<PlayerRealmCharacterCountRefreshRequestLikeCpp>>,
    outcome: PersistenceOutcomeLikeCpp,
}

impl RecordingPortLikeCpp {
    fn new(outcome: PersistenceOutcomeLikeCpp) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            collection_loads: Mutex::new(Vec::new()),
            collections: Mutex::new(Vec::new()),
            character_saves: Mutex::new(Vec::new()),
            buyback_clears: Mutex::new(Vec::new()),
            realm_character_count_refreshes: Mutex::new(Vec::new()),
            outcome,
        })
    }
    fn marks(&self) -> Vec<PlayerOfflineMarkLikeCpp> {
        self.seen.lock().unwrap().clone()
    }
    fn collection_saves(&self) -> Vec<AccountCollectionSaveLikeCpp> {
        self.collections.lock().unwrap().clone()
    }
    fn character_saves(&self) -> Vec<PlayerCharacterSaveRequestLikeCpp> {
        self.character_saves.lock().unwrap().clone()
    }
    fn buyback_clears(&self) -> Vec<PlayerBuybackClearRequestLikeCpp> {
        self.buyback_clears.lock().unwrap().clone()
    }
    fn realm_character_count_refreshes(
        &self,
    ) -> Vec<PlayerRealmCharacterCountRefreshRequestLikeCpp> {
        self.realm_character_count_refreshes.lock().unwrap().clone()
    }
}

impl PlayerLifecyclePortLikeCpp for RecordingPortLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.seen.lock().unwrap().push(mark);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        _request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        request: PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.buyback_clears.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.realm_character_count_refreshes
            .lock()
            .unwrap()
            .push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerInitialWorldStatesLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates: wow_persistence::PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "recording port has no initial-world-state fixture".to_owned(),
                },
                saved_values: wow_persistence::PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "recording port has no initial-world-state fixture".to_owned(),
                },
            }
        })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        self.collection_loads.lock().unwrap().push(request);
        Box::pin(async {
            AccountCollectionLoadOutcomeLikeCpp::Failed {
                reason: "recording port has no collection load fixture".to_owned(),
            }
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "recording port has no auxiliary login fixture".to_owned(),
            }
        })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.collections.lock().unwrap().push(save);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        let committed = request.committed_groups_like_cpp();
        self.character_saves.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { PlayerCharacterSaveResultLikeCpp { outcome, committed } })
    }
}

fn session_with_port(
    outcome: PersistenceOutcomeLikeCpp,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, _, _) = make_session();
    let port = RecordingPortLikeCpp::new(outcome);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    (session, port)
}

fn character_save_session_with_port(
    outcome: PersistenceOutcomeLikeCpp,
    guid_counter: i64,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, port) = session_with_port(outcome);
    session.set_player_guid(Some(ObjectGuid::create_player(1, guid_counter)));
    session.set_state(SessionState::LoggedIn);
    session.set_player_map_position_like_cpp(
        0,
        Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: 0.5,
        },
    );
    session.tutorials_changed_like_cpp = true;
    session.tutorials_loaded_coherently_like_cpp = true;
    (session, port)
}

#[tokio::test]
async fn character_save_reaches_the_sqlx_free_port_and_cleans_only_after_apply_like_cpp() {
    let (mut session, port) = character_save_session_with_port(
        PersistenceOutcomeLikeCpp::Applied { rows: 12 },
        0x7500_0001,
    );

    session.save_current_player_to_db_like_cpp().await;

    let saves = port.character_saves();
    assert_eq!(saves.len(), 1);
    assert!(saves[0].tutorials.is_some());
    assert_eq!(saves[0].player_guid, 0x7500_0001);
    assert!(!session.tutorials_changed_like_cpp);
    assert!(session.tutorials_loaded_from_db_like_cpp);
}

#[tokio::test]
async fn definite_character_save_rollback_preserves_dirty_state_like_cpp() {
    let (mut session, port) = character_save_session_with_port(
        PersistenceOutcomeLikeCpp::Failed {
            reason: "constraint failure before COMMIT".to_owned(),
        },
        0x7500_0002,
    );

    session.save_current_player_to_db_like_cpp().await;

    assert_eq!(port.character_saves().len(), 1);
    assert!(session.tutorials_changed_like_cpp);
    assert!(!session.tutorials_loaded_from_db_like_cpp);
    assert!(
        !session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn unknown_character_save_commit_fences_and_preserves_dirty_state_like_cpp() {
    let (mut session, port) = character_save_session_with_port(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "connection lost after COMMIT".to_owned(),
        },
        0x7500_0003,
    );

    session.save_current_player_to_db_like_cpp().await;

    assert_eq!(port.character_saves().len(), 1);
    assert!(session.tutorials_changed_like_cpp);
    assert!(!session.tutorials_loaded_from_db_like_cpp);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

fn represented_buyback_item_like_cpp(db_guid: u64) -> InventoryItem {
    InventoryItem {
        guid: ObjectGuid::create_item(1, db_guid as i64),
        entry_id: 25,
        db_guid,
        inventory_type: None,
    }
}

#[tokio::test]
async fn logout_buyback_clear_reaches_port_and_publishes_only_after_apply_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0101)));
    session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0001));

    session.clear_buyback_on_logout().await;

    assert_eq!(
        port.buyback_clears(),
        vec![PlayerBuybackClearRequestLikeCpp {
            player_guid: 0x7500_0101,
            item_db_guids: vec![0x8100_0001],
        }]
    );
    assert!(session.buyback_items_like_cpp().is_empty());
}

#[tokio::test]
async fn logout_buyback_clear_preserves_runtime_for_failed_and_unknown_durability_like_cpp() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rolled back before COMMIT".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "connection lost after COMMIT".to_owned(),
        },
    ] {
        let (mut session, port) = session_with_port(outcome);
        session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0102)));
        session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0002));

        session.clear_buyback_on_logout().await;

        assert_eq!(port.buyback_clears().len(), 1);
        assert!(session.buyback_items_like_cpp().contains_key(&94));
    }
}

#[tokio::test]
async fn logout_buyback_clear_without_port_does_not_fabricate_durable_success_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0103)));
    session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0003));

    session.clear_buyback_on_logout().await;

    assert!(session.buyback_items_like_cpp().contains_key(&94));
}

#[tokio::test]
async fn realm_character_count_refresh_reaches_the_lifecycle_port_without_database_handles() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_realm_id(12);

    session.update_realm_characters().await;

    assert_eq!(
        port.realm_character_count_refreshes(),
        vec![PlayerRealmCharacterCountRefreshRequestLikeCpp {
            account_id: 1,
            realm_id: 12,
        }]
    );
}

#[tokio::test]
async fn realm_character_count_refresh_without_port_does_not_fabricate_a_request() {
    let (session, _, _) = make_session();
    session.update_realm_characters().await;
}

/// Each offline mark reaches the port as its own request, naming the logical
/// database it belongs to — the characters writes and the login write are not
/// collapsed into one call.
#[tokio::test]
async fn logout_publishes_each_offline_mark_through_the_port_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let guid = ObjectGuid::create_player(1, 0x7200_0001);
    session.set_player_guid(Some(guid));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;
    session
        .mark_login_account_offline_on_disconnect_like_cpp()
        .await;

    let marks = port.marks();
    assert_eq!(
        marks,
        vec![
            PlayerOfflineMarkLikeCpp::Character {
                guid_low: guid.counter() as u32
            },
            PlayerOfflineMarkLikeCpp::CharacterAccount {
                account_id: session.account_id
            },
            PlayerOfflineMarkLikeCpp::LoginAccount {
                account_id: session.account_id
            },
        ]
    );
    assert_eq!(
        marks
            .iter()
            .map(|m| m.logical_database())
            .collect::<Vec<_>>(),
        vec![
            LogicalDatabaseLikeCpp::Characters,
            LogicalDatabaseLikeCpp::Characters,
            LogicalDatabaseLikeCpp::Login,
        ]
    );
}

/// With no selected character there is nothing to mark offline, so the port is
/// not called at all.
#[tokio::test]
async fn no_character_means_no_character_offline_request_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    assert!(session.player_guid().is_none());

    session.mark_character_offline().await;

    assert!(port.marks().is_empty());
}

/// A failed write is reported, not retried and not escalated.
#[tokio::test]
async fn a_failed_offline_mark_is_handled_without_panicking_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Failed {
        reason: "connection refused".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0002)));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;

    assert_eq!(port.marks().len(), 2);
}

/// An indeterminate outcome is a distinct class the caller must see; it is not
/// silently treated as success or as rollback.
#[tokio::test]
async fn an_unknown_offline_mark_outcome_is_handled_distinctly_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Unknown {
        reason: "connection lost after the write was sent".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0003)));

    session.mark_character_offline().await;

    assert_eq!(port.marks().len(), 1);
    assert!(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "x".to_owned()
        }
        .is_indeterminate()
    );
}

/// A session with no port installed performs no durable write and does not
/// panic: unit sessions and tests never reach a database.
#[tokio::test]
async fn a_session_without_a_port_performs_no_durable_write_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0004)));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;
    session
        .mark_login_account_offline_on_disconnect_like_cpp()
        .await;
}

/// Each account collection reaches the port as its own request against the
/// Login database. Three requests, not one: C++ logout commits them
/// separately, and #187 freezes that until a deliberate behaviour change.
#[tokio::test]
async fn account_collections_are_saved_as_separate_login_requests_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7300_0001)));

    session.save_account_mounts_like_cpp().await;
    session.save_account_toys_like_cpp().await;
    session.save_account_heirlooms_like_cpp().await;

    for save in port.collection_saves() {
        assert_eq!(
            save.logical_database(),
            LogicalDatabaseLikeCpp::Login,
            "account collections belong to the login database"
        );
        assert!(!save.is_empty(), "an empty collection must not be sent");
    }
}

/// An empty collection opens no transaction.
#[tokio::test]
async fn empty_account_collections_are_not_sent_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 0 });

    session.save_account_mounts_like_cpp().await;
    session.save_account_toys_like_cpp().await;
    session.save_account_heirlooms_like_cpp().await;

    assert!(port.collection_saves().is_empty());
}

/// With no port installed nothing is written and nothing panics.
#[tokio::test]
async fn account_collections_without_a_port_write_nothing_like_cpp() {
    let (mut session, _, _) = make_session();

    session.save_account_mounts_like_cpp().await;
    session.save_account_toys_like_cpp().await;
    session.save_account_heirlooms_like_cpp().await;
}

/// Appearances keep their favourite inserts ahead of their deletes. They share
/// one transaction, and a delete that overtook its insert would drop a
/// favourite the client still shows.
#[test]
fn appearance_saves_keep_inserts_before_deletes_like_cpp() {
    let save = AccountCollectionSaveLikeCpp::ItemAppearances {
        bnet_account_id: 7,
        appearance_blocks: vec![AccountMaskBlockLikeCpp {
            block_index: 0,
            mask: 0b1011,
        }],
        favorite_inserts: vec![101, 102],
        favorite_deletes: vec![103],
    };

    assert!(!save.is_empty());
    assert_eq!(save.logical_database(), LogicalDatabaseLikeCpp::Login);
    match save {
        AccountCollectionSaveLikeCpp::ItemAppearances {
            favorite_inserts,
            favorite_deletes,
            ..
        } => {
            assert_eq!(favorite_inserts, vec![101, 102]);
            assert_eq!(favorite_deletes, vec![103]);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

/// An appearance plan with no blocks and no favourite changes writes nothing.
#[test]
fn an_empty_appearance_plan_opens_no_transaction_like_cpp() {
    assert!(
        AccountCollectionSaveLikeCpp::ItemAppearances {
            bnet_account_id: 7,
            appearance_blocks: Vec::new(),
            favorite_inserts: Vec::new(),
            favorite_deletes: Vec::new(),
        }
        .is_empty()
    );
    assert!(
        AccountCollectionSaveLikeCpp::TransmogIllusions {
            bnet_account_id: 7,
            illusion_blocks: Vec::new(),
        }
        .is_empty()
    );
}

/// Appearances and illusions reach the port without a database handle.
#[tokio::test]
async fn appearances_and_illusions_are_saved_through_the_port_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7400_0001)));

    session.save_account_item_appearances_like_cpp().await;
    session.save_account_transmog_illusions_like_cpp().await;

    for save in port.collection_saves() {
        assert_eq!(save.logical_database(), LogicalDatabaseLikeCpp::Login);
    }
}
