//! The far destination follows the same Player incarnation as its teleport semaphores.
//! C++ Player.h:2167,3098 owns m_teleport_dest. This retains Rust's separate near/far
//! representations; it does not establish full world-entry phase completion.
use crate::session::WorldSession;
use wow_core::Position;

impl WorldSession {
    pub(crate) fn pending_teleport_like_cpp(&self) -> Option<(u32, Position)> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.pending_teleport_like_cpp(hub)
    }

    pub(crate) fn set_pending_teleport_like_cpp(
        &mut self,
        destination: Option<(u32, Position)>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_pending_teleport_like_cpp(&mut hub, destination)
    }
}
