use wow_core::ObjectGuid;

use crate::{PlayerGossipOptionLikeCpp, PlayerInteractionDataLikeCpp};

use super::Player;

impl Player {
    pub const fn interaction_data_like_cpp(&self) -> &PlayerInteractionDataLikeCpp {
        &self.gameplay_state.menu.interaction
    }

    pub fn interaction_data_mut_like_cpp(&mut self) -> &mut PlayerInteractionDataLikeCpp {
        &mut self.gameplay_state.menu.interaction
    }

    pub fn reset_interaction_data_like_cpp(&mut self) {
        self.interaction_data_mut_like_cpp().reset();
    }

    pub fn set_interaction_source_like_cpp(&mut self, source_guid: ObjectGuid) {
        self.interaction_data_mut_like_cpp().set_source(source_guid);
    }

    pub fn set_trainer_interaction_like_cpp(&mut self, source_guid: ObjectGuid, trainer_id: u32) {
        self.interaction_data_mut_like_cpp()
            .set_trainer(source_guid, trainer_id);
    }

    pub fn reset_interaction_if_source_like_cpp(&mut self, source_guid: ObjectGuid) -> bool {
        self.interaction_data_mut_like_cpp()
            .reset_if_source(source_guid)
    }

    pub fn gossip_options_like_cpp(&self) -> &[PlayerGossipOptionLikeCpp] {
        &self.gameplay_state.menu.gossip_options
    }

    pub fn replace_gossip_options_like_cpp(&mut self, options: Vec<PlayerGossipOptionLikeCpp>) {
        self.gameplay_state.menu.gossip_options = options;
    }

    pub fn clear_gossip_options_like_cpp(&mut self) {
        self.gameplay_state.menu.gossip_options.clear();
    }
}
