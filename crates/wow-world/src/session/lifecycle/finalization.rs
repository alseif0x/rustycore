//! Session-facing effects for the finalization application coordinator.
//! No transaction grouping or scheduling policy is defined by this adapter.
use crate::finalization::{
    FinalizationDisposition, FinalizationMode, FinalizationOutcome, FinalizationReport,
    FinalizationStep, SessionFinalization,
};
use crate::session::{PlayerSaveOutcomeLikeCpp, SessionState, WorldSession};
use wow_core::ObjectGuid;
use wow_packet::packets::misc::LogoutComplete;

impl WorldSession {
    pub fn finalization_report_like_cpp(&self) -> Option<FinalizationReport> {
        self.lifecycle
            .finalization
            .as_ref()
            .map(SessionFinalization::report)
    }

    /// Cancellation leaves an in-flight obligation, never a rollback receipt.
    pub fn interrupt_finalization_like_cpp(&mut self) -> Option<FinalizationReport> {
        if let Some(operation) = &mut self.lifecycle.finalization {
            operation.interrupt();
        }
        self.finalization_report_like_cpp()
    }

    fn finalization_result(&mut self) -> FinalizationReport {
        let report = self
            .lifecycle
            .finalization
            .as_ref()
            .expect("admitted finalization")
            .report();
        if report.disposition == FinalizationDisposition::RetainAndEscalate {
            self.kick("session finalization retained an unresolved obligation");
        }
        report
    }

