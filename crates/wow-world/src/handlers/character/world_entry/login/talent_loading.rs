// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load the selected character's talents, with their spell side effects, during login.

use super::*;

impl WorldSession {
    /// Returns whether the persisted talent rows were loaded; the later
    /// complete-spell-rows gate depends on it.
    pub(super) async fn load_character_talents_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
        guid: ObjectGuid,
        known_spells: &mut Vec<i32>,
        skill_rewarded_dependent_spells: &mut HashSet<i32>,
    ) -> bool {
        let mut rows_complete = false;
        // ── Load talents from character_talent ──
        // C++ `Player::LoadFromDB` calls `_LoadTalents` before `_LoadSpells`;
        // `_LoadTalents -> AddTalent` learns the active talent group's spell
        // immediately, so passive talent auras must be present before the
        // login AuraUpdate is built.
        self.reset_represented_talents_like_cpp();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Talents(rows),
            ) => {
                let mut loaded = 0usize;
                let mut skipped = 0usize;
                for row in rows {
                    if self.load_represented_talent_row_with_spell_side_effects_like_cpp(
                        player_bootstrap.talent_tabs.as_ref(),
                        row.talent_id,
                        row.rank,
                        row.talent_group,
                        known_spells,
                        skill_rewarded_dependent_spells,
                    ) {
                        loaded += 1;
                    } else {
                        skipped += 1;
                    }
                }
                self.mark_represented_talents_loaded_like_cpp();
                rows_complete = true;
                info!(
                    loaded,
                    skipped,
                    player_guid = guid.counter(),
                    "Loaded represented character talents like C++ Player::_LoadTalents"
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    "Failed to load character talents for {:?}: {}",
                    guid, reason
                );
            }
            _ => unreachable!("talent request returned a different row family"),
        }
        rows_complete
    }
}
