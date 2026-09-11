//! Represented trade sessions at the Session boundary.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn record_represented_trade_cancel_like_cpp(&mut self, status: u8) {
        #[cfg(test)]
        self.represented_trade_cancel_statuses_like_cpp.push(status);
        #[cfg(not(test))]
        let _ = status;
    }
    pub(in crate::session) fn player_trade_state_snapshot_like_cpp(
        &self,
    ) -> Option<Option<wow_entities::PlayerTradeStateLikeCpp>> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().trade.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                self.represented_active_trade_partner_like_cpp
                    .map(|partner_guid| wow_entities::PlayerTradeStateLikeCpp {
                        partner_guid,
                        accepted: self.represented_trade_accepted_like_cpp,
                        partner_server_state_index: self
                            .represented_partner_trade_server_state_index_like_cpp,
                        client_state_index: self.represented_trade_client_state_index_like_cpp,
                        server_state_index: self.represented_trade_server_state_index_like_cpp,
                        items: self.represented_trade_items_like_cpp,
                        money: self.represented_trade_money_like_cpp,
                        spell_id: self.represented_trade_spell_like_cpp,
                        spell_cast_item_guid: self.represented_trade_spell_cast_item_like_cpp,
                    }),
            );
        }
        canonical
    }
    pub(in crate::session) fn mutate_player_trade_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut Option<wow_entities::PlayerTradeStateLikeCpp>) -> R,
    ) -> Option<R> {
        let mut state = self.player_trade_state_snapshot_like_cpp()?;
        let result = mutate(&mut state);
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.install_trade_state_like_cpp(state.clone())
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            if let Some(state) = state {
                self.represented_active_trade_partner_like_cpp = Some(state.partner_guid);
                self.represented_trade_accepted_like_cpp = state.accepted;
                self.represented_partner_trade_server_state_index_like_cpp =
                    state.partner_server_state_index;
                self.represented_trade_client_state_index_like_cpp = state.client_state_index;
                self.represented_trade_server_state_index_like_cpp = state.server_state_index;
                self.represented_trade_items_like_cpp = state.items;
                self.represented_trade_money_like_cpp = state.money;
                self.represented_trade_spell_like_cpp = state.spell_id;
                self.represented_trade_spell_cast_item_like_cpp = state.spell_cast_item_guid;
            } else {
                self.represented_active_trade_partner_like_cpp = None;
                self.represented_trade_accepted_like_cpp = false;
                self.represented_partner_trade_server_state_index_like_cpp = 0;
                self.represented_trade_client_state_index_like_cpp = 1;
                self.represented_trade_server_state_index_like_cpp = 1;
                self.represented_trade_items_like_cpp = [None; TRADE_SLOT_COUNT_LIKE_CPP as usize];
                self.represented_trade_money_like_cpp = 0;
                self.represented_trade_spell_like_cpp = 0;
                self.represented_trade_spell_cast_item_like_cpp = None;
            }
            return Some(result);
        }
        canonical.then_some(result)
    }
    pub(crate) fn set_represented_active_trade_partner_like_cpp(
        &mut self,
        partner_guid: Option<ObjectGuid>,
    ) -> bool {
        self.mutate_player_trade_state_like_cpp(|state| {
            *state = partner_guid.map(wow_entities::PlayerTradeStateLikeCpp::new);
        })
        .is_some()
    }
    pub(crate) fn clear_represented_active_trade_partner_like_cpp(&mut self) -> bool {
        self.mutate_player_trade_state_like_cpp(|state| *state = None)
            .is_some()
    }
    pub(crate) fn resolved_represented_active_trade_partner_like_cpp(
        &self,
    ) -> Option<Option<ObjectGuid>> {
        self.player_trade_state_snapshot_like_cpp()
            .map(|state| state.map(|state| state.partner_guid))
    }
    #[cfg(test)]
    pub(crate) fn represented_active_trade_partner_like_cpp(&self) -> Option<ObjectGuid> {
        self.resolved_represented_active_trade_partner_like_cpp()
            .expect("test Player trade owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn set_represented_partner_trade_server_state_index_like_cpp(
        &mut self,
        state_index: u32,
    ) -> bool {
        self.mutate_player_trade_state_like_cpp(|state| {
            if let Some(state) = state {
                state.partner_server_state_index = state_index;
            }
        })
        .is_some()
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
    pub(crate) fn set_represented_trade_accepted_like_cpp_for_command(
        &mut self,
        accepted: bool,
    ) -> bool {
        self.mutate_player_trade_state_like_cpp(|state| {
            if let Some(state) = state {
                state.accepted = accepted;
            }
        })
        .is_some()
    }
    #[cfg(test)]
    pub(crate) fn represented_trade_client_state_index_like_cpp(&self) -> u32 {
        self.player_trade_state_snapshot_like_cpp()
            .flatten()
            .map_or(1, |state| state.client_state_index)
    }
    #[cfg(test)]
    pub(crate) fn represented_trade_server_state_index_like_cpp(&self) -> u32 {
        self.player_trade_state_snapshot_like_cpp()
            .flatten()
            .map_or(1, |state| state.server_state_index)
    }
    #[cfg(test)]
    pub(crate) fn represented_trade_money_like_cpp(&self) -> u64 {
        self.player_trade_state_snapshot_like_cpp()
            .flatten()
            .map_or(0, |state| state.money)
    }
    pub(crate) fn cancel_represented_trade_like_cpp(&mut self, status: u8, sendback: bool) {
        use wow_packet::ServerPacket;

        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        let packet_bytes = TradeStatus::cancel_like_cpp(status).to_bytes();
        self.record_represented_trade_cancel_like_cpp(status);
        if !self.clear_represented_active_trade_partner_like_cpp() {
            return;
        }

        if sendback {
            self.send_raw_packet(&packet_bytes);
        }

        self.try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::CancelRepresentedTradeLikeCpp(
                crate::session::mailbox::CancelRepresentedTradeLikeCppCommand {
                    status,
                    packet_bytes,
                },
            ),
        );
    }
    pub(crate) fn set_represented_trade_gold_like_cpp(&mut self, coinage: u64) {
        use wow_packet::ServerPacket;

        let Some(Some(mut trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        trade.client_state_index = trade.client_state_index.wrapping_add(1);

        if trade.money == coinage {
            let _ = self.mutate_player_trade_state_like_cpp(|state| *state = Some(trade));
            return;
        }

        if !self
            .resolved_player_money_like_cpp()
            .is_some_and(|player_money| player_money >= coinage)
        {
            if self
                .mutate_player_trade_state_like_cpp(|state| *state = Some(trade))
                .is_none()
            {
                return;
            }
            let packet_bytes =
                TradeStatus::failed_like_cpp(EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, 0).to_bytes();
            self.send_raw_packet(&packet_bytes);
            return;
        }

        trade.money = coinage;
        trade.accepted = false;
        trade.server_state_index = trade.server_state_index.wrapping_add(1);
        if self
            .mutate_player_trade_state_like_cpp(|state| *state = Some(trade))
            .is_none()
        {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_UNACCEPTED_LIKE_CPP).to_bytes();
        self.send_raw_packet(&packet_bytes);

        self.try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::UnacceptRepresentedTradeLikeCpp(
                crate::session::mailbox::UnacceptRepresentedTradeLikeCppCommand { packet_bytes },
            ),
        );
    }
    pub(crate) fn accept_represented_trade_like_cpp(&mut self, state_index: u32) {
        use wow_packet::ServerPacket;

        let Some(Some(mut trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        trade.accepted = true;

        if trade.partner_server_state_index != state_index {
            trade.accepted = false;
            let _ = self.mutate_player_trade_state_like_cpp(|state| *state = Some(trade));
            let packet_bytes =
                TradeStatus::status_only_like_cpp(TRADE_STATUS_STATE_CHANGED_LIKE_CPP).to_bytes();
            self.send_raw_packet(&packet_bytes);
            return;
        }

        if self
            .mutate_player_trade_state_like_cpp(|state| *state = Some(trade))
            .is_none()
        {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_ACCEPTED_LIKE_CPP).to_bytes();
        self.try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::SendRepresentedTradeStatusLikeCpp(
                crate::session::mailbox::SendRepresentedTradeStatusLikeCppCommand { packet_bytes },
            ),
        );
    }
    pub(crate) fn unaccept_represented_trade_like_cpp(&mut self) {
        use wow_packet::ServerPacket;

        let Some(Some(mut trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        trade.accepted = false;
        if self
            .mutate_player_trade_state_like_cpp(|state| *state = Some(trade))
            .is_none()
        {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_UNACCEPTED_LIKE_CPP).to_bytes();
        self.try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::SendRepresentedTradeStatusLikeCpp(
                crate::session::mailbox::SendRepresentedTradeStatusLikeCppCommand { packet_bytes },
            ),
        );
    }
    pub(crate) fn begin_represented_trade_like_cpp(&mut self) {
        use wow_packet::ServerPacket;

        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        let packet_bytes = TradeStatus::initiated_like_cpp(0).to_bytes();
        self.send_raw_packet(&packet_bytes);

        self.try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::SendRepresentedTradeStatusLikeCpp(
                crate::session::mailbox::SendRepresentedTradeStatusLikeCppCommand { packet_bytes },
            ),
        );
    }
    #[cfg(test)]
    pub(crate) fn represented_trade_cancel_statuses_like_cpp(&self) -> &[u8] {
        &self.represented_trade_cancel_statuses_like_cpp
    }
}
