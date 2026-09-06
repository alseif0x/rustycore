//! Production-linked login regression: wow-world is compiled WITHOUT cfg(test).
//! C++ CharacterHandler.cpp:1065-1070 constructs Player before LoadFromDB;
//! Player.cpp:17748/17759 loads inventory/mail on that same Player.
//! Fixtures stop at PetStable or EquipmentInventory, after initial map selection.
//! It is deliberately not a complete login or database integration test.
//! Run in both dev and release: storage mutations must survive disabled debug assertions.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wow_core::{ObjectGuid, Position};
use wow_data::character_progression::PowerTypeStore;
use wow_data::*;
use wow_persistence::*;
use wow_world::{WorldSession, session::*};

struct LoginPort {
    save_probe: std::sync::Mutex<Option<Arc<save::SaveProbe>>>,
    reached_pet_load: AtomicBool,
    reached_inventory_load: AtomicBool,
    stop_at_pet_load: bool,
    manager: Arc<std::sync::Mutex<wow_map::MapManager>>,
}

// Unexpected persistence calls fail the test rather than silently succeeding.
impl PlayerLifecyclePortLikeCpp for LoginPort {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        _: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        if self.save_probe.lock().unwrap().is_some() {
            return Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } });
        }
        panic!("unexpected mark_offline_like_cpp");
    }
    fn persist_homebind_like_cpp<'a>(
        &'a self,
        _: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_homebind_like_cpp");
    }
    fn clear_buyback_like_cpp<'a>(
        &'a self,
        _: PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        if self.save_probe.lock().unwrap().is_some() {
            return Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } });
        }
        panic!("unexpected clear_buyback_like_cpp");
    }
    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        _: PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        panic!("unexpected persist_money_transaction_like_cpp");
    }
    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        _: PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        panic!("unexpected persist_bank_slot_purchase_like_cpp");
    }
    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        _: PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerUncageItemStateLoadOutcomeLikeCpp> {
        panic!("unexpected load_uncage_item_state_like_cpp");
    }
    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        _: PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_durability_repair_like_cpp");
    }
    fn persist_money_write_like_cpp<'a>(
        &'a self,
        _: PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_money_write_like_cpp");
    }
    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        _: PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_currency_save_like_cpp");
    }
    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        _: PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_talent_reset_like_cpp");
    }
    fn persist_xp_like_cpp<'a>(
        &'a self,
        _: PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_xp_like_cpp");
    }
    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        _: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected refresh_realm_character_count_like_cpp");
    }
    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp> {
        panic!("unexpected load_initial_world_states_like_cpp");
    }
    fn load_login_transports_like_cpp<'a>(
        &'a self,
        _: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp> {
        panic!("unexpected load_login_transports_like_cpp");
    }
    fn load_account_collection_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        // Deterministic interleaving while login waits for collection rows.
        self.manager.lock().unwrap().update(10);
        let rows = match request {
            AccountCollectionLoadRequestLikeCpp::Mounts { .. } => {
                AccountCollectionLoadedLikeCpp::Mounts(vec![])
            }
            AccountCollectionLoadRequestLikeCpp::Toys { .. } => {
                AccountCollectionLoadedLikeCpp::Toys(vec![])
            }
            AccountCollectionLoadRequestLikeCpp::Heirlooms { .. } => {
                AccountCollectionLoadedLikeCpp::Heirlooms(vec![])
            }
            AccountCollectionLoadRequestLikeCpp::ItemAppearances { .. } => {
                AccountCollectionLoadedLikeCpp::ItemAppearances {
                    appearance_blocks: AccountCollectionRowsLikeCpp::Loaded(vec![]),
                    favorite_appearance_ids: AccountCollectionRowsLikeCpp::Loaded(vec![]),
                }
            }
            AccountCollectionLoadRequestLikeCpp::TransmogIllusions { .. } => {
                AccountCollectionLoadedLikeCpp::TransmogIllusions {
                    illusion_blocks: vec![],
                }
            }
        };
        Box::pin(async move { AccountCollectionLoadOutcomeLikeCpp::Loaded(rows) })
    }
    fn persist_login_item_repairs_like_cpp<'a>(
        &'a self,
        _: PlayerLoginItemRepairRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected persist_login_item_repairs_like_cpp");
    }
    fn reset_login_pet_talents_like_cpp<'a>(
        &'a self,
        _: u64,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginPetTalentResetOutcomeLikeCpp> {
        panic!("unexpected reset_login_pet_talents_like_cpp");
    }
    fn mark_player_online_like_cpp<'a>(
        &'a self,
        _: PlayerOnlineMarkRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected mark_player_online_like_cpp");
    }
    fn save_account_collection_like_cpp<'a>(
        &'a self,
        _: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        if self.save_probe.lock().unwrap().is_some() {
            return Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } });
        }
        panic!("unexpected save_account_collection_like_cpp");
    }
    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        save::save(self, request)
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        _: PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterBaseLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(PlayerCharacterBaseLoadRowLikeCpp {
                name: "Bootstrap".into(),
                race: 1,
                class: 1,
                gender: 0,
                level: 1,
                xp: Some(17),
                money: Some(123),
                inventory_slots: None,
                bank_slots: None,
                rest_state: None,
                player_flags: None,
                player_flags_ex: None,
                position_x: Some(1.0),
                position_y: Some(2.0),
                position_z: Some(3.0),
                map_id: Some(0),
                orientation: Some(0.0),
                create_mode: Some(0),
                total_played_time: None,
                level_played_time: None,
                rest_bonus: None,
                logout_time_secs: None,
                logout_was_resting: None,
                talent_reset_cost: None,
                talent_reset_time_secs: None,
                active_talent_group: None,
                bonus_talent_groups: None,
                transport_x: None,
                transport_y: None,
                transport_z: None,
                transport_orientation: None,
                transport_guid_low: None,
                summoned_pet_number: None,
                at_login_flags: Some(0),
                zone_id: Some(12),
                dungeon_difficulty: None,
                chosen_title: None,
                health: Some(100),
                powers: [None; 10],
                explored_zones: String::new(),
                known_titles: None,
                raid_difficulty: None,
                legacy_raid_difficulty: None,
            }))
        })
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        let loaded = match request {
            PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation { .. } => {
                PlayerLoginAdmissionLoadedLikeCpp::HomebindLocation(Some(
                    PlayerHomebindLocationLoadRowLikeCpp {
                        map_id: Some(0),
                        area_id: Some(12),
                        x: Some(1.0),
                        y: Some(2.0),
                        z: Some(3.0),
                        orientation: Some(0.0),
                    },
                ))
            }
            PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership { .. } => {
                PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(vec![])
            }
            _ => panic!("unexpected admission request"),
        };
        Box::pin(async move { PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(loaded) })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        match request {
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Mail { .. } => Box::pin(async {
                PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    PlayerLoginAuxiliaryLoadedLikeCpp::Mail(vec![]),
                )
            }),
            PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable { .. } => {
                self.reached_pet_load.store(true, Ordering::SeqCst);
                if !self.stop_at_pet_load {
                    return Box::pin(async {
                        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                            PlayerLoginAuxiliaryLoadedLikeCpp::PetStable(vec![]),
                        )
                    });
                }
                // Yield indefinitely so the test can cancel here. No late-login
                // fixture, map publication, timers or persistent writes are needed.
                Box::pin(std::future::pending())
            }
            PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership { .. } => Box::pin(async {
                PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(vec![]),
                )
            }),
            PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory { .. } => {
                self.reached_inventory_load.store(true, Ordering::SeqCst);
                Box::pin(std::future::pending())
            }
            _ => panic!("unexpected auxiliary request"),
        }
    }
}

