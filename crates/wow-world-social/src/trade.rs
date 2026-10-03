//! Represented trade fixture bookkeeping owned by Social.

use crate::SessionSocialLimits;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::PlayerTradeStateLikeCpp;

impl SessionSocialLimits {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_trade_state_for_test_like_cpp(
        &self,
    ) -> Option<PlayerTradeStateLikeCpp> {
        self.trade_test_fixture_like_cpp
            .represented_active_trade_partner_like_cpp
            .map(|partner_guid| PlayerTradeStateLikeCpp {
                partner_guid,
                accepted: self
                    .trade_test_fixture_like_cpp
                    .represented_trade_accepted_like_cpp,
                partner_server_state_index: self
                    .trade_test_fixture_like_cpp
                    .represented_partner_trade_server_state_index_like_cpp,
                client_state_index: self
                    .trade_test_fixture_like_cpp
                    .represented_trade_client_state_index_like_cpp,
                server_state_index: self
                    .trade_test_fixture_like_cpp
                    .represented_trade_server_state_index_like_cpp,
                items: self
                    .trade_test_fixture_like_cpp
                    .represented_trade_items_like_cpp,
                money: self
                    .trade_test_fixture_like_cpp
                    .represented_trade_money_like_cpp,
                spell_id: self
                    .trade_test_fixture_like_cpp
                    .represented_trade_spell_like_cpp,
                spell_cast_item_guid: self
                    .trade_test_fixture_like_cpp
                    .represented_trade_spell_cast_item_like_cpp,
            })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_trade_state_for_test_like_cpp(
        &mut self,
        state: Option<PlayerTradeStateLikeCpp>,
    ) {
        if let Some(state) = state {
            self.trade_test_fixture_like_cpp.represented_active_trade_partner_like_cpp =
                Some(state.partner_guid);
            self.trade_test_fixture_like_cpp.represented_trade_accepted_like_cpp = state.accepted;
            self.trade_test_fixture_like_cpp
                .represented_partner_trade_server_state_index_like_cpp =
                state.partner_server_state_index;
            self.trade_test_fixture_like_cpp
                .represented_trade_client_state_index_like_cpp = state.client_state_index;
            self.trade_test_fixture_like_cpp
                .represented_trade_server_state_index_like_cpp = state.server_state_index;
            self.trade_test_fixture_like_cpp.represented_trade_items_like_cpp = state.items;
            self.trade_test_fixture_like_cpp.represented_trade_money_like_cpp = state.money;
            self.trade_test_fixture_like_cpp.represented_trade_spell_like_cpp = state.spell_id;
            self.trade_test_fixture_like_cpp
                .represented_trade_spell_cast_item_like_cpp = state.spell_cast_item_guid;
        } else {
            self.trade_test_fixture_like_cpp.represented_active_trade_partner_like_cpp = None;
            self.trade_test_fixture_like_cpp.represented_trade_accepted_like_cpp = false;
            self.trade_test_fixture_like_cpp
                .represented_partner_trade_server_state_index_like_cpp = 0;
            self.trade_test_fixture_like_cpp
                .represented_trade_client_state_index_like_cpp = 1;
            self.trade_test_fixture_like_cpp
                .represented_trade_server_state_index_like_cpp = 1;
            self.trade_test_fixture_like_cpp.represented_trade_items_like_cpp =
                [None; wow_packet::packets::misc::TRADE_SLOT_COUNT_LIKE_CPP as usize];
            self.trade_test_fixture_like_cpp.represented_trade_money_like_cpp = 0;
            self.trade_test_fixture_like_cpp.represented_trade_spell_like_cpp = 0;
            self.trade_test_fixture_like_cpp
                .represented_trade_spell_cast_item_like_cpp = None;
        }
    }

    pub fn record_represented_trade_cancel_like_cpp(&mut self, status: u8) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.trade_test_fixture_like_cpp
            .represented_trade_cancel_statuses_like_cpp
            .push(status);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = status;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_trade_cancel_statuses_like_cpp(&self) -> &[u8] {
        &self
            .trade_test_fixture_like_cpp
            .represented_trade_cancel_statuses_like_cpp
    }
}
