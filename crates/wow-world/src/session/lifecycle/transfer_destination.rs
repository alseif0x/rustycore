//! The far destination follows the same Player incarnation as its teleport semaphores.
//! C++ Player.h:2167,3098 owns m_teleport_dest. This retains Rust's separate near/far
//! representations; it does not establish full world-entry phase completion.
use crate::session::WorldSession;
use wow_core::Position;

impl WorldSession {
    pub(crate) fn pending_teleport_like_cpp(&self) -> Option<(u32, Position)> {
        self.player_teleport_state_snapshot_like_cpp()
            .and_then(|state| state.far_destination)
    }

    pub(crate) fn set_pending_teleport_like_cpp(
        &mut self,
        destination: Option<(u32, Position)>,
    ) -> bool {
        self.update_player_teleport_state_like_cpp(|state| state.far_destination = destination)
    }
}