async fn exercise_initial_hydration(install_manager: bool, stop_at_pet_load: bool) {
    let _ = hydrate(install_manager, stop_at_pet_load, false).await;
}

async fn hydrate(
    install_manager: bool,
    stop_at_pet_load: bool,
    preinsert_player: bool,
) -> (
    WorldSession,
    Arc<LoginPort>,
    flume::Sender<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (_, packet_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(8);
    let output = send_tx.clone();
    let mut session = WorldSession::new(
        1,
        "fixture".into(),
        0,
        2,
        2,
        54261,
        vec![],
        "enUS".into(),
        packet_rx,
        send_tx,
    );
    let guid = ObjectGuid::create_player(1, 42);
    let manager = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(300_000, 10)));
    if preinsert_player {
        let mut player = wow_entities::Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(0, 0).unwrap();
        player.unit_mut().world_mut().object_mut().add_to_world();
        manager
            .lock()
            .unwrap()
            .create_world_map(0, 0)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    let port = Arc::new(LoginPort {
        save_probe: std::sync::Mutex::new(None),
        reached_pet_load: AtomicBool::new(false),
        reached_inventory_load: AtomicBool::new(false),
        stop_at_pet_load,
        manager: Arc::clone(&manager),
    });
    let maps = Arc::new(MapStore::from_entries([MapEntry {
        id: 0,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));
    session.set_map_store(Arc::clone(&maps));
    if install_manager {
        session.set_canonical_map_manager(manager);
    }
    session.set_player_lifecycle_port_like_cpp(port.clone());
    session.set_player_loading(Some(guid));
    let bootstrap = PlayerBootstrapCatalogsLikeCpp {
        create_info: Arc::new(PlayerCreateInfoStoreLikeCpp::from_rows_like_cpp(
            [PlayerCreateInfoRowLikeCpp {
                race: 1,
                class: 1,
                create_position: PlayerCreatePositionLikeCpp {
                    map_id: 0,
                    position: Position::new(1.0, 2.0, 3.0, 0.0),
                    transport_guid: None,
                },
                create_position_npe: None,
                npe_transport_template_valid: true,
            }],
            &maps,
            |_| true,
            |_| true,
            |_| true,
        )),
        cast_spells: Arc::new(PlayerCreateInfoCastSpellStoreLikeCpp::default()),
        glyph_properties: Arc::new(wow_data::GlyphPropertiesStore::from_entries([])),
        talent_tabs: Arc::new(wow_data::TalentTabStore::from_entries([])),
        trait_node_entries: Arc::new(wow_data::trait_tree::TraitNodeEntryStore::from_entries([])),
        custom_spells: Arc::new(PlayerCreateInfoCustomSpellStoreLikeCpp::default()),
        start_all_spells: false,
        start_all_explored: false,
        start_all_reputation: false,
    };
    let creatures = CreatureSpawnCatalogsLikeCpp {
        difficulty: Arc::new(CreatureDifficultyStoreLikeCpp::default()),
        base_stats: Arc::new(CreatureBaseStatsStoreLikeCpp::default()),
        health_rates: CreatureClassificationHealthRatesLikeCpp::default(),
        addons: Arc::new(CreatureAddonStoreLikeCpp::default()),
        equipment: Arc::new(CreatureEquipmentStoreLikeCpp::default()),
        power_types: Arc::new(PowerTypeStore::from_entries([])),
    };
    let progression = ProgressionCatalogsLikeCpp {
        no_reset_talent_cost: false,
        player_xp: Arc::new(vec![0, 400]),
        exploration_base_xp: Arc::new(ExplorationBaseXpStoreLikeCpp::default()),
        exploration_xp_rate: 1.0,
        min_discovered_scaled_xp_ratio: 0,
    };
    let grid: PlayerGridLoadResolverLikeCpp =
        Arc::new(|_, _, _| panic!("unexpected grid publication"));
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let modules = wow_module_api::ModuleRegistry::new();
    let rest = PlayerRestRatePolicyLikeCpp::default();
    let features = SupportFeaturePolicyLikeCpp::default();
    let mut login = Box::pin(
        session.handle_continue_player_login_with_module_registry_like_cpp(
            &generator,
            &modules,
            &creatures,
            &bootstrap,
            &rest,
            &progression,
            &features,
            &grid,
        ),
    );
    // Poll once: early fixture reads are ready; the selected checkpoint is pending.
    let pending =
        std::future::poll_fn(|cx| std::task::Poll::Ready(login.as_mut().poll(cx).is_pending()))
            .await;
    drop(login);
    assert_eq!(
        port.reached_pet_load.load(Ordering::SeqCst),
        install_manager,
        "initial mail/scalar hydration must resolve the production Player owner"
    );
    assert_eq!(
        port.reached_inventory_load.load(Ordering::SeqCst),
        install_manager && !stop_at_pet_load,
        "map selection must preserve the Player for currency/inventory hydration"
    );
    assert_eq!(pending, install_manager);
    assert!(
        send_rx.is_empty(),
        "no world-entry success may be published during this phase"
    );
    (session, port, output, send_rx)
}

#[path = "production_login_player_owner/cleanup.rs"]
mod cleanup;
#[path = "production_login_player_owner/save.rs"]
mod save;
#[path = "production_login_player_owner/teleport_admission.rs"]
mod teleport_admission;

#[tokio::test]
async fn production_login_constructs_player_before_inventory_and_mail_hydration() {
    exercise_initial_hydration(true, true).await;
}

#[tokio::test]
async fn production_login_without_player_manager_does_not_continue_hydration() {
    exercise_initial_hydration(false, true).await;
}

#[tokio::test]
async fn production_login_preserves_player_through_initial_map_selection() {
    exercise_initial_hydration(true, false).await;
}