    pub async fn finalize_session_with_generator_like_cpp(
        &mut self,
        mut mode: FinalizationMode,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> FinalizationReport {
        let no_player = self.player_guid().is_none();
        if let Some(previous) = &mut self.lifecycle.finalization {
            let report = previous.report();
            if previous.is_unstarted() && report.mode == FinalizationMode::TimedLogout {
                mode = FinalizationMode::TimedLogout;
            } else if mode == FinalizationMode::Disconnect
                && report.mode == FinalizationMode::CharacterSelection
                && report.outcome(FinalizationStep::NativeTransfer) == FinalizationOutcome::Deferred
                && report.outcome(FinalizationStep::CharacterSave)
                    == FinalizationOutcome::NotAttempted
            {
                // Completed terminal native recovery hands its source save to
                // disconnect, before any explicit-logout persistence submission.
            } else if !(report.disposition == FinalizationDisposition::Complete
                && report.mode == FinalizationMode::CharacterSelection
                && mode == FinalizationMode::Disconnect
                && no_player)
            {
                previous.interrupt();
                return self.finalization_result();
            }
        }
        self.lifecycle.finalization = Some(SessionFinalization::new(
            mode,
            !no_player,
            self.player_handle_like_cpp,
        ));

        while let Some(step) = self.lifecycle.finalization.as_ref().unwrap().next_step() {
            if !self.lifecycle.finalization.as_mut().unwrap().begin(step) {
                return self.finalization_result();
            }
            if !self.finalization_identity_is_current() {
                self.lifecycle
                    .finalization
                    .as_mut()
                    .unwrap()
                    .finish(step, FinalizationOutcome::Unavailable);
                return self.finalization_result();
            }
            let outcome = self
                .execute_finalization_step(step, mode, item_guid_generator)
                .await;
            if !self
                .lifecycle
                .finalization
                .as_mut()
                .unwrap()
                .finish(step, outcome)
            {
                return self.finalization_result();
            }
        }
        self.lifecycle.finalization.as_mut().unwrap().complete();
        self.finalization_result()
    }

    fn finalization_identity_is_current(&self) -> bool {
        let report = self.lifecycle.finalization.as_ref().unwrap().report();
        if report.outcome(FinalizationStep::Retirement) == FinalizationOutcome::Applied {
            return self.player_handle_like_cpp.is_none();
        }
        report.player == self.player_handle_like_cpp
            && report
                .player
                .is_none_or(|handle| self.player_guid() == Some(handle.guid()))
            && !(self.player_guid().is_none() && self.player_handle_like_cpp.is_some())
    }

    async fn execute_finalization_step(
        &mut self,
        step: FinalizationStep,
        mode: FinalizationMode,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> FinalizationOutcome {
        use FinalizationStep::*;
        match step {
            NativeTransfer => {
                if !self.finish_worldport_native_before_disconnect_like_cpp() {
                    return FinalizationOutcome::Unavailable;
                }
                if mode == FinalizationMode::CharacterSelection
                    && self.state() == SessionState::Disconnecting
                {
                    return FinalizationOutcome::Deferred;
                }
                if self.player_guid().is_some() {
                    self.set_player_logout_like_cpp(true);
                    self.lifecycle.logout_time = None;
                }
                FinalizationOutcome::Applied
            }
            LootSettlement => {
                self.wait_for_active_loot_persistence_with_generator_like_cpp(item_guid_generator)
                    .await;
                if let Some(guid) = self.player_guid() {
                    if self.has_active_loot_views_like_cpp() {
                        self.do_loot_release_all_like_cpp(guid).await;
                    }
                }
                FinalizationOutcome::Applied
            }
            Buyback => self.clear_buyback_on_logout().await,
            CharacterSave => match self
                .save_current_player_to_db_with_generator_like_cpp(item_guid_generator)
                .await
            {
                PlayerSaveOutcomeLikeCpp::Applied => FinalizationOutcome::Applied,
                PlayerSaveOutcomeLikeCpp::Failed => FinalizationOutcome::DefinitelyRolledBack,
                PlayerSaveOutcomeLikeCpp::Quarantined => FinalizationOutcome::Unknown,
                PlayerSaveOutcomeLikeCpp::Unavailable => FinalizationOutcome::Unavailable,
                PlayerSaveOutcomeLikeCpp::Deferred => FinalizationOutcome::Deferred,
            },
            Mounts => self.save_account_mounts_like_cpp().await,
            Toys => self.save_account_toys_like_cpp().await,
            Heirlooms => self.save_account_heirlooms_like_cpp().await,
            Appearances => self.save_account_item_appearances_like_cpp().await,
            Illusions => self.save_account_transmog_illusions_like_cpp().await,
            CharacterOffline => self.mark_character_offline().await,
            CharacterAccountOffline => self.mark_character_account_offline_like_cpp().await,
            LoginAccountOffline => {
                self.mark_login_account_offline_on_disconnect_like_cpp()
                    .await
            }
            Retirement => {
                self.unregister_from_player_registry();
                self.notify_other_players_visibility_changed_like_cpp();
                self.unregister_canonical_player_from_map_like_cpp()
            }
            LogoutPublication => {
                // Separate writers must finish the earlier instance response first.
                // A failed/cancelled fence cannot authorize completion publication.
                if !self
                    .wait_for_instance_send_before_realm_send_like_cpp()
                    .await
                {
                    return FinalizationOutcome::Unavailable;
                }
                // C++ Opcodes.cpp:1665 routes LogoutComplete to realm.
                // Saturation yields instead of blocking the executor thread.
                // Success proves channel acceptance, not client receipt.
                let bytes = wow_packet::ServerPacket::to_bytes(&LogoutComplete);
                match self.realm_route_tx().send_async(bytes).await {
                    Ok(()) => FinalizationOutcome::Applied,
                    Err(_) => FinalizationOutcome::Unavailable,
                }
            }
            Release => {
                if mode == FinalizationMode::CharacterSelection {
                    self.set_player_guid(None);
                    self.release_character_login_claim_like_cpp();
                    self.clear_all_inventory_runtime_like_cpp();
                    let _ = self.clear_player_currencies_like_cpp();
                    self.set_active_loot_guid(ObjectGuid::EMPTY);
                    self.restore_realm_channels();
                    self.set_player_logout_like_cpp(false);
                    self.set_state(SessionState::Authed);
                } else {
                    self.release_character_login_claim_like_cpp();
                    self.clear_inventory_items_and_objects_like_cpp();
                }
                FinalizationOutcome::Applied
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    #[cfg(feature = "test-fixtures")]
    use crate::canonical_player_access::install_canonical_player_owner_for_test;
    #[cfg(feature = "test-fixtures")]
    use std::sync::Arc;
    #[cfg(feature = "test-fixtures")]
    use crate::test_fixtures::CollectionLoadPortLikeCpp;
    #[cfg(feature = "test-fixtures")]
    use wow_core::{Position, guid::HighGuid};
    #[cfg(feature = "test-fixtures")]
    use wow_loot::{CreatureLoot, LootEntry, LootEntryFlags};
    #[cfg(feature = "test-fixtures")]
    use wow_packet::{
        WorldPacket, packets::loot::LOOT_TYPE_CORPSE_LIKE_CPP,
        packets::misc::LogoutRequest,
    };

    #[tokio::test]
    async fn logout_publication_uses_realm_and_preserves_backpressure_and_failure() {
        let (_, input) = flume::bounded(1);
        let (output, instance) = flume::bounded(1);
        let mut session = WorldSession::new(
            585,
            "LogoutRoute".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            input,
            output,
        );
        let (realm_tx, realm) = flume::bounded(1);
        realm_tx.send(vec![0]).unwrap();
        session.install_realm_send_channel_for_test(realm_tx);
        let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
        assert_eq!(
            session
                .execute_finalization_step(
                    FinalizationStep::LogoutPublication,
                    FinalizationMode::CharacterSelection,
                    &generator,
                )
                .await,
            FinalizationOutcome::Unavailable,
            "missing instance fence cannot authorize completion"
        );
        let fence = wow_network::SocketWriteFenceLikeCpp::default();
        session
            .transport
            .connection
            .set_send_write_fence_like_cpp(fence.clone());
        let mut publication = Box::pin(session.execute_finalization_step(
            FinalizationStep::LogoutPublication,
            FinalizationMode::CharacterSelection,
            &generator,
        ));
        assert!(
            std::future::poll_fn(|cx| {
                std::task::Poll::Ready(publication.as_mut().poll(cx).is_pending())
            })
            .await
        );
        let marker = instance.try_recv().expect("instance writer fence");
        assert_eq!(
            realm.len(),
            1,
            "completion cannot precede instance write acknowledgement"
        );
        assert!(fence.acknowledge_marker_like_cpp(&marker));
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                publication.as_mut().poll(cx).is_pending()
            ))
            .await,
            "realm backpressure remains after the instance fence"
        );
        assert_eq!(realm.try_recv().unwrap(), vec![0]);
        assert_eq!(publication.await, FinalizationOutcome::Applied);
        assert_eq!(
            realm.try_recv().unwrap(),
            wow_packet::ServerPacket::to_bytes(&LogoutComplete)
        );
        drop(realm);
        drop(instance);
        assert_eq!(
            session
                .execute_finalization_step(
                    FinalizationStep::LogoutPublication,
                    FinalizationMode::TimedLogout,
                    &generator,
                )
                .await,
            FinalizationOutcome::Unavailable
        );
    }

    #[cfg(feature = "test-fixtures")]
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
        session.set_item_guid_generator_like_cpp(Arc::new(
            wow_core::ObjectGuidGenerator::new(HighGuid::Item, 1),
        ));
        session.set_equipment_set_guid_generator_like_cpp(Arc::new(
            wow_core::EquipmentSetGuidGeneratorLikeCpp::new(1),
        ));
        (session, send_rx)
    }

    #[cfg(feature = "test-fixtures")]
    fn ensure_login_player_controller_for_test(
        session: &mut WorldSession,
        guid: ObjectGuid,
        name: String,
        position: Position,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) -> bool {
        session.ensure_login_player_controller_like_cpp(
            guid, name, position, map_id, race, class, level, gender,
        )
    }

    #[cfg(feature = "test-fixtures")]
    #[tokio::test]
    async fn logout_releases_active_loot_views_like_cpp_remove_from_world() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        session.set_player_lifecycle_port_like_cpp(CollectionLoadPortLikeCpp::new([]));
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 19_030);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        assert!(ensure_login_player_controller_for_test(
            &mut session,
            player_guid,
            "LogoutOwner".to_string(),
            Position::ZERO,
            1,
            1,
            1,
            10,
            0,
        ));
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
        assert!(session.try_claim_character_login_like_cpp(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_logout_request(LogoutRequest { idle_logout: false })
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootReleaseAll as u16
        );
        assert_eq!(sent.remaining(), 0);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LogoutResponse as u16
        );

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);

        assert!(
            send_rx.try_recv().is_err(),
            "failed persistence must not publish LogoutComplete"
        );
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(
            !session.loot_table.contains_key(&loot_guid),
            "loot release retires the packet-cache copy before persistence"
        );
        assert_eq!(session.player_guid(), Some(player_guid));
        assert_eq!(
            session.finalization_report_like_cpp().unwrap().disposition,
            crate::FinalizationDisposition::RetainAndEscalate
        );
        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(1, 0)
                .unwrap()
                .map()
                .get_typed_player(player_guid)
                .is_some(),
            "failed persistence retains the canonical Player"
        );
        let (mut replacement, _) = make_session_with_send_capacity(1);
        assert!(
            !replacement.try_claim_character_login_like_cpp(player_guid),
            "failed finalization retains its claim"
        );
        replacement.release_character_login_claim_like_cpp();
    }
}
