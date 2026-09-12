// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session mailbox: the cross-session command protocol, its durable rails and
//! the single pump that applies them.
//!
//! Issue #140 moved this complete vertical out of `wow-network`, which now owns
//! only transport primitives. Queue identity, FIFO order, durability, bounded
//! capacity, incarnation fences, acknowledgements and shutdown drain are
//! unchanged by the move.

mod durable;
mod protocol;
mod pump;
mod session_phase_permit;
mod session_phase_rail;

pub use durable::*;
pub use protocol::*;
pub use session_phase_permit::*;
pub use session_phase_rail::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
