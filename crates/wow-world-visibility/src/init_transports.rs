//! Represented `Map::SendInitTransports` delivery after login self CREATE.

use wow_core::ObjectGuid;
use wow_packet::packets::update::{UpdateBlock, UpdateObject};
use wow_world_core::session::HubMut;

use crate::VisibilityState;

#[derive(Default)]
pub struct InitTransportsPlanLikeCpp {
    pub own_transport: Option<(ObjectGuid, UpdateBlock)>,
    pub other_blocks: Vec<UpdateBlock>,
    pub other_visible_guids: Vec<ObjectGuid>,
    pub considered: usize,
    pub skipped_other_map: usize,
    pub skipped_missing_path: usize,
    pub skipped_phase: usize,
}

impl VisibilityState {
    /// C++ `Map::SendInitTransports`: after `SendInitSelf`, send map
    /// transports other than the player's current transport.
    pub fn send_init_transports_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        plan: Box<InitTransportsPlanLikeCpp>,
    ) {
        if plan.other_blocks.is_empty() {
            return;
        }

        let InitTransportsPlanLikeCpp {
            other_blocks,
            other_visible_guids,
            ..
        } = *plan;
        let update = UpdateObject::create_world_objects(other_blocks, map_id);
        if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
            for line in update.debug_create_summary_like_cpp() {
                tracing::info!("RUST_UPDATEOBJECT init_transports {line}");
            }
        }
        hub.core.send_packet(&update);
        self.extend_client_visible_transports_like_cpp(other_visible_guids);
    }
}
