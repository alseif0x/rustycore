// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared vocabulary for statically registered packet handlers.
//!
//! The registration carries both admission metadata and the call it performs,
//! so every opcode has one declaration and no second dispatcher match arm can
//! drift from it.  The generic types here are shared by the legacy world
//! session registry and Forever's build-specific registry.  The legacy
//! [`crate::session::registry`] module re-exports them to preserve its public
//! path while retaining ownership of its concrete inventory and dispatch
//! table.

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};
use wow_constants::ClientOpcodes;
use wow_handler::{HandlerFuture, PacketProcessing, SessionStatus};
use wow_packet::WorldPacket;

/// The call a registered opcode performs.
///
/// Handlers are `async` methods on [`WorldSession`], so a registration boxes
/// the future rather than storing an `async fn` pointer. A non-capturing
/// closure coerces to this type, which keeps a registration one literal.
pub type PacketHandlerFn<
    S = WorldSession,
    C = SessionHandlerCatalogsLikeCpp,
    P = WorldPacket,
    R = (),
> = for<'a> fn(&'a mut S, &'a C, P) -> HandlerFuture<'a, R>;

/// A registered packet handler: its admission rules and the call itself.
pub struct PacketHandlerEntryFor<
    S = WorldSession,
    C = SessionHandlerCatalogsLikeCpp,
    O = ClientOpcodes,
    P = WorldPacket,
    R = (),
    A = SessionStatus,
> {
    pub opcode: O,
    pub status: A,
    pub processing: PacketProcessing,
    pub handler_name: &'static str,
    /// The handler this opcode runs. The dispatcher calls this and nothing
    /// else; it does not know which method it reaches (#359).
    pub handler: PacketHandlerFn<S, C, P, R>,
}
