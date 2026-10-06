//! Session projection adapters for the existing finalization persistence obligations.
//! No transaction regrouping, retries or acknowledgement changes are introduced here.
use crate::finalization::FinalizationOutcome;
use crate::session::WorldSession;
use std::sync::Arc;
use tracing::warn;
use wow_persistence::{
    AccountCollectionSaveLikeCpp, AccountMaskBlockLikeCpp, PersistenceOutcomeLikeCpp,
};

impl WorldSession {
    pub(crate) async fn mark_character_offline(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.mark_character_offline(&mut hub).await
    }

    pub(crate) async fn mark_character_account_offline_like_cpp(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .mark_character_account_offline_like_cpp(&mut hub)
            .await
    }

    pub(crate) async fn mark_login_account_offline_on_disconnect_like_cpp(
        &mut self,
    ) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .mark_login_account_offline_on_disconnect_like_cpp(&mut hub)
            .await
    }

    pub(crate) async fn save_account_mounts_like_cpp(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.save_account_mounts_like_cpp(&mut hub).await
    }

    pub(crate) async fn save_account_toys_like_cpp(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.save_account_toys_like_cpp(&mut hub).await
    }

    pub(crate) async fn save_account_heirlooms_like_cpp(&mut self) -> FinalizationOutcome {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.save_account_heirlooms_like_cpp(&mut hub).await
    }
}

impl crate::session::LifecycleCx<'_> {
    pub(crate) async fn clear_buyback_on_logout(&mut self) -> crate::FinalizationOutcome {
        let guid = match self.hub.core.player_guid() {
            Some(g) => g,
            None => return crate::FinalizationOutcome::NoWork,
        };
        let Some(buyback_items) = self
            .inventory
            .resolved_buyback_items_like_cpp(self.hub.shared())
        else {
            return crate::FinalizationOutcome::Unavailable;
        };
        if buyback_items.is_empty() {
            self.inventory.clear_buyback_runtime_like_cpp(&mut self.hub);
            return crate::FinalizationOutcome::NoWork;
        }

        let port = match self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        {
            Some(port) => port,
            None => return crate::FinalizationOutcome::Unavailable,
        };
        let request = wow_persistence::PlayerBuybackClearRequestLikeCpp {
            player_guid: guid.counter() as u64,
            item_db_guids: buyback_items.values().map(|item| item.db_guid).collect(),
        };
        let outcome = port.clear_buyback_like_cpp(request).await;
        match &outcome {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    "Failed to clear buyback items on logout for guid {}: {reason}",
                    guid.counter()
                );
                return outcome.into();
            }
        }

        let Some(buyback_items) = self
            .inventory
            .resolved_buyback_items_like_cpp(self.hub.shared())
        else {
            return crate::FinalizationOutcome::Unavailable;
        };
        let removed_guids: Vec<_> = buyback_items.values().map(|item| item.guid).collect();
        for item_guid in removed_guids {
            self.inventory
                .remove_inventory_item_object(&mut self.hub, item_guid);
        }
        self.inventory.clear_buyback_runtime_like_cpp(&mut self.hub);
        crate::FinalizationOutcome::Applied
    }

    pub(crate) async fn save_account_item_appearances_like_cpp(&mut self) -> FinalizationOutcome {
        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            return FinalizationOutcome::Unavailable;
        };
        if self
            .hub
            .shared()
            .player_collection_state_snapshot_like_cpp()
            .is_none()
        {
            warn!(
                account = self.hub.core.account_id,
                "Skipping account appearance save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(plan) = self
            .inventory
            .account_item_appearance_save_plan_like_cpp(&mut self.hub)
        else {
            return FinalizationOutcome::Unavailable;
        };
        if plan.is_empty() {
            return FinalizationOutcome::NoWork;
        }

        let bnet_account_id = self.hub.core.battlenet_account_id();
        let save = AccountCollectionSaveLikeCpp::ItemAppearances {
            bnet_account_id,
            appearance_blocks: plan
                .appearance_blocks
                .into_iter()
                .map(|(block_index, mask)| AccountMaskBlockLikeCpp { block_index, mask })
                .collect(),
            favorite_inserts: plan.favorite_inserts,
            favorite_deletes: plan.favorite_deletes,
        };

        if let Some(operation) = self.lifecycle.finalization_mut() {
            operation.retain_collection(save.clone());
        }
        let outcome = port.save_account_collection_like_cpp(save).await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.hub.core.account_id,
                bnet_account = bnet_account_id,
                "Failed to save account item appearances: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.hub.core.account_id,
                bnet_account = bnet_account_id,
                "Account item appearance save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }

    pub(crate) async fn save_account_transmog_illusions_like_cpp(&mut self) -> FinalizationOutcome {
        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            return FinalizationOutcome::Unavailable;
        };
        if self
            .hub
            .shared()
            .player_collection_state_snapshot_like_cpp()
            .is_none()
        {
            warn!(
                account = self.hub.core.account_id,
                "Skipping account illusion save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(plan) = self
            .inventory
            .account_transmog_illusion_save_plan_like_cpp(self.hub.shared())
        else {
            return FinalizationOutcome::Unavailable;
        };
        if plan.is_empty() {
            return FinalizationOutcome::NoWork;
        }

        let bnet_account_id = self.hub.core.battlenet_account_id();
        let save = AccountCollectionSaveLikeCpp::TransmogIllusions {
            bnet_account_id,
            illusion_blocks: plan
                .illusion_blocks
                .into_iter()
                .map(|(block_index, mask)| AccountMaskBlockLikeCpp { block_index, mask })
                .collect(),
        };

        if let Some(operation) = self.lifecycle.finalization_mut() {
            operation.retain_collection(save.clone());
        }
        let outcome = port.save_account_collection_like_cpp(save).await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.hub.core.account_id,
                bnet_account = bnet_account_id,
                "Failed to save account transmog illusions: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.hub.core.account_id,
                bnet_account = bnet_account_id,
                "Account transmog illusion save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }
}
