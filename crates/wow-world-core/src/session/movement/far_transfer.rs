//! Hub access to canonical far-teleport state.

impl crate::session::HubMut<'_> {
    pub fn set_represented_far_teleport_pending_like_cpp(&mut self, pending: bool) -> bool {
        self.update_player_teleport_state_like_cpp(|state| state.far_pending = pending)
    }
}
impl crate::session::HubRef<'_> {
    pub fn represented_far_teleport_pending_like_cpp(&self) -> bool {
        self.player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.far_pending)
    }
}
