use std::sync::Arc;

use tracing::{info, warn};
use wow_persistence::{
    AccountCollectionSaveLikeCpp, AccountHeirloomRowLikeCpp, AccountMountRowLikeCpp,
    AccountToyRowLikeCpp, PersistenceOutcomeLikeCpp, PlayerOfflineMarkLikeCpp,
};
use wow_world_core::session::HubMut;

use super::SessionLifecycleState;
use crate::{
    AccountHeirloomSaveRowLikeCpp, AccountMountSaveRowLikeCpp, AccountToySaveRowLikeCpp,
    FinalizationOutcome,
};

impl SessionLifecycleState {
    /// Mark the current character as offline (#200: through the lifecycle port).
    pub async fn mark_character_offline(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> FinalizationOutcome {
        let Some(guid) = hub.core.player_guid() else {
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
    pub async fn mark_character_account_offline_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!(
                account = hub.core.account_id,
                "Character account offline save skipped: lifecycle persistence port unavailable"
            );
            return FinalizationOutcome::Unavailable;
        };

        let outcome = port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::CharacterAccount {
                account_id: hub.core.account_id,
            })
            .await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { rows } => {
                info!(
                    account = hub.core.account_id,
                    rows, "Marked character account offline like C++"
                );
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = hub.core.account_id,
                    "Failed to mark character account offline like C++: {reason}"
                );
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = hub.core.account_id,
                    "Character account offline mark outcome is unknown: {reason}"
                );
            }
        }
        outcome.into()
    }

    /// Mark the account as offline in the login database when the whole
    /// WorldSession is being destroyed, matching C++ `WorldSession::~WorldSession`.
    pub async fn mark_login_account_offline_on_disconnect_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!(
                account = hub.core.account_id,
                "Disconnect account offline save skipped: lifecycle persistence port unavailable"
            );
            return FinalizationOutcome::Unavailable;
        };

        let outcome = port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::LoginAccount {
                account_id: hub.core.account_id,
            })
            .await;
        match &outcome {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                info!(
                    account = hub.core.account_id,
                    "Marked login account offline on disconnect"
                );
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = hub.core.account_id,
                    "Failed to mark login account offline on disconnect: {reason}"
                );
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = hub.core.account_id,
                    "Login account offline mark outcome is unknown: {reason}"
                );
            }
        }
        outcome.into()
    }

    pub async fn save_account_mounts_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return FinalizationOutcome::Unavailable;
        };
        if hub
            .shared()
            .player_collection_state_snapshot_like_cpp()
            .is_none()
        {
            warn!(
                account = hub.core.account_id,
                "Skipping account mount save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(rows) = self.account_mount_save_rows_like_cpp(hub.shared()) else {
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
                account = hub.core.account_id,
                bnet_account = hub.core.battlenet_account_id(),
                "Failed to save account mount flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = hub.core.account_id,
                bnet_account = hub.core.battlenet_account_id(),
                "Account mount flags save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }

    pub async fn save_account_toys_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return FinalizationOutcome::Unavailable;
        };
        if hub
            .shared()
            .player_collection_state_snapshot_like_cpp()
            .is_none()
        {
            warn!(
                account = hub.core.account_id,
                "Skipping account toy save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(rows) = self.account_toy_save_rows_like_cpp(hub.shared()) else {
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
                account = hub.core.account_id,
                bnet_account = hub.core.battlenet_account_id(),
                "Failed to save account toy flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = hub.core.account_id,
                bnet_account = hub.core.battlenet_account_id(),
                "Account toy flags save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }

    pub async fn save_account_heirlooms_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> FinalizationOutcome {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return FinalizationOutcome::Unavailable;
        };
        if hub
            .shared()
            .player_collection_state_snapshot_like_cpp()
            .is_none()
        {
            warn!(
                account = hub.core.account_id,
                "Skipping account heirloom save because canonical Player collection ownership is unresolved"
            );
            return FinalizationOutcome::Unavailable;
        }
        let Some(rows) = self.account_heirloom_save_rows_like_cpp(hub.shared()) else {
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
                account = hub.core.account_id,
                bnet_account = hub.core.battlenet_account_id(),
                "Failed to save account heirloom flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = hub.core.account_id,
                bnet_account = hub.core.battlenet_account_id(),
                "Account heirloom flags save outcome is unknown: {reason}"
            ),
        }
        outcome.into()
    }
}
