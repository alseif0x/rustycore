//! Commit, durability, finalization and rollback for represented Session state.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Commit a trainer fee when the represented cast has no durable
    /// spell/skill mutation (for example, every acquisition effect was
    /// suppressed by target immunity). C++ charges and publishes its trainer
    /// visuals before that triggered cast resolves its hit effects.
    pub(crate) async fn commit_exclusive_trainer_money_only_like_cpp(
        &mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        money_before: u64,
        money_after: u64,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
        #[cfg(test)]
        if let Some(success) = self.loot_money_persistence_test_result_like_cpp {
            return success.then_some(money_persistence);
        }

        if money_before == money_after {
            return Some(money_persistence);
        }
        let guid = self.player_guid()?.counter() as u64;
        let port = self.player_lifecycle_port_like_cpp().map(Arc::clone)?;
        let request = wow_persistence::PlayerMoneyTransactionRequestLikeCpp {
            player_guid: guid,
            money_after,
            durability_repairs: Vec::new(),
        };
        self.await_exclusive_player_money_transaction_outcome_like_cpp(
            money_persistence,
            port.persist_money_transaction_like_cpp(request),
            money_before,
            money_after,
            "trainer fee without durable acquisition mutation",
        )
        .await
    }
    /// Apply the already-committed void-storage unlock to runtime state and
    /// emit the same PlayerData::Flags values delta that C++ SetPlayerFlag does.
    pub(crate) fn apply_committed_void_storage_unlock_like_cpp(&mut self) {
        let values_update = self.player_values_update_snapshot().and_then(|mut player| {
            player.set_player_flag(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP);
            Some(player.values_update(true))
        });

        let Some(_current_flags) = self.represented_player_flags_value_like_cpp() else {
            return;
        };
        let _canonical = self
            .mutate_canonical_player_like_cpp(|player| {
                player.set_player_flag(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP);
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_loaded_player_flags_like_cpp =
                Some(_current_flags | PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP);
            self.represented_loaded_player_flags_applied_like_cpp = _canonical;
        }

        if let Some(update) = values_update {
            self.send_player_values_update_like_cpp(&update);
        }
    }
    pub(crate) async fn commit_represented_talent_reset_like_cpp(
        &mut self,
        no_reset_talent_cost: bool,
    ) -> Option<CommittedRepresentedTalentResetLikeCpp> {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.commit_represented_talent_reset_at_like_cpp(now_secs, no_reset_talent_cost)
            .await
    }
    pub(in crate::session) async fn commit_represented_talent_reset_at_like_cpp(
        &mut self,
        now_secs: u64,
        no_reset_talent_cost: bool,
    ) -> Option<CommittedRepresentedTalentResetLikeCpp> {
        let cost = if no_reset_talent_cost {
            0
        } else {
            u64::from(self.represented_next_reset_talents_cost_like_cpp(now_secs)?)
        };

        let money_persistence = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await?;
        let old_money = self.resolved_player_money_like_cpp()?;
        if old_money < cost {
            self.send_buy_error(BuyResult::NotEnoughtMoney, None, 0);
            return None;
        }
        let new_money = old_money - cost;
        let player_guid = self.player_guid()?;
        let (state_plan, persistence_request) = self
            .represented_talent_reset_persistence_plan_like_cpp(
                player_guid.counter() as u64,
                old_money,
                new_money,
                cost as u32,
                now_secs,
            )?;
        // Unit fixtures without a lifecycle port explicitly model a successful
        // COMMIT. The failure seam proves that no covered runtime state is
        // published on a definite rollback.
        #[cfg(test)]
        if self.loot_money_persistence_test_result_like_cpp == Some(false) {
            return None;
        }
        #[cfg(test)]
        let bypass_database_like_cpp = self.loot_money_persistence_test_result_like_cpp
            == Some(true)
            || self.player_lifecycle_port_like_cpp().is_none();
        #[cfg(not(test))]
        let bypass_database_like_cpp = false;

        let money_persistence = if bypass_database_like_cpp {
            money_persistence
        } else {
            let port = self.player_lifecycle_port_like_cpp().cloned()?;
            let mut cancellation_fence = PlayerMoneyCommitCancellationFenceLikeCpp::new(
                Arc::clone(&self.durable_loot_money_persistence_like_cpp),
            );
            match port
                .persist_talent_reset_like_cpp(persistence_request)
                .await
            {
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {
                    cancellation_fence.disarm_like_cpp();
                    money_persistence
                }
                wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => {
                    cancellation_fence.disarm_like_cpp();
                    warn!(%reason, operation = "talent reset", "player-money transaction definitely rolled back");
                    return None;
                }
                wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    self.durable_loot_money_persistence_like_cpp
                        .mark_indeterminate_like_cpp();
                    cancellation_fence.disarm_like_cpp();
                    self.kick(
                        "player-money COMMIT outcome is unknown; relog required before another money mutation",
                    );
                    warn!(
                        %reason,
                        operation = "talent reset",
                        money_before = old_money,
                        money_after = new_money,
                        "player-money COMMIT outcome remains indeterminate; quarantined the session"
                    );
                    return None;
                }
            }
        };

        Some(CommittedRepresentedTalentResetLikeCpp {
            money_persistence,
            old_money,
            new_money,
            cost: cost as u32,
            reset_time_secs: now_secs,
            state_plan,
        })
    }
    /// Publish every runtime effect covered by the committed reset before the
    /// shared money guard is released. There is deliberately no `.await`
    /// between entry and `drop(money_persistence)`: cancellation cannot expose
    /// a durable fee/talent reset with the old session state still live.
    pub(crate) async fn publish_committed_represented_talent_reset_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        committed: CommittedRepresentedTalentResetLikeCpp,
        request: RepresentedConfirmRespecWipeLikeCpp,
        visual_spell_id: u32,
    ) {
        let CommittedRepresentedTalentResetLikeCpp {
            money_persistence,
            old_money,
            new_money,
            cost,
            reset_time_secs,
            state_plan,
        } = committed;

        self.remove_represented_pet_not_in_slot_like_cpp();
        self.record_represented_confirm_respec_wipe_like_cpp(request);

        debug_assert_eq!(
            self.represented_active_talent_group_like_cpp(),
            Some(state_plan.active_group)
        );
        for (talent_id, rank) in &state_plan.active_talents {
            self.remove_represented_active_talent_side_effects_like_cpp(*talent_id, *rank);
        }
        if !self.install_reset_talent_groups_like_cpp(state_plan.post_talents.clone()) {
            self.kick("canonical Player talent owner became unavailable after talent-reset COMMIT");
            return;
        }
        self.refresh_represented_talent_points_like_cpp();

        if !self.stage_player_money_change_like_cpp(old_money, new_money) {
            self.kick("canonical Player money owner became unavailable after talent-reset COMMIT");
            return;
        }
        if !self.set_represented_talent_reset_state_like_cpp(cost, reset_time_secs) {
            self.kick("canonical Player specialization owner became unavailable after talent-reset COMMIT");
            return;
        }
        self.record_represented_talent_respec_criteria_like_cpp(cost);

        if let Some(talent_data) = self.resolved_update_talent_data_packet_like_cpp() {
            self.send_packet(&talent_data);
        }
        if let Some(player_guid) = self.player_guid() {
            self.record_represented_talent_respec_visual_spell_cast_like_cpp(
                RepresentedTalentRespecVisualSpellCastLikeCpp {
                    caster_guid: request.respec_master,
                    target_guid: player_guid,
                    spell_id: visual_spell_id,
                    triggered: true,
                    spell_runtime_unrepresented: true,
                },
            );
        }

        drop(money_persistence);
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
    }
    pub(crate) fn resolved_player_skill_non_durable_tombstones_like_cpp(
        &self,
    ) -> Option<BTreeSet<u16>> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.non_durable_skill_tombstones_like_cpp().clone()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_skill_non_durable_tombstones_like_cpp.clone());
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn player_skill_non_durable_tombstones_like_cpp(&self) -> BTreeSet<u16> {
        self.resolved_player_skill_non_durable_tombstones_like_cpp()
            .expect("test Player skill owner must resolve")
    }
}
