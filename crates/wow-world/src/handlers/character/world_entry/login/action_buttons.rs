// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load and packet-project the selected character action-button map.

use super::*;

impl WorldSession {
    pub(super) async fn load_action_buttons_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) -> Option<[i64; 180]> {
        // Column types: button=tinyint unsigned, action=int unsigned, type=tinyint unsigned
        let mut action_buttons = [0i64; 180];
        let mut action_count = 0u32;
        self.reset_represented_action_buttons_like_cpp();
        // C++ loads the action-button map for GetActiveTalentGroup(), not always spec 0.
        let Some((active_spec, trait_config_id)) =
            self.represented_action_button_db_context_like_cpp()
        else {
            self.kick(
                "canonical Player specialization owner unavailable while loading action buttons",
            );
            return None;
        };
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons {
                    player_guid: guid.counter() as u64,
                    active_spec,
                    trait_config_id,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::ActionButtons(rows),
            ) => {
                for row in rows {
                    if (row.button as usize) < 180 && row.action > 0 {
                        self.record_loaded_action_button_like_cpp(
                            row.button,
                            row.action,
                            row.button_type,
                        );
                        action_buttons[row.button as usize] =
                            wow_packet::packets::misc::UpdateActionButtons::pack_button(
                                row.action as i32,
                                row.button_type,
                            );
                        action_count += 1;
                    }
                }
                self.mark_represented_action_buttons_loaded_like_cpp();
                info!("Loaded {} action buttons for {:?}", action_count, guid);
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load action buttons for {:?}: {}", guid, reason);
            }
            _ => unreachable!("action-button request returned a different row family"),
        }
        Some(action_buttons)
    }
}
