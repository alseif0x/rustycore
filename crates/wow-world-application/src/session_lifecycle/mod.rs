// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session persistence and lifecycle orchestration (#1363 slice 13b).
//!
//! C++ `WorldSession::LogoutPlayer` with the Rust finalization ledger that
//! executes it, the disconnect cleanup, the account and buyback saves it
//! runs, the deferred and periodic `Player::SaveToDB` entries, and
//! `WorldSession::HandleConfirmRespecWipeOpcode` with the `Player::ResetTalents`
//! commit and its publication. Moved from the World
//! `session/lifecycle/{finalization,cleanup,account}.rs`,
//! `session/lifecycle/persistence/deferred.rs`, `session/persistence/{save,commit}.rs`
//! and `handlers/talent/state.rs` verbatim except for the mechanical
//! `self` -> `host` rewrite: each [`SessionLifecycleHostLikeCpp`] method (and
//! each supertrait method) is the same-named `WorldSession` operation, called
//! at the exact point the body called it before the move. The far-transfer
//! native completion is `wow_world_character`'s, the loot settlement
//! `wow_world_loot_ops`'s and the full save `wow_world_lifecycle`'s.

use wow_world_character::WorldportHostLikeCpp;
use wow_world_core::session::HubMut;
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::{PlayerSaveHostLikeCpp, SessionLifecycleState};
use wow_world_loot_ops::LootSessionHostLikeCpp;
use wow_world_spell::TalentHostLikeCpp;

mod account_saves;
mod cleanup;
mod finalization;
mod logout_hook;
mod player_save;
mod talent_reset;

pub use cleanup::cleanup_shared_runtime_state;
#[cfg(any(test, feature = "test-fixtures"))]
pub use finalization::execute_finalization_step;
pub use finalization::{
    finalize_session_with_generator_like_cpp,
    finalize_session_with_resurrection_catalogs,
    run_logout_timer_with_catalogs,
};
pub use logout_hook::PlayerLogoutHookEventLikeCpp;
pub use player_save::{
    process_pending_periodic_player_save_with_generator_like_cpp,
    resume_deferred_player_save_with_generator_like_cpp,
};
pub use talent_reset::handle_confirm_respec_wipe_with_generator_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use talent_reset::{
    AT_LOGIN_RESET_TALENTS_LIKE_CPP, CommittedRepresentedTalentResetLikeCpp,
    UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP, commit_represented_talent_reset_at_like_cpp,
};

/// The session owners the logout buyback clear and the account collection
/// saves borrow together, from disjoint `WorldSession` fields: the World
/// `LifecycleCx` without the instance state these steps never read.
pub struct SessionAccountSaveCxLikeCpp<'a> {
    pub lifecycle: &'a mut SessionLifecycleState,
    pub inventory: &'a mut InventoryState,
    pub hub: HubMut<'a>,
}

/// The World operations the moved lifecycle orchestration still calls.
pub trait SessionLifecycleHostLikeCpp:
    WorldportHostLikeCpp + LootSessionHostLikeCpp + TalentHostLikeCpp + PlayerSaveHostLikeCpp + Send
{
    fn session_account_save_cx_like_cpp(&mut self) -> SessionAccountSaveCxLikeCpp<'_>;

    /// Hook port for `player.logout` (#1444 H1): C++ `ScriptMgr::OnPlayerLogout`
    /// from `WorldSession::LogoutPlayer`. The World adapter dispatches it to
    /// the trusted linked modules; the default does nothing. Called once, from
    /// the finalization ledger's `Retirement` step, outside any split borrow.
    fn player_logout_hook_like_cpp(&mut self, _event: &PlayerLogoutHookEventLikeCpp) {}

    /// C++ `Player::CleanupsBeforeDelete` → `TradeCancel(false)` (#38):
    /// called from `Retirement` after the logout hook, before the map removal.
    fn cancel_trade_before_logout_like_cpp(&mut self) {}

    /// Whether any linked module wants `player.logout`; asked before the event
    /// is built. The default, used by every test host, wants nothing.
    fn player_logout_hook_wanted_like_cpp(&self) -> bool {
        false
    }
}
