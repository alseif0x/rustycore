// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load the selected character's persisted auras and aura effects during login.

use super::*;

impl WorldSession {
    pub(super) async fn load_character_auras_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) {
        self.set_player_aura_authority_complete_like_cpp(false);
        let mut aura_rows = Vec::new();
        let mut aura_rows_complete = false;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuras(rows),
            ) => {
                aura_rows_complete = true;
                aura_rows.extend(rows.into_iter().map(|row| {
                    crate::session::CharacterAuraRowLikeCpp {
                        caster_guid: object_guid_from_db_binary_like_cpp(row.caster_guid_binary),
                        spell_id: row.spell_id,
                        effect_mask: row.effect_mask,
                        recalculate_mask: row.recalculate_mask,
                        difficulty: row.difficulty,
                        stack_count: row.stack_count,
                        max_duration_ms: row.max_duration_ms,
                        remain_time_ms: row.remain_time_ms,
                        remain_charges: row.remain_charges,
                    }
                }));
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load character auras for {:?}: {}", guid, reason)
            }
            _ => unreachable!("character-aura request returned a different row family"),
        }

        let mut aura_effect_rows = Vec::new();
        let mut aura_effect_rows_complete = false;
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuraEffects(rows),
            ) => {
                aura_effect_rows_complete = true;
                aura_effect_rows.extend(rows.into_iter().map(|row| {
                    crate::session::CharacterAuraEffectRowLikeCpp {
                        caster_guid: object_guid_from_db_binary_like_cpp(row.caster_guid_binary),
                        spell_id: row.spell_id,
                        effect_mask: row.effect_mask,
                        effect_index: row.effect_index,
                        amount: row.amount,
                        base_amount: row.base_amount,
                    }
                }));
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    "Failed to load character aura effects for {:?}: {}",
                    guid, reason
                )
            }
            _ => unreachable!("character-aura-effect request returned a different row family"),
        }
        let loaded_character_auras =
            self.load_represented_character_auras_like_cpp(aura_rows, aura_effect_rows, 0);
        self.set_player_aura_authority_complete_like_cpp(
            aura_rows_complete && aura_effect_rows_complete,
        );
        info!(
            loaded_character_auras,
            player_guid = guid.counter(),
            "Loaded represented character auras like C++ Player::_LoadAuras"
        );
    }
}
