// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load the selected character's persisted skill rows during login.

use super::*;

impl WorldSession {
    /// Returns the persisted skill rows and whether the query completed; the
    /// coordinator normalizes them and replaces the canonical owner only when
    /// the rows were loaded.
    pub(super) async fn load_character_skill_rows_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) -> (
        HashMap<u16, crate::session::RepresentedPlayerSkillLikeCpp>,
        bool,
    ) {
        let mut skill_records =
            HashMap::<u16, crate::session::RepresentedPlayerSkillLikeCpp>::new();
        let mut loaded = false;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Skills(rows),
            ) => {
                loaded = true;
                for row in rows {
                    if row.skill_id > 0 {
                        skill_records.insert(
                            row.skill_id,
                            crate::session::RepresentedPlayerSkillLikeCpp {
                                skill_id: row.skill_id,
                                step: 0,
                                value: row.value,
                                max: row.max,
                                profession_slot: row.profession_slot,
                                state:
                                    crate::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
                            },
                        );
                    }
                }
                info!(
                    "Loaded {} persisted skill rows for {:?}",
                    skill_records.len(),
                    guid
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load character_skills for {:?}: {}", guid, reason);
            }
            _ => unreachable!("skill request returned a different row family"),
        }
        (skill_records, loaded)
    }
}
