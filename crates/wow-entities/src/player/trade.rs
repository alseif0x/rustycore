//! Player-owned trade transitions.
//!
//! C++ keeps the live `TradeData` in `Player::m_trade`.  The session adapts
//! packets, inventory admission and the partner mailbox around these named
//! transitions; it does not receive a mutable trade-state closure.

use super::{PLAYER_TRADE_SLOT_COUNT_LIKE_CPP, Player, PlayerTradeStateLikeCpp};
use wow_core::ObjectGuid;

impl Player {
    /// Read the currently open `Player::m_trade` value.
    pub fn trade_state_snapshot_like_cpp(&self) -> Option<PlayerTradeStateLikeCpp> {
        self.gameplay_state().trade.clone()
    }

    /// C++ `TradeData::GetItem` → `Player::GetItemByGuid`
    /// (`ItemSearchLocation::Everywhere`: equipment, bags, bank, reagent bank;
    /// never buyback). The inventory runtime is the position authority: its
    /// top-level records, plus item objects contained in a top-level bag.
    pub fn trade_item_by_guid_like_cpp(&self, item_guid: ObjectGuid) -> Option<&crate::Item> {
        self.trade_item_position_like_cpp(item_guid)?;
        self.inventory_runtime_like_cpp()
            .item_objects()
            .get(&item_guid)
    }

    /// Mark this Player's session as logging out for trade admission.
    pub fn set_trade_session_logging_out_like_cpp(&mut self, logging_out: bool) {
        self.gameplay_state_mut().trade_session_logging_out = logging_out;
    }

    /// C++ `pOther->GetSession()->isLogingOut()` as trade admission sees it.
    pub fn trade_session_logging_out_like_cpp(&self) -> bool {
        self.gameplay_state().trade_session_logging_out
    }

    /// C++ `WorldSession::HandleInitiateTradeOpcode` creates `m_trade` on both
    /// participants (`TradeHandler.cpp:694-695`).
    pub fn open_trade_like_cpp(&mut self, partner_guid: ObjectGuid) {
        self.gameplay_state_mut().trade = Some(PlayerTradeStateLikeCpp::new(partner_guid));
    }

    /// C++ `Player::TradeCancel` clears `m_trade` (`Player.cpp:12877`).
    pub fn clear_trade_like_cpp(&mut self) {
        self.gameplay_state_mut().trade = None;
    }

    /// C++ `TradeData::SetAccepted` state half; the status packet is the
    /// caller's publication.
    pub fn set_trade_accepted_like_cpp(&mut self, accepted: bool) -> bool {
        let Some(trade) = self.gameplay_state_mut().trade.as_mut() else {
            return false;
        };
        trade.accepted = accepted;
        true
    }

    /// C++ `TradeData::UpdateClientStateIndex`.
    pub fn advance_trade_client_state_index_like_cpp(&mut self) -> bool {
        let Some(trade) = self.gameplay_state_mut().trade.as_mut() else {
            return false;
        };
        trade.client_state_index = trade.client_state_index.wrapping_add(1);
        true
    }

    /// C++ `TradeData::SetInAcceptProcess`.
    pub fn set_trade_in_accept_process_like_cpp(&mut self, in_accept_process: bool) -> bool {
        let Some(trade) = self.gameplay_state_mut().trade.as_mut() else {
            return false;
        };
        trade.in_accept_process = in_accept_process;
        true
    }

    /// The state half of C++ `TradeData::SetItem(slot, item, update)`
    /// (`TradeData.cpp:57-80`): `None` without a trade or for an invalid slot,
    /// `Some(false)` when the slot already holds `item` and `update` is unset,
    /// otherwise the slot changes, `_accepted` clears and the server state
    /// index takes the caller's `rand32()` value. Resetting the trader's
    /// acceptance, the packets and the spell resets are the caller's.
    pub fn set_trade_slot_item_like_cpp(
        &mut self,
        trade_slot: u8,
        item_guid: Option<ObjectGuid>,
        update: bool,
        server_state_index: u32,
    ) -> Option<bool> {
        let slot = usize::from(trade_slot);
        if slot >= PLAYER_TRADE_SLOT_COUNT_LIKE_CPP {
            return None;
        }
        let trade = self.gameplay_state_mut().trade.as_mut()?;
        if trade.items[slot] == item_guid && !update {
            return Some(false);
        }
        trade.items[slot] = item_guid;
        trade.accepted = false;
        trade.server_state_index = server_state_index;
        Some(true)
    }

