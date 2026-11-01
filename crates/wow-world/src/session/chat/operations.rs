//! Represented chat, gossip and text operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_remote_address_like_cpp(&mut self, address: Option<String>) {
        self.core.transport.remote_address_like_cpp = address;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/chat/operations/f3_shims.rs"]
mod f3_shims;
