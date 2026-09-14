//! Minimal character-screen composition; unexpected persistence is a failure.
use super::*;
use wow_persistence::*;

/// An authentic admission whose map owner is retired before delivery.
pub(super) fn retired_map_admission(
    session: &WorldSession,
) -> wow_world::session::mailbox::MapPhaseAdmissionLikeCpp {
    use wow_world::session::directory::{
        PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
        PlayerSessionRegistrationLikeCpp,
    };
    let guid = wow_core::ObjectGuid::create_player(1, 787);
    let registry = PlayerRegistry::new();
    let (send_tx, _sent) = flume::unbounded();
    let registration = registry.register_or_replace(
        guid,
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp::new("Retired", 787, 0, 1, 1, 0, 2),
            placement: PlayerDirectoryPlacementLikeCpp {
                map_id: 1,
                instance_id: 0,
                position: Default::default(),
                is_in_world: true,
                level: 1,
                is_alive: true,
            },
            active_loot_rolls: vec![],
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx: session.session_command_tx(),
            session_phase_tx: session.session_phase_sender_like_cpp(),
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            client_visible_transports_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        },
        Default::default(),
    );
    let map_key = wow_map::MapKey::new(1, 0);
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let mut player = Box::new(wow_entities::Player::new(Some(787), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, map_key, Default::default())
        .unwrap();
    let (_, residence_revision) = manager.current_player_admission_like_cpp(guid).unwrap();
    wow_world::session::mailbox::MapPhaseAdmissionLikeCpp {
        coordinator_id: 787,
        tick_epoch: 1,
        phase: wow_handler::PacketUpdatePhase::Map,
        registration,
        handle,
        map_key,
        map_incarnation: manager.map_incarnation_like_cpp(map_key).unwrap(),
        residence_revision,
        diff_ms: 1,
    }
}

pub(super) struct OfflinePort {
    pub(super) entered: flume::Sender<PlayerOfflineMarkLikeCpp>,
    pub(super) result: flume::Receiver<PersistenceOutcomeLikeCpp>,
    pub(super) destroyed: Arc<AtomicBool>,
}

impl Drop for OfflinePort {
    fn drop(&mut self) {
        self.destroyed.store(true, Ordering::Release);
    }
}

impl PlayerLifecyclePortLikeCpp for OfflinePort {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            self.entered.send(mark).unwrap();
            self.result.recv_async().await.unwrap()
        })
    }
    fn persist_homebind_like_cpp<'a>(
        &'a self,
        _request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_homebind_like_cpp")
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        _request: PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: clear_buyback_like_cpp")
    }

    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        _request: PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_money_transaction_like_cpp")
    }

    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        _request: PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_bank_slot_purchase_like_cpp")
    }

    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        _request: PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerUncageItemStateLoadOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: load_uncage_item_state_like_cpp")
    }

    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        _repair: PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_durability_repair_like_cpp")
    }

    fn persist_money_write_like_cpp<'a>(
        &'a self,
        _request: PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_money_write_like_cpp")
    }

    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        _request: PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_currency_save_like_cpp")
    }

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        _request: PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_talent_reset_like_cpp")
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        _request: PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_xp_like_cpp")
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        _request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: refresh_realm_character_count_like_cpp")
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: load_initial_world_states_like_cpp")
    }

    fn load_login_transports_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: load_login_transports_like_cpp")
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        _request: PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterBaseLoadOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: load_character_base_like_cpp")
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        _request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: load_account_collection_like_cpp")
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: load_login_admission_like_cpp")
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: load_login_auxiliary_like_cpp")
    }

    fn persist_login_item_repairs_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginItemRepairRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: persist_login_item_repairs_like_cpp")
    }

    fn reset_login_pet_talents_like_cpp<'a>(
        &'a self,
        _player_guid: u64,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginPetTalentResetOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: reset_login_pet_talents_like_cpp")
    }

    fn mark_player_online_like_cpp<'a>(
        &'a self,
        _request: PlayerOnlineMarkRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: mark_player_online_like_cpp")
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        _save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        panic!("unexpected lifecycle call: save_account_collection_like_cpp")
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        _request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        panic!("unexpected lifecycle call: save_character_like_cpp")
    }
}

pub(super) struct SessionHarness {
    pub(super) session: WorldSession,
    pub(super) registry: Arc<ActiveWorldSessionRegistryLikeCpp>,
    pub(super) registration: ActiveWorldSessionRegistrationGuardLikeCpp,
    pub(super) cancellation: Arc<ActiveWorldSessionCancellationLikeCpp>,
    pub(super) ready: Arc<AtomicBool>,
}

impl SessionHarness {
    pub(super) async fn drive(mut self, runtime: Arc<WorldRuntimeStateLikeCpp>) {
        let catalogs = wow_world::session::SessionHandlerCatalogsLikeCpp::default();
        let outcome = run_world_session_until_disconnect_like_cpp(
            &mut self.session,
            &catalogs,
            787,
            &self.registry,
            &self.cancellation,
            &self.ready,
        )
        .await;
        finalization::finalize_owned_world_session_like_cpp(
            self.session,
            outcome,
            787,
            self.registration,
            &runtime,
            catalogs.id_generators.item.as_ref(),
            Duration::from_secs(1),
        )
        .await;
    }
}

pub(super) fn session() -> (
    SessionHarness,
    flume::Sender<wow_packet::WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (packets, packet_rx) = flume::unbounded();
    let (send_tx, sent) = flume::unbounded();
    let session = WorldSession::new(
        787,
        "PhaseLifecycle".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "enUS".into(),
        packet_rx,
        send_tx,
    );
    let registry = Arc::new(ActiveWorldSessionRegistryLikeCpp::new());
    let (id, cancellation, ready) = registry
        .try_register(
            787,
            session.session_command_tx(),
            session.session_phase_sender_like_cpp(),
        )
        .unwrap();
    let registration = ActiveWorldSessionRegistrationGuardLikeCpp {
        registry: Arc::clone(&registry),
        id,
    };
    (
        SessionHarness {
            session,
            registry,
            registration,
            cancellation,
            ready,
        },
        packets,
        sent,
    )
}

pub(super) fn offline_port(
    session: &mut WorldSession,
) -> (
    flume::Receiver<PlayerOfflineMarkLikeCpp>,
    flume::Sender<PersistenceOutcomeLikeCpp>,
    Arc<AtomicBool>,
) {
    let (entered, started) = flume::bounded(1);
    let (reply, result) = flume::bounded(1);
    let destroyed = Arc::new(AtomicBool::new(false));
    session.set_player_lifecycle_port_like_cpp(Arc::new(OfflinePort {
        entered,
        result,
        destroyed: Arc::clone(&destroyed),
    }));
    (started, reply, destroyed)
}
