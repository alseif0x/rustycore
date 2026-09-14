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

    /// C++ `WorldSession::HandleInitiateTradeOpcode` creates `m_trade` on both
    /// participants (`TradeHandler.cpp:694-695`).
    pub fn open_trade_like_cpp(&mut self, partner_guid: ObjectGuid) {
        self.gameplay_state_mut().trade = Some(PlayerTradeStateLikeCpp::new(partner_guid));
    }

    /// C++ `Player::TradeCancel` clears `m_trade` (`Player.cpp:12877`).
    pub fn clear_trade_like_cpp(&mut self) {
        self.gameplay_state_mut().trade = None;
    }

    pub fn set_trade_partner_server_state_index_like_cpp(&mut self, state_index: u32) -> bool {
        let Some(trade) = self.gameplay_state_mut().trade.as_mut() else {
            return false;
        };
        trade.partner_server_state_index = state_index;
        true
    }

    pub fn set_trade_accepted_like_cpp(&mut self, accepted: bool) -> bool {
        let Some(trade) = self.gameplay_state_mut().trade.as_mut() else {
            return false;
        };
        trade.accepted = accepted;
        true
    }

    pub fn advance_trade_client_state_index_like_cpp(&mut self) -> bool {
        let Some(trade) = self.gameplay_state_mut().trade.as_mut() else {
            return false;
        };
        trade.client_state_index = trade.client_state_index.wrapping_add(1);
        true
    }

    /// Apply the complete C++ SetTradeGold transition after the session has
    /// checked the represented player's money.  C++ advances the client state
    /// even when the requested amount is unchanged or unaffordable; only an
    /// affordable changed amount clears acceptance and advances server state.
    pub fn set_trade_gold_like_cpp(
        &mut self,
        coinage: u64,
        affordable: bool,
    ) -> Option<PlayerTradeStateLikeCpp> {
        let trade = self.gameplay_state_mut().trade.as_mut()?;
        trade.client_state_index = trade.client_state_index.wrapping_add(1);
        if trade.money != coinage && affordable {
            trade.money = coinage;
            trade.accepted = false;
            trade.server_state_index = trade.server_state_index.wrapping_add(1);
        }
        Some(trade.clone())
    }

    /// Apply C++ `TradeData::SetItem` and its handler-side acceptance reset.
    pub fn set_trade_item_like_cpp(
        &mut self,
        trade_slot: u8,
        item_guid: ObjectGuid,
    ) -> Option<PlayerTradeStateLikeCpp> {
        let slot = usize::from(trade_slot);
        if slot >= PLAYER_TRADE_SLOT_COUNT_LIKE_CPP {
            return None;
        }
        let trade = self.gameplay_state_mut().trade.as_mut()?;
        trade.client_state_index = trade.client_state_index.wrapping_add(1);
        trade.items[slot] = Some(item_guid);
        trade.accepted = false;
        trade.server_state_index = trade.server_state_index.wrapping_add(1);
        Some(trade.clone())
    }

    pub fn clear_trade_item_like_cpp(&mut self, trade_slot: u8) -> Option<PlayerTradeStateLikeCpp> {
        let slot = usize::from(trade_slot);
        if slot >= PLAYER_TRADE_SLOT_COUNT_LIKE_CPP {
            return None;
        }
        let trade = self.gameplay_state_mut().trade.as_mut()?;
        trade.client_state_index = trade.client_state_index.wrapping_add(1);
        if trade.items[slot].is_some() {
            trade.items[slot] = None;
            trade.accepted = false;
            trade.server_state_index = trade.server_state_index.wrapping_add(1);
        }
        Some(trade.clone())
    }

    pub fn set_trade_spell_like_cpp(
        &mut self,
        spell_id: u32,
        cast_item_guid: Option<ObjectGuid>,
    ) -> bool {
        let Some(trade) = self.gameplay_state_mut().trade.as_mut() else {
            return false;
        };
        if trade.spell_id == spell_id && trade.spell_cast_item_guid == cast_item_guid {
            return true;
        }
        trade.spell_id = spell_id;
        trade.spell_cast_item_guid = cast_item_guid;
        trade.accepted = false;
        trade.server_state_index = trade.server_state_index.wrapping_add(1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Player;
    use wow_core::ObjectGuid;

    #[test]
    fn player_trade_owner_applies_complete_named_transitions_like_cpp() {
        let mut player = Player::new(Some(1), false);
        let partner = ObjectGuid::create_player(1, 2);
        let item = ObjectGuid::create_item(1, 77);

        player.open_trade_like_cpp(partner);
        assert_eq!(
            player.trade_state_snapshot_like_cpp().unwrap().partner_guid,
            partner
        );
        assert!(player.set_trade_item_like_cpp(0, item).is_some());
        assert_eq!(
            player.trade_state_snapshot_like_cpp().unwrap().items[0],
            Some(item)
        );
        assert!(player.set_trade_gold_like_cpp(125, true).is_some());
        assert_eq!(player.trade_state_snapshot_like_cpp().unwrap().money, 125);
        assert!(player.set_trade_spell_like_cpp(42, Some(item)));
        assert!(player.set_trade_accepted_like_cpp(true));
        assert!(player.clear_trade_item_like_cpp(0).is_some());
        assert_eq!(
            player.trade_state_snapshot_like_cpp().unwrap().items[0],
            None
        );
        player.clear_trade_like_cpp();
        assert!(player.trade_state_snapshot_like_cpp().is_none());
    }

    #[test]
    fn unaffordable_trade_gold_only_advances_client_state_like_cpp() {
        let mut player = Player::new(Some(1), false);
        player.open_trade_like_cpp(ObjectGuid::create_player(1, 2));
        let before = player.trade_state_snapshot_like_cpp().unwrap();
        let after = player.set_trade_gold_like_cpp(125, false).unwrap();
        assert_eq!(after.money, before.money);
        assert_eq!(after.server_state_index, before.server_state_index);
        assert_eq!(
            after.client_state_index,
            before.client_state_index.wrapping_add(1)
        );
    }
}
