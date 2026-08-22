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
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerLifecyclePortLikeCpp,
    PlayerOfflineMarkLikeCpp, PlayerTutorialsSaveLikeCpp,
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
}
