use wow_packet::packets::gossip::GossipComplete;
use wow_world_core::session::HubRef;

use crate::InteractionState;

impl InteractionState {
    pub fn send_close_gossip_like_cpp(&mut self, hub: HubRef<'_>) {
        self.reset_player_interaction_data_like_cpp(hub);
        hub.core.send_packet_realm(&GossipComplete {
            suppress_sound: false,
        });
    }
}
