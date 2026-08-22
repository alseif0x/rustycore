// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! MariaDB adapter for the Player lifecycle port.
//!
//! `wow-persistence` says *what* to persist and how the outcome is classified;
//! this is the only place that knows the statement, the pool and the driver
//! error. Keeping the mapping here is the point of the split: the Session sees
//! `Applied` / `Failed` / `Unknown` and never a `sqlx::Error`.

use std::sync::Arc;

use wow_persistence::{
    AccountCollectionSaveLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerLifecyclePortLikeCpp, PlayerOfflineMarkLikeCpp, PlayerTutorialsSaveLikeCpp,
};

use crate::params::PreparedStatement;
use crate::statements::{CharStatements, LoginStatements, StatementDef};
use crate::transaction::SqlTransaction;
use crate::{CharacterDatabase, LoginDatabase};

/// Build the tutorials statement for one account.
///
/// Shared rather than duplicated: the Player full-save plan in `wow-world`
/// still appends this same row to its own transaction, and two independent
/// copies of the column order would be free to drift. #286 removes the other
/// caller when the full-save plan moves behind the port.
pub fn build_tutorials_save_statement_like_cpp(
    account_id: u32,
    tutorials: &[u32],
    already_persisted: bool,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(if already_persisted {
        CharStatements::UPD_TUTORIALS.sql()
    } else {
        CharStatements::INS_TUTORIALS.sql()
    });
    for (index, value) in tutorials.iter().copied().enumerate() {
        stmt.set_u32(index, value);
    }
    stmt.set_u32(tutorials.len(), account_id);
    stmt
}

/// Binds the port to the two logical databases the offline marks address.
pub struct MariaDbPlayerLifecycleAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
    login_db: Arc<LoginDatabase>,
}

impl MariaDbPlayerLifecycleAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>, login_db: Arc<LoginDatabase>) -> Self {
        Self {
            character_db,
            login_db,
        }
    }
}

impl PlayerLifecyclePortLikeCpp for MariaDbPlayerLifecycleAdapterLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match mark {
                PlayerOfflineMarkLikeCpp::Character { guid_low } => {
                    let mut stmt = self.character_db.prepare(CharStatements::UPD_CHAR_OFFLINE);
                    stmt.set_u32(0, guid_low);
                    self.character_db.execute(&stmt).await
                }
                PlayerOfflineMarkLikeCpp::CharacterAccount { account_id } => {
                    let mut stmt = self
                        .character_db
                        .prepare(CharStatements::UPD_ACCOUNT_ONLINE);
                    stmt.set_u32(0, account_id);
                    self.character_db.execute(&stmt).await
                }
                PlayerOfflineMarkLikeCpp::LoginAccount { account_id } => {
                    let mut stmt = self.login_db.prepare(LoginStatements::UPD_ACCOUNT_OFFLINE);
                    stmt.set_u32(0, account_id);
                    self.login_db.execute(&stmt).await
                }
            };
            match result {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                // A single-statement write outside a transaction either applied
                // or it did not; there is no COMMIT whose outcome could be
                // indeterminate. `Unknown` is reserved for the transactional
                // paths #200 migrates next, so do not manufacture it here.
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn save_tutorials_like_cpp<'a>(
        &'a self,
        save: PlayerTutorialsSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let stmt = build_tutorials_save_statement_like_cpp(
                save.account_id,
                &save.tutorials,
                save.already_persisted,
            );
            // C++ SaveTutorialsData commits this on its own; keep the single
            // statement inside its own transaction rather than borrowing the
            // character-save transaction it is not part of.
            let mut tx = SqlTransaction::new();
            tx.append(stmt);
            match self.character_db.commit_transaction(tx).await {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows: 1 },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let mut tx = SqlTransaction::new();
            let rows = match &save {
                AccountCollectionSaveLikeCpp::Mounts(rows) => {
                    for row in rows {
                        let mut stmt = self.login_db.prepare(LoginStatements::REP_ACCOUNT_MOUNTS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.mount_spell_id);
                        stmt.set_u8(2, row.flags);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::Toys(rows) => {
                    for row in rows {
                        let mut stmt = self.login_db.prepare(LoginStatements::REP_ACCOUNT_TOYS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.item_id);
                        stmt.set_bool(2, row.is_favorite);
                        stmt.set_bool(3, row.has_fanfare);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::Heirlooms(rows) => {
                    for row in rows {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::REP_ACCOUNT_HEIRLOOMS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.item_id);
                        stmt.set_u32(2, row.flags);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::ItemAppearances {
                    bnet_account_id,
                    appearance_blocks,
                    favorite_inserts,
                    favorite_deletes,
                } => {
                    for block in appearance_blocks {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_ITEM_APPEARANCES);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, block.block_index);
                        stmt.set_u32(2, block.mask);
                        tx.append(stmt);
                    }
                    // Inserts before deletes, as the Session built them.
                    for id in favorite_inserts {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_ITEM_FAVORITE_APPEARANCE);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, *id);
                        tx.append(stmt);
                    }
                    for id in favorite_deletes {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::DEL_BNET_ITEM_FAVORITE_APPEARANCE);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, *id);
                        tx.append(stmt);
                    }
                    appearance_blocks.len() + favorite_inserts.len() + favorite_deletes.len()
                }
                AccountCollectionSaveLikeCpp::TransmogIllusions {
                    bnet_account_id,
                    illusion_blocks,
                } => {
                    for block in illusion_blocks {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_TRANSMOG_ILLUSIONS);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, block.block_index);
                        stmt.set_u32(2, block.mask);
                        tx.append(stmt);
                    }
                    illusion_blocks.len()
                }
            };
            // One collection, one transaction — the shape C++ logout uses and
            // #187 freezes.
            match self.login_db.commit_transaction(tx).await {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows: rows as u64 },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}
