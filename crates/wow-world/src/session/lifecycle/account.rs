//! Session projection adapters for the existing finalization persistence obligations.
//! No transaction regrouping, retries or acknowledgement changes are introduced here.
use crate::finalization::FinalizationOutcome;
use crate::session::WorldSession;
use std::sync::Arc;
use tracing::{info, warn};
use wow_persistence::{
    AccountCollectionSaveLikeCpp, AccountHeirloomRowLikeCpp, AccountMaskBlockLikeCpp,
    AccountMountRowLikeCpp, AccountToyRowLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerOfflineMarkLikeCpp,
};

impl WorldSession {
    pub(crate) async fn clear_buyback_on_logout(&mut self) -> crate::FinalizationOutcome {
        let guid = match self.player_guid() {
            Some(g) => g,
            None => return crate::FinalizationOutcome::NoWork,
        };
        let Some(buyback_items) = self.resolved_buyback_items_like_cpp() else {
            return crate::FinalizationOutcome::Unavailable;
        };
        if buyback_items.is_empty() {
            self.clear_buyback_runtime_like_cpp();
            return crate::FinalizationOutcome::NoWork;
        }

        let port = match self.player_lifecycle_port_like_cpp().map(Arc::clone) {
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

        let Some(buyback_items) = self.resolved_buyback_items_like_cpp() else {
            return crate::FinalizationOutcome::Unavailable;
        };
        let removed_guids: Vec<_> = buyback_items.values().map(|item| item.guid).collect();
        for item_guid in removed_guids {
            self.remove_inventory_item_object(item_guid);
        }
        self.clear_buyback_runtime_like_cpp();
        crate::FinalizationOutcome::Applied
    }

    /// Mark the current character as offline (#200: through the lifecycle port).
    pub(crate) async fn mark_character_offline(&mut self) -> FinalizationOutcome {
        let Some(guid) = self.player_guid() else {
            return FinalizationOutcome::NoWork;
        };
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            return FinalizationOutcome::Unavailable;
        };

        let outcome = port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::Character {
                guid_low: guid.counter() as u32,
            })
            .await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                info!("Marked character offline for guid {}", guid.counter());
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to mark character offline: {reason}");
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("Character offline mark outcome is unknown: {reason}");
            }
        }
        outcome.into()
    }

    /// Trinity marks every character for the active account offline after
    /// `SMSG_LOGOUT_COMPLETE` because one account can only have one online
    /// character.  See C++ `WorldSession::LogoutPlayer`.
    pub(crate) async fn mark_character_account_offline_like_cpp(&mut self) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!(
                account = self.account_id,
                "Character account offline save skipped: lifecycle persistence port unavailable"
            );
            return FinalizationOutcome::Unavailable;
        };

        let outcome = port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::CharacterAccount {
                account_id: self.account_id,
            })
            .await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { rows } => {
                info!(
                    account = self.account_id,
                    rows, "Marked character account offline like C++"
                );
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    "Failed to mark character account offline like C++: {reason}"
                );
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id,
                    "Character account offline mark outcome is unknown: {reason}"
                );
            }
        }
        outcome.into()
    }

    /// Mark the account as offline in the login database when the whole
    /// WorldSession is being destroyed, matching C++ `WorldSession::~WorldSession`.
    pub(crate) async fn mark_login_account_offline_on_disconnect_like_cpp(
        &mut self,
    ) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!(
                account = self.account_id,
                "Disconnect account offline save skipped: lifecycle persistence port unavailable"
            );
            return FinalizationOutcome::Unavailable;
        };

        let outcome = port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::LoginAccount {
                account_id: self.account_id,
            })
            .await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                info!(
                    account = self.account_id,
                    "Marked login account offline on disconnect"
                );
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    "Failed to mark login account offline on disconnect: {reason}"
                );
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id,
                    "Login account offline mark outcome is unknown: {reason}"
                );
            }
        }
        outcome.into()
    }

    pub(crate) async fn save_account_mounts_like_cpp(&mut self) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return FinalizationOutcome::Unavailable;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account mount save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(rows) = self.account_mount_save_rows_like_cpp() else {
            return FinalizationOutcome::Unavailable;
        };
        let save = AccountCollectionSaveLikeCpp::Mounts(
            rows.into_iter()
                .map(|row| AccountMountRowLikeCpp {
                    bnet_account_id: row.bnet_account_id,
                    mount_spell_id: row.mount_spell_id,
                    flags: row.flags,
                })
                .collect(),
        );
        if save.is_empty() {
            return FinalizationOutcome::NoWork;
        }

        if let Some(operation) = &mut self.finalization {
            operation.retain_collection(save.clone());
        }
        let outcome = port.save_account_collection_like_cpp(save).await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account mount flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Account mount flags save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }

    pub(crate) async fn save_account_toys_like_cpp(&mut self) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return FinalizationOutcome::Unavailable;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account toy save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(rows) = self.account_toy_save_rows_like_cpp() else {
            return FinalizationOutcome::Unavailable;
        };
        let save = AccountCollectionSaveLikeCpp::Toys(
            rows.into_iter()
                .map(|row| AccountToyRowLikeCpp {
                    bnet_account_id: row.bnet_account_id,
                    item_id: row.item_id,
                    is_favorite: row.is_favorite,
                    has_fanfare: row.has_fanfare,
                })
                .collect(),
        );
        if save.is_empty() {
            return FinalizationOutcome::NoWork;
        }

        if let Some(operation) = &mut self.finalization {
            operation.retain_collection(save.clone());
        }
        let outcome = port.save_account_collection_like_cpp(save).await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account toy flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Account toy flags save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }

    pub(crate) async fn save_account_heirlooms_like_cpp(&mut self) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return FinalizationOutcome::Unavailable;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account heirloom save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(rows) = self.account_heirloom_save_rows_like_cpp() else {
            return FinalizationOutcome::Unavailable;
        };
        let save = AccountCollectionSaveLikeCpp::Heirlooms(
            rows.into_iter()
                .map(|row| AccountHeirloomRowLikeCpp {
                    bnet_account_id: row.bnet_account_id,
                    item_id: row.item_id,
                    flags: row.flags,
                })
                .collect(),
        );
        if save.is_empty() {
            return FinalizationOutcome::NoWork;
        }

        if let Some(operation) = &mut self.finalization {
            operation.retain_collection(save.clone());
        }
        let outcome = port.save_account_collection_like_cpp(save).await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account heirloom flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Account heirloom flags save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }

    pub(crate) async fn save_account_item_appearances_like_cpp(&mut self) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return FinalizationOutcome::Unavailable;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account appearance save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(plan) = self.account_item_appearance_save_plan_like_cpp() else {
            return FinalizationOutcome::Unavailable;
        };
        if plan.is_empty() {
            return FinalizationOutcome::NoWork;
        }

        let bnet_account_id = self.battlenet_account_id();
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

        if let Some(operation) = &mut self.finalization {
            operation.retain_collection(save.clone());
        }
        let outcome = port.save_account_collection_like_cpp(save).await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Failed to save account item appearances: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Account item appearance save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }

    pub(crate) async fn save_account_transmog_illusions_like_cpp(&mut self) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return FinalizationOutcome::Unavailable;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account illusion save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(plan) = self.account_transmog_illusion_save_plan_like_cpp() else {
            return FinalizationOutcome::Unavailable;
        };
        if plan.is_empty() {
            return FinalizationOutcome::NoWork;
        }

        let bnet_account_id = self.battlenet_account_id();
        let save = AccountCollectionSaveLikeCpp::TransmogIllusions {
            bnet_account_id,
            illusion_blocks: plan
                .illusion_blocks
                .into_iter()
                .map(|(block_index, mask)| AccountMaskBlockLikeCpp { block_index, mask })
                .collect(),
        };

        if let Some(operation) = &mut self.finalization {
            operation.retain_collection(save.clone());
        }
        let outcome = port.save_account_collection_like_cpp(save).await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Failed to save account transmog illusions: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Account transmog illusion save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }
}
