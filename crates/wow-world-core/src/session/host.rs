//! The session hub that moved handler bodies borrow (#1363 slice 4).
//!
//! A domain crate that owns a moved `WorldSession` body names the World
//! operations it still needs in its own `*HostLikeCpp` trait, with this trait
//! as supertrait, and the World session implements both. The provided methods
//! forward exactly as the session's inherent methods of the same name do
//! (`self.core.player_guid()`, `self.core.send_packet(..)`,
//! `self.core.send_raw_packet(..)`, `self.core.state`).

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
    fn send_raw_packet(&self, data: &[u8]) {
        self.core().send_raw_packet(data)
    }
    fn state(&self) -> SessionState {
        self.core().state
    }
    /// The World session's fixtures inside the World crate's own `cfg(test)`
    /// build, `None` everywhere else. A moved body runs a former `#[cfg(test)]`
    /// fixture step only when this is `Some`, so the step keeps meaning the
    /// World test build rather than the destination crate's (#1363 slice 4b).
    #[cfg(any(test, feature = "test-fixtures"))]
    fn test_fixtures_like_cpp(&mut self) -> Option<&mut super::state::SessionFixtures> {
        None
    }
}
