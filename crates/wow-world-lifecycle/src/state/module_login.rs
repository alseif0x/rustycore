use tracing::{debug, warn};
use wow_world_core::session::HubRef;

use super::SessionLifecycleState;

impl SessionLifecycleState {
    /// C++ `ScriptMgr::OnPlayerLogin` (`ScriptMgr.cpp:2052-2055`), invoked
    /// once after a completed login (`CharacterHandler.cpp:1452`).
    ///
    /// Modules receive an immutable snapshot and return effects; the batch is
    /// validated as a whole before anything is applied, so an invalid effect
    /// discards the batch instead of half-applying it. A rejected batch is
    /// logged and the login continues: a module must not be able to fail a
    /// player's login.
    pub fn dispatch_module_player_login_like_cpp(
        &self,
        hub: HubRef<'_>,
        registry: &wow_module_api::ModuleRegistry,
        first_login: bool,
    ) {
        if registry.is_empty() {
            return;
        }
        let Some(guid) = hub.core.player_guid() else {
            return;
        };
        let snapshot = wow_module_api::PlayerLoginSnapshot {
            guid,
            name: hub.player_name_like_cpp().unwrap_or_default(),
            race: hub.player_race_like_cpp(),
            class: hub.player_class_like_cpp(),
            level: hub.player_level_like_cpp(),
            map_id: hub.core.player_map_id_like_cpp(),
            first_login,
        };
        match registry.dispatch_player_login(&snapshot) {
            Ok(effects) => {
                for (module, effect) in effects.iter() {
                    match effect {
                        wow_module_api::PlayerLoginEffect::SendSystemMessageSelf { text } => {
                            debug!(module = %module, "module login message");
                            hub.core.send_system_message_like_cpp(text);
                        }
                    }
                }
            }
            Err(error) => {
                warn!(%error, "module login effect batch rejected; no effect applied");
            }
        }
    }
}
