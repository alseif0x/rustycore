//! The session hub that moved handler bodies borrow (#1363 slice 4).
//!
//! A domain crate that owns a moved `WorldSession` body names the World
//! operations it still needs in its own `*HostLikeCpp` trait, with this trait
//! as supertrait, and the World session implements both. The provided methods
//! forward exactly as the session's inherent methods of the same name do
//! (`self.core.player_guid()`, `self.core.send_packet(..)`, `self.core.state`).

use wow_core::ObjectGuid;

use super::{HubMut, HubRef, SessionCore, SessionState};

pub trait SessionHubHostLikeCpp {
    fn hub_ref(&self) -> HubRef<'_>;
    fn hub_mut(&mut self) -> HubMut<'_>;

    fn core(&self) -> &SessionCore {
        self.hub_ref().core
    }
    fn core_mut(&mut self) -> &mut SessionCore {
        self.hub_mut().core
    }
    fn player_guid(&self) -> Option<ObjectGuid> {
        self.core().player_guid()
    }
    fn send_packet<P: wow_packet::ServerPacket>(&self, pkt: &P) -> bool {
        self.core().send_packet(pkt)
    }
    fn state(&self) -> SessionState {
        self.core().state
    }
}
