// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trade adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;
use super::{ObjectGuid, SessionCommand, TRADE_SLOT_COUNT_LIKE_CPP};
use super::{TRADE_STATUS_CANCELLED_LIKE_CPP, TRADE_STATUS_UNACCEPTED_LIKE_CPP, TradeStatus};

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_trade_item_like_cpp(&self, slot: u8) -> Option<ObjectGuid> {
        (slot < TRADE_SLOT_COUNT_LIKE_CPP)
            .then(|| {
                self.player_trade_state_snapshot_like_cpp()
                    .flatten()
                    .and_then(|state| state.items[slot as usize])
            })
            .flatten()
    }

    pub(crate) fn clear_represented_trade_item_like_cpp(&mut self, trade_slot: u8) {
        use wow_packet::ServerPacket;

        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if trade_slot >= TRADE_SLOT_COUNT_LIKE_CPP {
            let canonical = self
                .with_owned_player_mut_like_cpp(|player| {
                    player.advance_trade_client_state_index_like_cpp()
                })
                .is_some_and(|changed| changed);
            #[cfg(test)]
            if !canonical && self.player_handle_like_cpp.is_none() {
                let _ = self.mutate_player_trade_state_like_cpp(|state| {
                    if let Some(state) = state {
                        state.client_state_index = state.client_state_index.wrapping_add(1);
                    }
                });
            }
            return;
        }

        let slot = trade_slot as usize;
        if trade.items[slot].is_none() {
            let canonical = self
                .with_owned_player_mut_like_cpp(|player| {
                    player.advance_trade_client_state_index_like_cpp()
                })
                .is_some_and(|changed| changed);
            #[cfg(test)]
            if !canonical && self.player_handle_like_cpp.is_none() {
                let _ = self.mutate_player_trade_state_like_cpp(|state| {
                    if let Some(state) = state {
                        state.client_state_index = state.client_state_index.wrapping_add(1);
                    }
                });
            }
            return;
        }

        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.clear_trade_item_like_cpp(trade_slot))
            .is_some();
        #[cfg(test)]
        let canonical = if !canonical && self.player_handle_like_cpp.is_none() {
            self.mutate_player_trade_state_like_cpp(|state| {
                let Some(state) = state else { return };
                state.client_state_index = state.client_state_index.wrapping_add(1);
                state.items[slot] = None;
                state.accepted = false;
                state.server_state_index = state.server_state_index.wrapping_add(1);
            })
            .is_some()
        } else {
            canonical
        };
        if !canonical {
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

    pub(crate) fn set_represented_trade_item_like_cpp(
        &mut self,
        trade_slot: u8,
        pack_slot: u8,
        item_slot_in_pack: u8,
    ) {
        use wow_packet::ServerPacket;

        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if trade_slot >= TRADE_SLOT_COUNT_LIKE_CPP {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.send_raw_packet(&packet_bytes);
            return;
        }

        let Some(item) = self.get_inventory_item_by_pos(pack_slot, item_slot_in_pack) else {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.send_raw_packet(&packet_bytes);
            return;
        };

        if trade.items.contains(&Some(item.guid)) {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.send_raw_packet(&packet_bytes);
            return;
        }

        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_trade_item_like_cpp(trade_slot, item.guid)
            })
            .is_some();
        #[cfg(test)]
        let canonical = if !canonical && self.player_handle_like_cpp.is_none() {
            self.mutate_player_trade_state_like_cpp(|state| {
                let Some(state) = state else { return };
                state.client_state_index = state.client_state_index.wrapping_add(1);
                state.items[trade_slot as usize] = Some(item.guid);
                state.accepted = false;
                state.server_state_index = state.server_state_index.wrapping_add(1);
            })
            .is_some()
        } else {
            canonical
        };
        if !canonical {
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
}
