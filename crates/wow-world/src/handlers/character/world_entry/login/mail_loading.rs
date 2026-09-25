// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hydrate the canonical Player mail owner during character login.

use super::*;

impl WorldSession {
    /// Returns `false` after kicking when mail hydration fails; login aborts.
    pub(super) async fn load_character_mail_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) -> bool {
        let mail_rows = match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Mail {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Mail(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(player_guid = guid.counter(), %reason, "failed to load canonical Player mail owner");
                self.kick("WorldSession::HandlePlayerLogin Player mail hydration failed");
                return false;
            }
            _ => {
                self.kick("WorldSession::HandlePlayerLogin invalid Player mail hydration outcome");
                return false;
            }
        };
        let mails = mail_rows
            .into_iter()
            .map(|row| wow_entities::PlayerMailRecord {
                mail_id: row.mail_id,
                message_type: row.message_type,
                sender: row.sender,
                receiver: row.receiver,
                template_id: (row.template_id != 0).then_some(row.template_id),
                deliver_time: row.deliver_time,
                expire_time: row.expire_time,
                checked_flags: row.checked_flags,
                stationery_id: row.stationery_id,
            })
            .collect();
        if !self.replace_owned_player_mails_like_cpp(mails) {
            self.kick("WorldSession::HandlePlayerLogin canonical Player mail owner disappeared");
            return false;
        }
        true
    }
}
