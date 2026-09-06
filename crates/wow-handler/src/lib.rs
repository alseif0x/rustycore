// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Opcode handler registration and dispatch for the world server.
//!
//! This crate owns the vocabulary an opcode registration is written in:
//! when a handler may run ([`SessionStatus`]), how it is processed
//! ([`PacketProcessing`]) and the shape of the future it returns
//! ([`HandlerFuture`]).
//!
//! The registry itself lives beside the session it dispatches to
//! (`wow_world::session::registry`, #359): an entry names the concrete
//! session type in its handler thunk, and that type is defined in the crate
//! that depends on this one.

mod processing;

pub use processing::{PacketUpdatePhase, PlayerPacketResidence};

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
    /// Eligible in either the world-session or map-session packet filter.
    /// This does not request a second invocation or imply socket-thread execution.
    Inplace,
    /// Eligible only in the world-session packet filter (thread-unsafe).
    ThreadUnsafe,
    /// Eligible in the map filter while the Player is in world, otherwise world.
    ThreadSafe,
}

/// The future a packet handler returns.
///
/// Handlers are `async` methods on the session, so a registration stores a
/// boxed future rather than an `async fn` pointer, which has no nameable type.
pub type HandlerFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
