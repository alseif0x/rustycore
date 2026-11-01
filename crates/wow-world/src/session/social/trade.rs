//! Represented trade sessions at the Session boundary.
//!
//! Moved out of the Session root under #615 and reduced to the Session-side
//! adapter under #1263 F5: the trade bodies and their canonical/represented
//! transitions now live in `wow-world-social::trade_handlers`, and this
//! module only builds the borrowed context for the World callers that still
//! need it.

use super::*;
use wow_world_social::TradeHandlerCxLikeCpp;

impl WorldSession {
    fn trade_handler_cx_like_cpp(&mut self) -> TradeHandlerCxLikeCpp<'_> {
        let (social, inventory, spell_state, hub) = crate::session::split_trade_mut(self);
        TradeHandlerCxLikeCpp::new(hub, social, inventory, spell_state)
    }

    #[cfg(test)]
    pub(in crate::session) fn player_trade_state_snapshot_like_cpp(
        &self,
    ) -> Option<Option<wow_entities::PlayerTradeStateLikeCpp>> {
        wow_world_social::player_trade_state_snapshot_like_cpp(&self.core, &self.social)
    }

    pub(crate) fn clear_represented_active_trade_partner_like_cpp(&mut self) -> bool {
        self.trade_handler_cx_like_cpp()
            .clear_represented_active_trade_partner_like_cpp()
    }

    pub(crate) fn set_represented_trade_accepted_like_cpp_for_command(
        &mut self,
        accepted: bool,
    ) -> bool {
        self.trade_handler_cx_like_cpp()
            .set_represented_trade_accepted_like_cpp_for_command(accepted)
    }

    #[cfg(test)]
    pub(crate) fn mutate_player_trade_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut Option<wow_entities::PlayerTradeStateLikeCpp>) -> R,
    ) -> Option<R> {
        self.trade_handler_cx_like_cpp()
            .mutate_player_trade_state_like_cpp(mutate)
    }

    #[cfg(test)]
    pub(crate) fn represented_active_trade_partner_like_cpp(&self) -> Option<ObjectGuid> {
        wow_world_social::resolved_represented_active_trade_partner_like_cpp(
            &self.core,
            &self.social,
        )
        .expect("test Player trade owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn represented_trade_accepted_like_cpp(&self) -> bool {
        self.player_trade_state_snapshot_like_cpp()
            .flatten()
            .is_some_and(|state| state.accepted)
    }

    #[cfg(test)]
    pub(crate) fn set_represented_trade_accepted_like_cpp_for_test(&mut self, accepted: bool) {
        let _ = self.set_represented_trade_accepted_like_cpp_for_command(accepted);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/social/trade/f3_shims.rs"]
mod f3_shims;
