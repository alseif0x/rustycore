// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The one place an opcode is bound to the code that runs it.
//!
//! #359 retires the dispatcher's opcode match. Before it, an opcode had to be
//! declared twice — a `PacketHandlerEntry` for its admission metadata and a
//! match arm for the call — and `AGENTS.md` had to warn that forgetting either
//! silently drops the packet. The registration now carries the call as well, so
//! there is one declaration per opcode and no second side to drift from.
//!
//! The shared generic entry vocabulary lives in the private crate-root
//! `packet_registry` module. This module re-exports its public types to keep
//! the established path, and retains the concrete legacy alias, inventory,
//! and dispatch-table construction.

use std::collections::HashMap;

use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};
use wow_packet::WorldPacket;

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};

pub use crate::packet_registry::{PacketHandlerEntryFor, PacketHandlerFn};

/// Concrete legacy alias: Rust does not apply generic defaults while inferring
/// a struct literal's closure arguments. Keep every existing literal fully typed
/// without editing hundreds of registrations or adding another call source.
pub type PacketHandlerEntry = PacketHandlerEntryFor<
    WorldSession,
    SessionHandlerCatalogsLikeCpp,
    ClientOpcodes,
    WorldPacket,
    (),
    SessionStatus,
>;

inventory::collect!(PacketHandlerEntry);

/// Build the dispatch table from all statically registered handlers.
#[must_use]
pub fn build_dispatch_table() -> HashMap<ClientOpcodes, &'static PacketHandlerEntry> {
    inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .map(|entry| (entry.opcode, entry))
        .collect()
}

/// Check if a handler is registered for the given opcode.
#[must_use]
pub fn contains_handler(opcode: ClientOpcodes) -> bool {
    get_handler(opcode).is_some()
}

/// Get the handler entry for a specific opcode.
#[must_use]
pub fn get_handler(opcode: ClientOpcodes) -> Option<&'static PacketHandlerEntry> {
    inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == opcode)
}
