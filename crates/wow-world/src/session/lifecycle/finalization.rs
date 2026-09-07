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
        self.finalization.as_ref().map(SessionFinalization::report)
    }

    /// Cancellation leaves an in-flight obligation, never a rollback receipt.
    pub fn interrupt_finalization_like_cpp(&mut self) -> Option<FinalizationReport> {
        if let Some(operation) = &mut self.finalization {
            operation.interrupt();
        }
        self.finalization_report_like_cpp()
    }

    fn finalization_result(&mut self) -> FinalizationReport {
        let report = self
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
        if let Some(previous) = &mut self.finalization {
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
        self.finalization = Some(SessionFinalization::new(
            mode,
            !no_player,
            self.player_handle_like_cpp,
        ));

        while let Some(step) = self.finalization.as_ref().unwrap().next_step() {
            if !self.finalization.as_mut().unwrap().begin(step) {
                return self.finalization_result();
            }
            if !self.finalization_identity_is_current() {
                self.finalization
                    .as_mut()
                    .unwrap()
                    .finish(step, FinalizationOutcome::Unavailable);
                return self.finalization_result();
            }
            let outcome = self
                .execute_finalization_step(step, mode, item_guid_generator)
                .await;
            if !self.finalization.as_mut().unwrap().finish(step, outcome) {
                return self.finalization_result();
            }
        }
        self.finalization.as_mut().unwrap().complete();
        self.finalization_result()
    }

    fn finalization_identity_is_current(&self) -> bool {
        let report = self.finalization.as_ref().unwrap().report();
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
                    self.logout_time = None;
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
                // Same channel and bytes as the previous send_packet call, but
                // saturation yields instead of blocking the executor thread.
                // Success proves channel acceptance, not client receipt.
                let bytes = wow_packet::ServerPacket::to_bytes(&LogoutComplete);
                match self.send_tx().send_async(bytes).await {
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
