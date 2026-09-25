// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load the selected character's persisted spell and favorite-spell rows during login.

use super::*;
use crate::handlers::character::spell_rules::{
    active_known_spell_for_send_like_cpp, loaded_spell_for_add_spell_side_effects_like_cpp,
};

/// Persisted `character_spell` rows projected for the login coordinator.
#[derive(Default)]
pub(super) struct LoginSpellRowsLikeCpp {
    /// Raw logical rows, including inactive and disabled spells.
    pub(super) player_spell_rows: Vec<crate::session::RepresentedPlayerSpellLikeCpp>,
    /// Spells whose C++ `AddSpell` side effects run on load.
    pub(super) side_effect_spells: Vec<i32>,
    /// Active, enabled spells projected for the client.
    pub(super) known_spells: Vec<i32>,
    /// Whether the query completed; the complete-spell-rows gate depends on it.
    pub(super) complete: bool,
}

impl WorldSession {
    pub(super) async fn load_character_spell_rows_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) -> LoginSpellRowsLikeCpp {
        // Column types: spell=int unsigned, active=tinyint unsigned, disabled=tinyint unsigned
        let mut rows = LoginSpellRowsLikeCpp::default();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Spells(db_rows),
            ) => {
                for row in db_rows {
                    if let Ok(spell_id) = i32::try_from(row.spell_id)
                        && spell_id > 0
                    {
                        rows.player_spell_rows.push(
                            crate::session::RepresentedPlayerSpellLikeCpp {
                                spell_id,
                                active: row.active != 0,
                                disabled: row.disabled != 0,
                                dependent: false,
                                favorite: false,
                                state:
                                    crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
                            },
                        );
                    }
                    if let Some(spell_id_i32) =
                        loaded_spell_for_add_spell_side_effects_like_cpp(row.spell_id, row.disabled)
                    {
                        rows.side_effect_spells.push(spell_id_i32);
                    }
                    if let Some(spell_id) =
                        active_known_spell_for_send_like_cpp(row.spell_id, row.active, row.disabled)
                    {
                        rows.known_spells.push(spell_id);
                    }
                }
                rows.complete = true;
                info!(
                    "Loaded {} DB spells for {:?}",
                    rows.known_spells.len(),
                    guid
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load spells for {:?}: {}", guid, reason);
            }
            _ => unreachable!("spell request returned a different row family"),
        }
        rows
    }

    /// Returns the persisted favorite spells and whether the query completed.
    pub(super) async fn load_character_favorite_spells_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) -> (HashSet<i32>, bool) {
        let mut favorites = HashSet::new();
        let mut complete = false;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::SpellFavorites(rows),
            ) => {
                for spell_id in rows {
                    if let Ok(spell_id) = i32::try_from(spell_id) {
                        favorites.insert(spell_id);
                    }
                }
                complete = true;
                info!(
                    "Loaded {} DB favorite spells for {:?}",
                    favorites.len(),
                    guid
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load favorite spells for {:?}: {}", guid, reason);
            }
            _ => unreachable!("favorite-spell request returned a different row family"),
        }
        (favorites, complete)
    }
}
