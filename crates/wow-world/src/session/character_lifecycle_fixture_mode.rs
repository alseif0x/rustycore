//! Explicit selection of the historical Character lifecycle fixture rail.
//! Generation-bearing sessions always use canonical access, including stale handles.

use super::*;

impl WorldSession {
    pub(in crate::session) fn character_lifecycle_fixture_mode(&self) -> bool {
        #[cfg(test)]
        {
            true
        }
        #[cfg(all(not(test), feature = "test-fixtures"))]
        {
            self.character_lifecycle_fixture_mode
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            false
        }
    }

    pub(crate) fn character_lifecycle_handleless_fixture(&self) -> bool {
        self.character_lifecycle_fixture_mode() && self.player_handle_like_cpp.is_none()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_character_lifecycle_fixture(
        account_id: u32,
        account_name: String,
        security: u8,
        expansion: u8,
        account_expansion: u8,
        build: u32,
        session_key: Vec<u8>,
        locale: String,
        packet_rx: flume::Receiver<WorldPacket>,
        send_tx: flume::Sender<Vec<u8>>,
    ) -> Self {
        let mut session = Self::new(
            account_id,
            account_name,
            security,
            expansion,
            account_expansion,
            build,
            session_key,
            locale,
            packet_rx,
            send_tx,
        );
        #[cfg(feature = "test-fixtures")]
        {
            session.character_lifecycle_fixture_mode = true;
        }
        session
    }
}
