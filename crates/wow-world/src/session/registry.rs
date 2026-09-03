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
//! The entry lives here rather than in `wow-handler` because it names
//! [`WorldSession`]: a handler thunk is `fn(&mut WorldSession, WorldPacket)`,
//! and `wow-handler` is the crate `wow-world` depends on, not the reverse.
//! `wow-handler` keeps the vocabulary both sides share — [`SessionStatus`],
//! [`PacketProcessing`] and [`HandlerFuture`].

use std::collections::HashMap;

use wow_constants::ClientOpcodes;
use wow_handler::{HandlerFuture, PacketProcessing, SessionStatus};
use wow_packet::WorldPacket;

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};

/// The call a registered opcode performs.
///
/// Handlers are `async` methods on [`WorldSession`], so a registration boxes
/// the future rather than storing an `async fn` pointer. A non-capturing
/// closure coerces to this type, which keeps a registration one literal.
pub type PacketHandlerFn = for<'a> fn(
    &'a mut WorldSession,
    &'a SessionHandlerCatalogsLikeCpp,
    WorldPacket,
) -> HandlerFuture<'a, ()>;

/// A registered packet handler: its admission rules and the call itself.
///
/// Collected at startup via the `inventory` crate to build the dispatch table.
pub struct PacketHandlerEntry {
    pub opcode: ClientOpcodes,
    pub status: SessionStatus,
    pub processing: PacketProcessing,
    pub handler_name: &'static str,
    /// The handler this opcode runs. The dispatcher calls this and nothing
    /// else; it does not know which method it reaches (#359).
    pub handler: PacketHandlerFn,
}

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
