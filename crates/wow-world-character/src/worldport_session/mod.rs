// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The far-transfer completion (#1363 slice 13a): C++
//! `WorldSession::HandleMoveWorldportAck` with `Map::AddPlayerToMap`'s
//! `SendInitSelf`, the worldport post-add phases, the native completion a
//! logout or save must run first, and the bounded homebind recovery. Moved
//! from the World `handlers/travel/{travel,worldport_create}.rs` and
//! `session/lifecycle/{transfer_completion,transfer_recovery}.rs` verbatim
//! except for the mechanical `self` -> `host` rewrite: each
//! [`WorldportHostLikeCpp`] method is the same-named `WorldSession`
//! operation, called at the exact point the body called it before the move,
//! and a method returning a future returns the session's own future
//! unchanged. The map entry is `wow_world_instances`'s.

use std::future::Future;

use wow_world_entities::VisibilitySessionHostLikeCpp;
use wow_world_instances::InstanceSessionHostLikeCpp;
use wow_world_lifecycle::{PetSessionHostLikeCpp, PlayerSaveOutcomeLikeCpp};
use wow_world_spell::SpellEffectHostLikeCpp;

use crate::CharacterLoginHostLikeCpp;

mod completion;
mod recovery;
mod response;
mod self_create;

pub use completion::finish_worldport_native_before_disconnect_like_cpp;
pub use response::handle_world_port_response_with_catalogs_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use {
    completion::begin_worldport_post_add_like_cpp, recovery::recover_rejected_worldport_like_cpp,
    response::send_player_self_create_for_teleport_like_cpp,
    self_create::prepare_player_self_create_for_teleport_like_cpp,
};

/// The World operations the moved far-transfer completion still calls.
pub trait WorldportHostLikeCpp:
    CharacterLoginHostLikeCpp
    + InstanceSessionHostLikeCpp
    + VisibilitySessionHostLikeCpp
    + SpellEffectHostLikeCpp
    + PetSessionHostLikeCpp
{
    fn resume_deferred_player_save_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = Option<PlayerSaveOutcomeLikeCpp>> + Send;
}
