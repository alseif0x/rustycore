use wow_core::ObjectGuid;
use wow_packet::packets::misc::NpcInteractionOpenResult;
use wow_world_core::session::HubRef;

use crate::InteractionState;

impl InteractionState {
    pub fn send_show_bank_like_cpp(&mut self, hub: HubRef<'_>, banker_guid: ObjectGuid) {
        // C++ `WorldSession::SendShowBank` resets PlayerMenu::InteractionData
        // and stores the banker as the sole active interaction source.
        self.set_player_interaction_source_like_cpp(hub, banker_guid);
        hub.core
            .send_packet(&NpcInteractionOpenResult::new(banker_guid, 8));
    }
}
