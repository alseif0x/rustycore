// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter of the lifecycle orchestration host (#1363 slice 13b): the
//! buyback and account-collection saves borrow the `LifecycleCx` owners.

use wow_world_application::{
    PlayerLogoutHookEventLikeCpp, SessionAccountSaveCxLikeCpp, SessionLifecycleHostLikeCpp,
};

use crate::session::WorldSession;

impl SessionLifecycleHostLikeCpp for WorldSession {
    fn session_account_save_cx_like_cpp(&mut self) -> SessionAccountSaveCxLikeCpp<'_> {
        let crate::session::LifecycleCx {
            lifecycle,
            inventory,
            hub,
            ..
        } = crate::session::cx_lifecycle(self);
        SessionAccountSaveCxLikeCpp {
            lifecycle,
            inventory,
            hub,
        }
    }

    /// The `player.logout` hook port, dispatched by the World module adapter.
    fn player_logout_hook_like_cpp(&mut self, event: &PlayerLogoutHookEventLikeCpp) {
        self.module_hooks.player_logout_like_cpp(event);
    }

    /// C++ `Player::CleanupsBeforeDelete` → `TradeCancel(false)`
    /// (`Player.cpp:380`): the partner is told, the leaving client is not.
    fn cancel_trade_before_logout_like_cpp(&mut self) {
        let (social, inventory, spell_state, hub) = crate::session::split_trade_mut(self);
        wow_world_social::TradeHandlerCxLikeCpp::new(hub, social, inventory, spell_state)
            .cancel_represented_trade_like_cpp(
                wow_packet::packets::misc::TRADE_STATUS_CANCELLED_LIKE_CPP,
                false,
            );
    }

    fn player_logout_hook_wanted_like_cpp(&self) -> bool {
        self.module_hooks
            .wants_player_hook_like_cpp(wow_module_api::PlayerHook::Logout)
    }
}
