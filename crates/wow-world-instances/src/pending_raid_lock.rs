use crate::{InstanceState, RepresentedPendingBind};
use wow_packet::packets::instance::PendingRaidLock;
use wow_world_core::session::HubMut;

impl InstanceState {
    pub fn has_pending_bind_like_cpp(&self) -> bool {
        self.pending_bind.is_some()
    }

    pub fn take_pending_bind_like_cpp(&mut self) -> Option<RepresentedPendingBind> {
        self.pending_bind.take()
    }

    #[allow(dead_code)]
    pub fn send_pending_raid_lock_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        instance_id: u32,
        completed_mask: u32,
        extending: bool,
        warning_only: bool,
    ) {
        hub.core.send_packet(&PendingRaidLock {
            time_until_lock: 60_000,
            completed_mask,
            extending,
            warning_only,
        });

        if !warning_only {
            self.pending_bind = Some(RepresentedPendingBind {
                map_id: u32::from(hub.core.player_map_id_like_cpp()),
                instance_id,
                completed_mask,
                time_until_lock_ms: 60_000,
            });
        }
    }
}
