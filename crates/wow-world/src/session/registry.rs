// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Protocol-specific adapters for the shared packet-handler registry.

#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;
#[cfg(test)]
use wow_constants::ClientOpcodes;

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};

/// A handler thunk specialized for the world session and its catalog view.
pub type PacketHandlerFn =
    wow_handler::PacketHandlerFn<WorldSession, SessionHandlerCatalogsLikeCpp>;

/// A registered handler specialized for the world session and its catalog view.
pub type PacketHandlerEntry =
    wow_handler::PacketHandlerEntry<WorldSession, SessionHandlerCatalogsLikeCpp>;

/// A registered handler specialized for an explicitly selected protocol.
pub type PacketHandlerEntryFor<S, C, O, P, R, A> =
    wow_handler::PacketHandlerEntry<S, C, O, P, R, A>;

/// The immutable registry used by a world session.
pub type WorldPacketHandlerRegistry =
    wow_handler::PacketHandlerRegistry<WorldSession, SessionHandlerCatalogsLikeCpp>;

/// The builder used to compose world-session packet handlers.
pub type WorldPacketHandlerRegistryBuilder =
    wow_handler::RegistryBuilder<WorldSession, SessionHandlerCatalogsLikeCpp>;

/// Compose the immutable world-session dispatch table from registered handlers.
#[cfg(any(test, feature = "test-fixtures"))]
#[must_use]
pub fn build_dispatch_table() -> Arc<WorldPacketHandlerRegistry> {
    crate::handler_composition::compose_packet_handlers_like_cpp()
        .expect("invalid duplicate packet handler composition")
}

/// Snapshot the composed handler entries for callers outside the dispatch path.
#[cfg(any(test, feature = "test-fixtures"))]
#[must_use]
pub fn registered_handler_entries_like_cpp() -> std::vec::IntoIter<PacketHandlerEntry> {
    build_dispatch_table()
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
}

/// Check if a handler is registered for the given opcode.
#[cfg(test)]
#[must_use]
pub(crate) fn contains_handler(opcode: ClientOpcodes) -> bool {
    get_handler(opcode).is_some()
}

/// Get the composed handler entry for a specific opcode.
#[cfg(test)]
#[must_use]
pub(crate) fn get_handler(opcode: ClientOpcodes) -> Option<PacketHandlerEntry> {
    build_dispatch_table().get(opcode).copied()
}
