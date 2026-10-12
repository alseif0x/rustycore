// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use rand::RngCore;
use tracing::warn;
use wow_world_core::session::PlayerMoneyTransactionSessionAccessLikeCpp;
use wow_world_lifecycle::{
    ExclusivePlayerMoneyPersistenceLikeCpp, PlayerMoneyCommitCancellationFenceLikeCpp,
    SessionLifecycleState,
};

use super::{
    PlayerSpellAcquisitionPersistenceOutcomeLikeCpp as Outcome,
    PreparedPlayerSpellAcquisitionLikeCpp, persist_player_spell_acquisition_through_port_like_cpp,
    player_spell_acquisition_persistence_request_like_cpp,
};

/// Commit one trainer fee and its prepared spell/skill acquisition while the
/// caller retains the exclusive money guard through publication.
pub async fn commit_exclusive_player_money_and_spell_acquisition_like_cpp(
    lifecycle: &SessionLifecycleState,
    core_access: PlayerMoneyTransactionSessionAccessLikeCpp<'_>,
    money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
    money_before: u64,
    money_after: u64,
    #[cfg(any(test, feature = "test-fixtures"))] fixture_result: Option<bool>,
) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(success) = fixture_result {
        return success.then_some(money_persistence);
    }

    let context = TrainerSpellAcquisitionCommitContextLikeCpp {
        lifecycle,
        core_access,
    };
    context
        .commit_like_cpp(money_persistence, prepared, money_before, money_after)
        .await
}

/// The narrow lifecycle and canonical-session participants held by one
/// trainer-acquisition commit. Its Core borrow intentionally lives through the
/// persistence await so quarantine remains in the same operation.
struct TrainerSpellAcquisitionCommitContextLikeCpp<'lifecycle, 'core> {
    lifecycle: &'lifecycle SessionLifecycleState,
    core_access: PlayerMoneyTransactionSessionAccessLikeCpp<'core>,
}

impl TrainerSpellAcquisitionCommitContextLikeCpp<'_, '_> {
    async fn commit_like_cpp(
        mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
        money_before: u64,
        money_after: u64,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
        let port = self
            .lifecycle
            .player_spell_acquisition_persistence_port_like_cpp()
            .cloned()?;
        let player_guid = self.core_access.player_guid()?;
        let guid_counter = player_guid.counter() as u64;
        let mut cancellation_fence = PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(
            self.lifecycle
                .durable_loot_money_persistence_tracker_like_cpp(),
        ));
        let mut operation_token = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut operation_token);
        let request = match player_spell_acquisition_persistence_request_like_cpp(
            guid_counter,
            prepared,
            money_before,
            money_after,
            operation_token,
        ) {
            Ok(request) => request,
            Err(error) => {
                cancellation_fence.disarm_like_cpp();
                warn!(%error, "trainer purchase request was not persistence-safe");
                return None;
            }
        };
        match persist_player_spell_acquisition_through_port_like_cpp(&*port, request).await {
            Outcome::Applied => {
                cancellation_fence.disarm_like_cpp();
                Some(money_persistence)
            }
            Outcome::DefinitelyRolledBack(reason) => {
                cancellation_fence.disarm_like_cpp();
                warn!(error = %reason, "trainer purchase transaction definitely rolled back");
                None
            }
            Outcome::ReconciledCommit(reason) => {
                cancellation_fence.disarm_like_cpp();
                warn!(error = %reason, "trainer COMMIT reply was lost but durable rows prove commit");
                Some(money_persistence)
            }
            Outcome::Indeterminate(reason) => {
                self.lifecycle
                    .durable_loot_money_persistence_tracker_like_cpp()
                    .mark_indeterminate_like_cpp();
                cancellation_fence.disarm_like_cpp();
                self.core_access.quarantine_like_cpp(
                    "trainer purchase COMMIT outcome is unknown; relog required",
                );
                warn!(error = %reason, "trainer COMMIT outcome remains indeterminate; session quarantined");
                None
            }
        }
    }
}
