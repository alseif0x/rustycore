// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented pet loading, mode state and resummon at the session boundary
//! (#1363 slice 7a).
//!
//! C++ `Player::LoadPet`/`Pet::LoadPetFromDB`, `Player::ResummonPetTemporaryUnSummonedIfAny`
//! and `Pet::SetReactState` run on the Player. Their Rust bodies now live in
//! this crate; each host method below is the same-named `WorldSession` field
//! split or operation, called at the exact point the body called it before the
//! move. The former `#[cfg(test)]` fixture steps run through
//! `SessionHubHostLikeCpp::test_fixtures_like_cpp`.

use wow_world_core::session::{HubMut, SessionHubHostLikeCpp};

use crate::SessionLifecycleState;

mod loading;
mod login_loading;
mod summoning;

pub use loading::{
    adjusted_represented_pet_aura_remain_time_like_cpp,
    begin_represented_character_pet_authority_load_like_cpp,
    load_represented_pet_aura_effect_rows_like_cpp, load_represented_pet_aura_rows_like_cpp,
    load_represented_pet_aura_rows_with_timediff_like_cpp,
    load_represented_pet_declined_names_like_cpp, load_represented_pet_spell_charge_rows_like_cpp,
    load_represented_pet_spell_cooldown_rows_like_cpp, load_represented_pet_spell_rows_like_cpp,
    set_represented_pet_mode_state_with_spell_like_cpp,
};
pub use login_loading::{
    apply_represented_login_pet_talent_reset_like_cpp, load_represented_login_pet_state_like_cpp,
};
pub use summoning::resummon_pet_temporary_unsummoned_like_cpp;

pub trait PetSessionHostLikeCpp: SessionHubHostLikeCpp {
    /// The World `split_lifecycle_mut` split: the lifecycle state and the mutable hub.
    fn split_pets_mut(&mut self) -> (&mut SessionLifecycleState, HubMut<'_>);
    fn resolved_represented_at_login_flags_like_cpp(&self) -> Option<u16>;
}
