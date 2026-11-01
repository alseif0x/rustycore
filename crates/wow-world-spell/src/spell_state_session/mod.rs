// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented spell state at the session boundary (#1363 slice 12b3).
//!
//! The World `session/spell_state` field-split queries, the self-resurrection
//! spell check, the far-teleport cast interrupt, the login proficiency replay
//! and the spell-pull attack start. Their Rust bodies now live in this crate
//! as free functions; each host method is the same-named `WorldSession`
//! operation, called at the exact point the body called it before the move.

mod capability;
mod operations;
mod pull_attack_start;
mod queries;

pub use capability::*;
pub use operations::*;
pub use pull_attack_start::*;
pub use queries::*;

/// The World operations the moved spell-state bodies still call.
pub trait SpellStateHostLikeCpp: crate::SpellExecutionHostLikeCpp {
    /// The World cast interrupt (it runs the application request lifecycle).
    fn interrupt_player_cast_like_cpp(&mut self, spell_id: Option<i32>) -> bool;
}
