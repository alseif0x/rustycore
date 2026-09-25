// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load the selected character's reputation rows during login.

use super::*;

impl WorldSession {
    /// Returns whether the persisted rows were merged into the represented
    /// reputation state; the later first-login reputation phase depends on it.
    pub(super) async fn load_character_reputation_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) -> bool {
        let mut rows_complete = false;
        // C++ login query set includes CHAR_SEL_CHARACTER_REPUTATION and
        // ReputationMgr::LoadFromDB reinitializes from Faction.db2 before merging rows.
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Reputation(rows),
            ) => {
                let rows: Vec<_> = rows
                    .into_iter()
                    .map(|row| CharacterReputationRowLikeCpp {
                        faction_id: row.faction_id,
                        standing: row.standing,
                        flags: row.flags,
                    })
                    .collect();
                if self.load_character_reputation_rows_like_cpp(rows) {
                    rows_complete = true;
                    info!("Loaded character reputation rows for {:?}", guid);
                } else {
                    warn!(
                        "Skipped character reputation load for {:?}: missing Faction.db2 store",
                        guid
                    );
                }
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    "Failed to load character reputation for {:?}: {}",
                    guid, reason
                );
            }
            _ => unreachable!("reputation request returned a different row family"),
        }
        rows_complete
    }
}
