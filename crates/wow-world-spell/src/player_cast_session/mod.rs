// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented player cast at the session boundary (#1363 slice 9e),
//! moved from the World `session/player_cast*`, `handlers/spell/ops_{1,2}.rs`
//! and `session/spell_state/spell_click.rs`.
//!
//! C++ `WorldSession::HandleCastSpellOpcode`, `HandleSpellClick` and
//! `HandleSelfResOpcode`, the Player cast driver and the cast publication
//! rail. The bodies run over [`PlayerCastHostLikeCpp`]; each host method is
//! the same-named `WorldSession` operation, called at the exact point the
//! body called it before the move. The request lifecycle's own
//! [`crate::player_cast::Runtime`] stays implemented by the session.

use std::future::Future;

use wow_core::ObjectGuid;
use wow_world_entities::{
    CreatureSpawnCatalogsLikeCpp, RepresentedSpellClickCreatureSnapshotLikeCpp,
};
use wow_world_inventory::PlayerConditionContextSourceLikeCpp;

use crate::SpellExecutionHostLikeCpp;

mod driver;
mod handlers;
mod publication;

pub use driver::*;
pub use handlers::*;
pub use publication::*;

/// The World session as the player cast's host (supertraits: the spell
/// execution host, the request lifecycle runtime and the movement host the
/// embedded `MoveUpdate` runs over).
pub trait PlayerCastHostLikeCpp:
    SpellExecutionHostLikeCpp
    + crate::player_cast::Runtime
    + wow_world_entities::PlayerMovementHostLikeCpp
{
    /// The World player-condition context, projected once per spell-click
    /// plan and viewed against this host at each condition.
    type PlayerConditionContext: PlayerConditionContextSourceLikeCpp<Self>;
    /// The pending-request tick stays on the session: its canonical owner
    /// transition takes a test-fixture fallback closure.
    fn tick_pending_spell_cast_request_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send;
    fn has_represented_self_res_spell_like_cpp(&self, spell_id: i32) -> bool;
    fn represented_spell_click_creature_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedSpellClickCreatureSnapshotLikeCpp>;
    fn represented_player_condition_context_like_cpp(&self)
    -> Option<Self::PlayerConditionContext>;
}
