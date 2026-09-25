//! Account-wide collection loading and login spell projection.

use super::*;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp,
};

impl WorldSession {
    pub(in crate::handlers::character) fn login_known_spells_after_account_collections_like_cpp(
        &self,
    ) -> Vec<i32> {
        // C++ `Player::HasSpell` includes inactive, non-disabled rows, while
        // `Player::SendKnownSpells` publishes only active rows. Prefer the
        // complete PlayerSpellMap when available so the internal mirror can
        // retain lower ranks without leaking them into the login packet.
        let mut spells = self
            .complete_represented_player_spell_rows_like_cpp()
            .map(|rows| {
                rows.values()
                    .filter(|spell| {
                        spell.state != crate::session::RepresentedPlayerSpellStateLikeCpp::Removed
                            && spell.active
                            && !spell.disabled
                    })
                    .map(|spell| spell.spell_id)
                    .collect()
            })
            .unwrap_or_else(|| self.known_spells_like_cpp().to_vec());
        for mount in self.account_mount_rows_like_cpp() {
            if !spells.contains(&mount.spell_id) {
                spells.push(mount.spell_id);
            }
        }
        spells
    }

    pub(in crate::handlers::character) async fn load_account_mounts_like_cpp(&mut self) -> bool {
        self.set_account_mounts_like_cpp(Vec::new());
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return false;
        };

        let bnet_account_id = self.battlenet_account_id();
        if bnet_account_id == 0 {
            warn!(
                account = self.account_id,
                "Skipping account mount load because the game account is not linked to a Battle.net account"
            );
            return false;
        }
        let rows = match port
            .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Mounts {
                bnet_account_id,
            })
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::Mounts(rows),
            ) => rows,
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account mounts: {reason}"
                );
                return false;
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for mounts"
                );
                return false;
            }
        };

        if rows.is_empty() {
            info!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Loaded 0 account mounts from battlenet_account_mounts"
            );
            return true;
        }

        let mut mounts = Vec::new();
        let mut skipped_invalid_spell_id = 0usize;
        let mut skipped_missing_mount_db2 = 0usize;
        for row in rows {
            let spell_id = row.mount_spell_id;
            if spell_id <= 0 {
                skipped_invalid_spell_id += 1;
                continue;
            }

            let has_mount = spell_id > 0
                && self.mount_store().is_none_or(|store| {
                    store
                        .get_by_source_spell_id_like_cpp(spell_id as u32)
                        .is_some()
                });
            if has_mount {
                mounts.push(AccountMount {
                    spell_id,
                    flags: row.flags,
                });
            } else {
                skipped_missing_mount_db2 += 1;
            }
        }

        info!(
            account = self.account_id,
            bnet_account = bnet_account_id,
            loaded = mounts.len(),
            skipped_invalid_spell_id,
            skipped_missing_mount_db2,
            "Loaded represented account mounts like C++ CollectionMgr"
        );
        self.set_account_mounts_like_cpp(mounts.clone());
        true
    }

    pub(in crate::handlers::character) async fn load_account_toys_like_cpp(&mut self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_toys_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let rows = match port
            .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id,
            })
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(
                rows,
            )) => rows
                .into_iter()
                .filter_map(|row| {
                    u32::try_from(row.item_id)
                        .ok()
                        .map(|item_id| (item_id, row.is_favorite, row.has_fanfare))
                })
                .collect(),
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account toys: {reason}"
                );
                Vec::new()
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for toys"
                );
                Vec::new()
            }
        };

        self.load_represented_account_toys_like_cpp(rows);
    }

    pub(in crate::handlers::character) async fn load_account_heirlooms_like_cpp(&mut self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_heirlooms_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let rows = match port
            .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id,
            })
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::Heirlooms(rows),
            ) => rows
                .into_iter()
                .filter_map(|row| {
                    u32::try_from(row.item_id)
                        .ok()
                        .map(|item_id| (item_id, row.flags))
                })
                .collect(),
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account heirlooms: {reason}"
                );
                Vec::new()
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for heirlooms"
                );
                Vec::new()
            }
        };

        self.load_represented_account_heirlooms_like_cpp(rows);
    }

    pub(in crate::handlers::character) async fn load_account_item_appearances_like_cpp(&mut self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_item_appearances_like_cpp([], []);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let (appearance_blocks, favorite_appearances) = match port
            .load_account_collection_like_cpp(
                AccountCollectionLoadRequestLikeCpp::ItemAppearances { bnet_account_id },
            )
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::ItemAppearances {
                    appearance_blocks,
                    favorite_appearance_ids,
                },
            ) => {
                let appearance_blocks = match appearance_blocks {
                    AccountCollectionRowsLikeCpp::Loaded(rows) => rows
                        .into_iter()
                        .map(|row| (row.block_index, row.mask))
                        .collect(),
                    AccountCollectionRowsLikeCpp::Failed { reason } => {
                        warn!(
                            account = self.account_id,
                            bnet_account = bnet_account_id,
                            "Failed to load account item appearances: {reason}"
                        );
                        Vec::new()
                    }
                };
                let favorite_appearances = match favorite_appearance_ids {
                    AccountCollectionRowsLikeCpp::Loaded(rows) => rows,
                    AccountCollectionRowsLikeCpp::Failed { reason } => {
                        warn!(
                            account = self.account_id,
                            bnet_account = bnet_account_id,
                            "Failed to load account favorite item appearances: {reason}"
                        );
                        Vec::new()
                    }
                };
                (appearance_blocks, favorite_appearances)
            }
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account item appearances: {reason}"
                );
                (Vec::new(), Vec::new())
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for item appearances"
                );
                (Vec::new(), Vec::new())
            }
        };

        self.load_represented_account_item_appearances_like_cpp(
            appearance_blocks,
            favorite_appearances,
        );
    }

    pub(in crate::handlers::character) async fn load_account_transmog_illusions_like_cpp(
        &mut self,
    ) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_transmog_illusions_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let illusion_blocks = match port
            .load_account_collection_like_cpp(
                AccountCollectionLoadRequestLikeCpp::TransmogIllusions { bnet_account_id },
            )
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::TransmogIllusions { illusion_blocks },
            ) => illusion_blocks
                .into_iter()
                .map(|row| (row.block_index, row.mask))
                .collect(),
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account transmog illusions: {reason}"
                );
                Vec::new()
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for transmog illusions"
                );
                Vec::new()
            }
        };

        self.load_represented_account_transmog_illusions_like_cpp(illusion_blocks);
    }
}