    /// The state half of C++ `TradeData::SetSpell` (`TradeData.cpp:82-98`).
    pub fn set_trade_spell_value_like_cpp(
        &mut self,
        spell_id: u32,
        cast_item_guid: Option<ObjectGuid>,
        server_state_index: u32,
    ) -> Option<bool> {
        let trade = self.gameplay_state_mut().trade.as_mut()?;
        if trade.spell_id == spell_id && trade.spell_cast_item_guid == cast_item_guid {
            return Some(false);
        }
        trade.spell_id = spell_id;
        trade.spell_cast_item_guid = cast_item_guid;
        trade.accepted = false;
        trade.server_state_index = server_state_index;
        Some(true)
    }

    /// The state half of C++ `TradeData::SetMoney` (`TradeData.cpp:100-122`)
    /// once the caller checked `HasEnoughMoney`.
    pub fn set_trade_money_value_like_cpp(
        &mut self,
        money: u64,
        server_state_index: u32,
    ) -> Option<bool> {
        let trade = self.gameplay_state_mut().trade.as_mut()?;
        if trade.money == money {
            return Some(false);
        }
        trade.money = money;
        trade.accepted = false;
        trade.server_state_index = server_state_index;
        Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::Player;
    use wow_core::ObjectGuid;

    #[test]
    fn player_trade_owner_applies_cpp_trade_data_transitions() {
        let mut player = Player::new(Some(1), false);
        let partner = ObjectGuid::create_player(1, 2);
        let item = ObjectGuid::create_item(1, 77);

        player.open_trade_like_cpp(partner);
        let opened = player.trade_state_snapshot_like_cpp().unwrap();
        assert_eq!(opened.partner_guid, partner);
        assert_eq!(
            (opened.client_state_index, opened.server_state_index),
            (1, 1)
        );

        assert!(player.set_trade_accepted_like_cpp(true));
        assert_eq!(
            player.set_trade_slot_item_like_cpp(0, Some(item), false, 99),
            Some(true)
        );
        let state = player.trade_state_snapshot_like_cpp().unwrap();
        assert_eq!(state.items[0], Some(item));
        assert!(!state.accepted);
        assert_eq!(state.server_state_index, 99);
        // Same item without `update`: C++ returns before touching anything.
        assert_eq!(
            player.set_trade_slot_item_like_cpp(0, Some(item), false, 5),
            Some(false)
        );
        assert_eq!(
            player
                .trade_state_snapshot_like_cpp()
                .unwrap()
                .server_state_index,
            99
        );
        // `update = true` (Item::SetCount) re-sends the same item.
        assert_eq!(
            player.set_trade_slot_item_like_cpp(0, Some(item), true, 6),
            Some(true)
        );
        assert_eq!(
            player.set_trade_slot_item_like_cpp(7, Some(item), false, 6),
            None
        );

        assert_eq!(player.set_trade_money_value_like_cpp(125, 7), Some(true));
        assert_eq!(player.set_trade_money_value_like_cpp(125, 8), Some(false));
        assert_eq!(
            player.set_trade_spell_value_like_cpp(42, Some(item), 9),
            Some(true)
        );
        assert_eq!(
            player.set_trade_spell_value_like_cpp(42, Some(item), 10),
            Some(false)
        );
        let state = player.trade_state_snapshot_like_cpp().unwrap();
        assert_eq!(
            (state.money, state.spell_id, state.server_state_index),
            (125, 42, 9)
        );

        assert!(player.advance_trade_client_state_index_like_cpp());
        assert_eq!(
            player
                .trade_state_snapshot_like_cpp()
                .unwrap()
                .client_state_index,
            2
        );
        assert!(player.set_trade_in_accept_process_like_cpp(true));
        assert!(
            player
                .trade_state_snapshot_like_cpp()
                .unwrap()
                .in_accept_process
        );

        player.clear_trade_like_cpp();
        assert!(player.trade_state_snapshot_like_cpp().is_none());
        assert_eq!(player.set_trade_money_value_like_cpp(1, 1), None);
        assert!(!player.set_trade_accepted_like_cpp(true));
    }
}
