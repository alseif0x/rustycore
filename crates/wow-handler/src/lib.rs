// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Opcode handler registration and dispatch for the world server.
//!
//! Uses the `inventory` crate for static registration of packet handlers.
//! Handler functions live in their respective game crates; this crate provides
//! the dispatch table that maps opcodes to handler metadata.

use std::collections::HashMap;

use wow_constants::ClientOpcodes;

/// Status requirements for a packet handler.
///
/// Controls when a handler is allowed to run based on the session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    /// Before character login (account authenticated).
    Authed,
    /// Character is logged in and in the world.
    LoggedIn,
    /// During a map/instance transfer.
    Transfer,
    /// Logged in, or recently logged out (grace period).
    LoggedInOrRecentlyLogout,
}

/// How the packet should be processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketProcessing {
    /// Process immediately in the socket/network thread.
    Inplace,
    /// Queue for processing during the session update tick (thread-unsafe).
    ThreadUnsafe,
}

/// A registered packet handler entry.
///
/// Collected at startup via the `inventory` crate to build the dispatch table.
pub struct PacketHandlerEntry {
    pub opcode: ClientOpcodes,
    pub status: SessionStatus,
    pub processing: PacketProcessing,
    pub handler_name: &'static str,
}

// Enable static collection via inventory
inventory::collect!(PacketHandlerEntry);

/// Build the dispatch table from all statically registered handlers.
///
/// Returns a map from `ClientOpcodes` to the handler entry.
pub fn build_dispatch_table() -> HashMap<ClientOpcodes, &'static PacketHandlerEntry> {
    inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .map(|entry| (entry.opcode, entry))
        .collect()
}

/// Check if a handler is registered for the given opcode.
pub fn contains_handler(opcode: ClientOpcodes) -> bool {
    inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .any(|entry| entry.opcode == opcode)
}

/// Get the handler entry for a specific opcode.
pub fn get_handler(opcode: ClientOpcodes) -> Option<&'static PacketHandlerEntry> {
    inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == opcode)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Register a test handler
    inventory::submit! {
        PacketHandlerEntry {
            opcode: ClientOpcodes::Ping,
            status: SessionStatus::Authed,
            processing: PacketProcessing::Inplace,
            handler_name: "handle_ping",
        }
    }

    #[test]
    fn dispatch_table_contains_registered() {
        let table = build_dispatch_table();
        assert!(table.contains_key(&ClientOpcodes::Ping));
        let entry = table[&ClientOpcodes::Ping];
        assert_eq!(entry.handler_name, "handle_ping");
        assert_eq!(entry.status, SessionStatus::Authed);
        assert_eq!(entry.processing, PacketProcessing::Inplace);
    }

    #[test]
    fn contains_handler_check() {
        assert!(contains_handler(ClientOpcodes::Ping));
        assert!(!contains_handler(ClientOpcodes::AttackSwing));
    }

    #[test]
    fn get_handler_found() {
        let entry = get_handler(ClientOpcodes::Ping).unwrap();
        assert_eq!(entry.handler_name, "handle_ping");
    }

    #[test]
    fn get_handler_not_found() {
        assert!(get_handler(ClientOpcodes::AttackSwing).is_none());
    }
}
