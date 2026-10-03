use crate::SessionSpellState;
use wow_entities::AuraApplicationLikeCpp as AuraApplication;
use wow_world_core::session::{HubMut, HubRef};

impl SessionSpellState {
    pub fn remove_player_visible_aura_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
    ) -> Option<AuraApplication> {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.remove_player_visible_aura_like_cpp(slot)
            })
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_aura_subsystem_like_cpp(hub, |auras| {
                    auras.remove_runtime_application_like_cpp(slot)
                })
                .flatten();
        }
        canonical
    }

    pub fn send_aura_update_removed(&self, hub: HubRef<'_>, slot: u8) {
        let Some(target_guid) = hub.core.player_guid() else {
            return;
        };
        hub.core
            .send_packet(&wow_packet::packets::misc::AuraUpdate {
                unit_guid: target_guid,
                update_all: false,
                auras: vec![wow_packet::packets::misc::AuraInfoLikeCpp {
                    slot,
                    aura_data: None,
                }],
            });
    }
}
