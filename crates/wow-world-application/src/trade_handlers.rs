// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trade handler bodies for the represented trade session.
//!
//! C++ source of truth: `src/server/game/Handlers/TradeHandler.cpp` and
//! `Player::TradeCancel`/`TradeAccept`/`SetTradeItem`/`SetTradeGold` in
//! `Player.cpp`. The canonical trade state already lives on the Player
//! (`wow-entities::PlayerTradeStateLikeCpp`, reached through the Core hub), and
//! the represented partner bookkeeping lives in `wow-world-social`, so the
//! World session only builds the borrowed social/inventory/hub context (#1263
//! F5). The spell-trade body, the duel handlers and the petition handlers stay
//! in the World shell while they need its spell store and pet state.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_entities::{PlayerDuelInfoLikeCpp, PlayerDuelStateLikeCpp, PlayerTradeStateLikeCpp};
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{
    AcceptTrade, BeginTrade, BusyTrade, CanDuel, ClearTradeItem, DeclinePetition, DuelCountdown,
    DuelResponse, EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, IgnoreTrade, QueryPetition,
    QueryPetitionResponse, SetTradeGold, SetTradeItem, SignPetition, TRADE_SLOT_COUNT_LIKE_CPP,
    TRADE_STATUS_ACCEPTED_LIKE_CPP, TRADE_STATUS_CANCELLED_LIKE_CPP,
    TRADE_STATUS_PLAYER_BUSY_LIKE_CPP, TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP,
    TRADE_STATUS_STATE_CHANGED_LIKE_CPP, TRADE_STATUS_UNACCEPTED_LIKE_CPP, TradeStatus,
    UnacceptTrade,
};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket};
use wow_world_core::session::mailbox::{
    CancelRepresentedTradeLikeCppCommand, SendRepresentedTradeStatusLikeCppCommand, SessionCommand,
    UnacceptRepresentedTradeLikeCppCommand,
};
use wow_world_core::session::{HubMut, SessionCore};
use wow_world_inventory::InventoryState;
use wow_world_social::SessionSocialLimits;

/// Reads the canonical trade snapshot, falling back to the represented fixture
/// on a handle-less test session (Core hub plus Social bookkeeping).
pub fn player_trade_state_snapshot_like_cpp(
    core: &SessionCore,
    social: &SessionSocialLimits,
) -> Option<Option<PlayerTradeStateLikeCpp>> {
    let canonical =
        core.with_owned_player_like_cpp(|player| player.trade_state_snapshot_like_cpp());
    #[cfg(any(test, feature = "test-fixtures"))]
    if canonical.is_none() && core.player_handle_like_cpp.is_none() {
        return Some(social.represented_trade_state_for_test_like_cpp());
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = social;
    canonical
}

/// Canonical `Player::SetDuelInfo` through the hub's canonical-player access.
pub fn set_represented_duel_state_like_cpp(
    hub: &mut HubMut<'_>,
    player_guid: ObjectGuid,
    opponent_guid: ObjectGuid,
    state: PlayerDuelStateLikeCpp,
) {
    let _ = hub
        .core
        .mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
            player.set_duel_info_like_cpp(Some(PlayerDuelInfoLikeCpp {
                opponent: opponent_guid,
                state,
            }));
        });
}

/// C++ represented arbiter install through the Social owner.
pub fn set_represented_duel_arbiter_guid_like_cpp(
    hub: &mut HubMut<'_>,
    social: &mut SessionSocialLimits,
    guid: Option<ObjectGuid>,
) {
    social.set_represented_duel_arbiter_guid_like_cpp(hub, guid);
}

/// C++ represented arbiter query through the Social owner.
pub fn resolved_represented_duel_arbiter_guid_like_cpp(
    hub: wow_world_core::session::HubRef<'_>,
    social: &SessionSocialLimits,
) -> Option<Option<ObjectGuid>> {
    social.resolved_represented_duel_arbiter_guid_like_cpp(hub)
}

/// Borrowed inputs of one trade handler invocation.
pub struct TradeHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    social: &'a mut SessionSocialLimits,
    inventory: &'a mut InventoryState,
}

impl<'a> TradeHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        social: &'a mut SessionSocialLimits,
        inventory: &'a mut InventoryState,
    ) -> Self {
        Self {
            hub,
            social,
            inventory,
        }
    }

    /// Canonical Player trade snapshot with the represented fixture fallback.
    pub fn player_trade_state_snapshot_like_cpp(&self) -> Option<Option<PlayerTradeStateLikeCpp>> {
        player_trade_state_snapshot_like_cpp(self.hub.shared().core, self.social)
    }

    /// C++ `Player::TradeData` partner query.
    pub fn resolved_represented_active_trade_partner_like_cpp(&self) -> Option<Option<ObjectGuid>> {
        self.player_trade_state_snapshot_like_cpp()
            .map(|state| state.map(|state| state.partner_guid))
    }

    /// C++ `Player::TradeCancel` partner install/clear.
    pub fn set_represented_active_trade_partner_like_cpp(
        &mut self,
        partner_guid: Option<ObjectGuid>,
    ) -> bool {
        let canonical = partner_guid
            .map(|partner_guid| {
                self.hub.core.with_owned_player_mut_like_cpp(|player| {
                    player.open_trade_like_cpp(partner_guid)
                })
            })
            .unwrap_or_else(|| {
                self.hub
                    .core
                    .with_owned_player_mut_like_cpp(|player| player.clear_trade_like_cpp())
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.hub.shared().core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_trade_state_like_cpp(|state| {
                    *state = partner_guid.map(PlayerTradeStateLikeCpp::new);
                })
                .is_some();
        }
        canonical
    }

    /// C++ `Player::TradeCancel` clear.
    pub fn clear_represented_active_trade_partner_like_cpp(&mut self) -> bool {
        let canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.clear_trade_like_cpp())
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.hub.shared().core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_trade_state_like_cpp(|state| *state = None)
                .is_some();
        }
        canonical
    }

    /// C++ `Player::SetTradeAccepted`.
    pub fn set_represented_trade_accepted_like_cpp_for_command(&mut self, accepted: bool) -> bool {
        let canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_trade_accepted_like_cpp(accepted))
            .is_some_and(|changed| changed);
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.hub.shared().core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_trade_state_like_cpp(|state| {
                    if let Some(state) = state {
                        state.accepted = accepted;
                    }
                })
                .is_some();
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_player_trade_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut Option<PlayerTradeStateLikeCpp>) -> R,
    ) -> Option<R> {
        let mut state = self.player_trade_state_snapshot_like_cpp()?;
        let result = mutate(&mut state);
        let canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.install_trade_state_like_cpp(state.clone())
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.hub.shared().core.player_handle_like_cpp.is_none() {
            self.social
                .set_represented_trade_state_for_test_like_cpp(state);
            return Some(result);
        }
        canonical.then_some(result)
    }

    fn advance_trade_client_state_index_like_cpp(&mut self) {
        let _canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.advance_trade_client_state_index_like_cpp()
            })
            .is_some_and(|changed| changed);
        #[cfg(any(test, feature = "test-fixtures"))]
        if !_canonical && self.hub.shared().core.player_handle_like_cpp.is_none() {
            let _ = self.mutate_player_trade_state_like_cpp(|state| {
                if let Some(state) = state {
                    state.client_state_index = state.client_state_index.wrapping_add(1);
                }
            });
        }
    }

    fn send_unaccept_to_partner_like_cpp(&self, partner_guid: ObjectGuid, packet_bytes: Vec<u8>) {
        self.hub
            .shared()
            .core
            .try_send_connected_player_command_like_cpp(
                partner_guid,
                SessionCommand::UnacceptRepresentedTradeLikeCpp(
                    UnacceptRepresentedTradeLikeCppCommand { packet_bytes },
                ),
            );
    }

    /// CMSG_CLEAR_TRADE_ITEM — clear one offered slot or advance the client index.
    pub fn clear_represented_trade_item_like_cpp(&mut self, trade_slot: u8) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if trade_slot >= TRADE_SLOT_COUNT_LIKE_CPP {
            self.advance_trade_client_state_index_like_cpp();
            return;
        }

        let slot = trade_slot as usize;
        if trade.items[slot].is_none() {
            self.advance_trade_client_state_index_like_cpp();
            return;
        }

        let canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.clear_trade_item_like_cpp(trade_slot))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        let canonical = if !canonical && self.hub.shared().core.player_handle_like_cpp.is_none() {
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
        self.hub.shared().core.send_raw_packet(&packet_bytes);

        self.send_unaccept_to_partner_like_cpp(partner_guid, packet_bytes);
    }

    /// CMSG_SET_TRADE_ITEM — offer one carried item in a trade slot.
    pub fn set_represented_trade_item_like_cpp(
        &mut self,
        trade_slot: u8,
        pack_slot: u8,
        item_slot_in_pack: u8,
    ) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if trade_slot >= TRADE_SLOT_COUNT_LIKE_CPP {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.hub.shared().core.send_raw_packet(&packet_bytes);
            return;
        }

        let Some(item) = self.inventory.get_inventory_item_by_pos(
            self.hub.shared(),
            pack_slot,
            item_slot_in_pack,
        ) else {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.hub.shared().core.send_raw_packet(&packet_bytes);
            return;
        };

        if trade.items.contains(&Some(item.guid)) {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.hub.shared().core.send_raw_packet(&packet_bytes);
            return;
        }

        let canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_trade_item_like_cpp(trade_slot, item.guid)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        let canonical = if !canonical && self.hub.shared().core.player_handle_like_cpp.is_none() {
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
        self.hub.shared().core.send_raw_packet(&packet_bytes);

        self.send_unaccept_to_partner_like_cpp(partner_guid, packet_bytes);
    }

    fn set_represented_trade_gold_state_like_cpp(
        &mut self,
        coinage: u64,
        affordable: bool,
    ) -> bool {
        let canonical = self
            .hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_trade_gold_like_cpp(coinage, affordable)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.hub.shared().core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_trade_state_like_cpp(|state| {
                    let Some(state) = state else { return };
                    state.client_state_index = state.client_state_index.wrapping_add(1);
                    if state.money != coinage && affordable {
                        state.money = coinage;
                        state.accepted = false;
                        state.server_state_index = state.server_state_index.wrapping_add(1);
                    }
                })
                .is_some();
        }
        canonical
    }

    /// CMSG_SET_TRADE_GOLD — offer coinage, refunding when unaffordable.
    pub fn set_represented_trade_gold_like_cpp(&mut self, coinage: u64) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if trade.money == coinage {
            let _ = self.set_represented_trade_gold_state_like_cpp(coinage, true);
            return;
        }

        let affordable = self
            .inventory
            .resolved_player_money_like_cpp(self.hub.shared())
            .is_some_and(|player_money| player_money >= coinage);
        if !affordable {
            if !self.set_represented_trade_gold_state_like_cpp(coinage, false) {
                return;
            }
            let packet_bytes =
                TradeStatus::failed_like_cpp(EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP, 0).to_bytes();
            self.hub.shared().core.send_raw_packet(&packet_bytes);
            return;
        }

        if !self.set_represented_trade_gold_state_like_cpp(coinage, true) {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_UNACCEPTED_LIKE_CPP).to_bytes();
        self.hub.shared().core.send_raw_packet(&packet_bytes);

        self.send_unaccept_to_partner_like_cpp(partner_guid, packet_bytes);
    }

    /// C++ `Player::TradeAccept`.
    pub fn accept_represented_trade_like_cpp(&mut self, state_index: u32) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if trade.partner_server_state_index != state_index {
            let _ = self.set_represented_trade_accepted_like_cpp_for_command(false);
            let packet_bytes =
                TradeStatus::status_only_like_cpp(TRADE_STATUS_STATE_CHANGED_LIKE_CPP).to_bytes();
            self.hub.shared().core.send_raw_packet(&packet_bytes);
            return;
        }

        if !self.set_represented_trade_accepted_like_cpp_for_command(true) {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_ACCEPTED_LIKE_CPP).to_bytes();
        self.hub
            .shared()
            .core
            .try_send_connected_player_command_like_cpp(
                partner_guid,
                SessionCommand::SendRepresentedTradeStatusLikeCpp(
                    SendRepresentedTradeStatusLikeCppCommand { packet_bytes },
                ),
            );
    }

    /// C++ `Player::TradeUnaccept`.
    pub fn unaccept_represented_trade_like_cpp(&mut self) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if !self.set_represented_trade_accepted_like_cpp_for_command(false) {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_UNACCEPTED_LIKE_CPP).to_bytes();
        self.hub
            .shared()
            .core
            .try_send_connected_player_command_like_cpp(
                partner_guid,
                SessionCommand::SendRepresentedTradeStatusLikeCpp(
                    SendRepresentedTradeStatusLikeCppCommand { packet_bytes },
                ),
            );
    }

    /// C++ `Player::TradeCancel`.
    pub fn cancel_represented_trade_like_cpp(&mut self, status: u8, sendback: bool) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        let packet_bytes = TradeStatus::cancel_like_cpp(status).to_bytes();
        self.social.record_represented_trade_cancel_like_cpp(status);
        if !self.clear_represented_active_trade_partner_like_cpp() {
            return;
        }

        if sendback {
            self.hub.shared().core.send_raw_packet(&packet_bytes);
        }

        self.hub
            .shared()
            .core
            .try_send_connected_player_command_like_cpp(
                partner_guid,
                SessionCommand::CancelRepresentedTradeLikeCpp(
                    CancelRepresentedTradeLikeCppCommand {
                        status,
                        packet_bytes,
                    },
                ),
            );
    }

    /// CMSG_BEGIN_TRADE — open the local side of an accepted trade.
    pub fn begin_represented_trade_like_cpp(&mut self) {
        let Some(Some(trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        let packet_bytes = TradeStatus::initiated_like_cpp(0).to_bytes();
        self.hub.shared().core.send_raw_packet(&packet_bytes);

        self.hub
            .shared()
            .core
            .try_send_connected_player_command_like_cpp(
                partner_guid,
                SessionCommand::SendRepresentedTradeStatusLikeCpp(
                    SendRepresentedTradeStatusLikeCppCommand { packet_bytes },
                ),
            );
    }

    /// CMSG_CANCEL_TRADE / CMSG_BUSY_TRADE / CMSG_IGNORE_TRADE.
    pub fn cancel_with_status_like_cpp(&mut self, status: u8) {
        self.cancel_represented_trade_like_cpp(status, true);
    }

    /// CMSG_ACCEPT_TRADE.
    pub async fn handle_accept_trade(&mut self, pkt: WorldPacket) {
        let mut pkt = pkt;
        let packet = match AcceptTrade::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "AcceptTrade parse failed: {error}"
                );
                return;
            }
        };
        self.accept_represented_trade_like_cpp(packet.state_index);
    }

    /// CMSG_CLEAR_TRADE_ITEM.
    pub async fn handle_clear_trade_item(&mut self, pkt: WorldPacket) {
        let mut pkt = pkt;
        let packet = match ClearTradeItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "ClearTradeItem parse failed: {error}"
                );
                return;
            }
        };
        self.clear_represented_trade_item_like_cpp(packet.trade_slot);
    }

    /// CMSG_SET_TRADE_ITEM.
    pub async fn handle_set_trade_item(&mut self, pkt: WorldPacket) {
        let mut pkt = pkt;
        let packet = match SetTradeItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "SetTradeItem parse failed: {error}"
                );
                return;
            }
        };
        self.set_represented_trade_item_like_cpp(
            packet.trade_slot,
            packet.pack_slot,
            packet.item_slot_in_pack,
        );
    }

    /// CMSG_SET_TRADE_GOLD.
    pub async fn handle_set_trade_gold(&mut self, pkt: WorldPacket) {
        let mut pkt = pkt;
        let packet = match SetTradeGold::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "SetTradeGold parse failed: {error}"
                );
                return;
            }
        };
        self.set_represented_trade_gold_like_cpp(packet.coinage);
    }

    /// CMSG_UNACCEPT_TRADE.
    pub async fn handle_unaccept_trade(&mut self, pkt: WorldPacket) {
        let mut pkt = pkt;
        if let Err(error) = UnacceptTrade::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "UnacceptTrade parse failed: {error}"
            );
            return;
        }
        self.unaccept_represented_trade_like_cpp();
    }

    /// CMSG_BEGIN_TRADE.
    pub async fn handle_begin_trade(&mut self, pkt: WorldPacket) {
        let mut pkt = pkt;
        if let Err(error) = BeginTrade::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "BeginTrade parse failed: {error}"
            );
            return;
        }
        self.begin_represented_trade_like_cpp();
    }

    /// CMSG_BUSY_TRADE.
    pub async fn handle_busy_trade(&mut self, pkt: WorldPacket) {
        let mut pkt = pkt;
        if let Err(error) = BusyTrade::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "BusyTrade parse failed: {error}"
            );
            return;
        }
        self.cancel_with_status_like_cpp(TRADE_STATUS_PLAYER_BUSY_LIKE_CPP);
    }

    /// CMSG_IGNORE_TRADE.
    pub async fn handle_ignore_trade(&mut self, pkt: WorldPacket) {
        let mut pkt = pkt;
        if let Err(error) = IgnoreTrade::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "IgnoreTrade parse failed: {error}"
            );
            return;
        }
        self.cancel_with_status_like_cpp(TRADE_STATUS_PLAYER_IGNORED_LIKE_CPP);
    }

    /// CMSG_CANCEL_TRADE.
    pub async fn handle_cancel_trade(&mut self, _pkt: WorldPacket) {
        // C++ calls Player::TradeCancel(true) for a present player; TradeCancel
        // itself is a no-op when no active TradeData exists.
        self.cancel_with_status_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP);
    }

    /// CMSG_SIGN_PETITION — record the represented signature evidence.
    pub fn handle_sign_petition(&mut self, mut pkt: WorldPacket) {
        let packet = match SignPetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "SignPetition parse failed: {error}"
                );
                return;
            }
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        self.social
            .record_represented_sign_petition_for_test_like_cpp(
                wow_world_social::RepresentedSignPetitionLikeCpp {
                    petition_guid: packet.petition_guid,
                    choice: packet.choice,
                },
            );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (packet.petition_guid, packet.choice);
    }

    /// CMSG_DECLINE_PETITION — record the represented decline evidence.
    pub fn handle_decline_petition(&mut self, mut pkt: WorldPacket) {
        let packet = match DeclinePetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "DeclinePetition parse failed: {error}"
                );
                return;
            }
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        self.social
            .record_represented_decline_petition_for_test_like_cpp(
                wow_world_social::RepresentedDeclinePetitionLikeCpp {
                    petition_guid: packet.petition_guid,
                },
            );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = packet.petition_guid;
    }

    /// CMSG_QUERY_PETITION — record the evidence and answer not-found.
    pub fn handle_query_petition(&mut self, mut pkt: WorldPacket) {
        let packet = match QueryPetition::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "QueryPetition parse failed: {error}"
                );
                return;
            }
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        self.social
            .record_represented_query_petition_for_test_like_cpp(
                wow_world_social::RepresentedQueryPetitionLikeCpp {
                    petition_id: packet.petition_id,
                    item_guid: packet.item_guid,
                },
            );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = (packet.petition_id, packet.item_guid);

        self.hub
            .shared()
            .core
            .send_packet(&QueryPetitionResponse::not_found_like_cpp(packet.item_guid));
    }

    /// CMSG_CAN_DUEL — validate a duel request and answer the client.
    pub fn handle_can_duel(&mut self, mut pkt: WorldPacket) {
        let packet = match CanDuel::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "CanDuel parse failed: {error}"
                );
                return;
            }
        };

        self.social.handle_can_duel_like_cpp(
            &mut self.hub,
            packet.target_guid,
            packet.to_the_death,
        );
    }

    /// CMSG_DUEL_RESPONSE — accept or cancel a represented duel request.
    pub fn handle_duel_response(&mut self, mut pkt: WorldPacket) {
        let packet = match DuelResponse::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "DuelResponse parse failed: {error}"
                );
                return;
            }
        };

        if packet.accepted && !packet.forfeited {
            let _ = self.handle_duel_accepted_like_cpp(packet.arbiter_guid);
        } else {
            self.social.handle_duel_cancelled_like_cpp(&mut self.hub);
        }
    }

    fn handle_duel_accepted_like_cpp(&mut self, arbiter_guid: ObjectGuid) -> bool {
        let Some(player_guid) = self.hub.shared().core.player_guid() else {
            return false;
        };
        if resolved_represented_duel_arbiter_guid_like_cpp(self.hub.shared(), self.social)
            != Some(Some(arbiter_guid))
        {
            return false;
        }

        let Some(duel) = self
            .social
            .represented_current_duel_info_like_cpp(&mut self.hub)
        else {
            return false;
        };
        if duel.state != PlayerDuelStateLikeCpp::Challenged {
            return false;
        }

        let opponent_guid = duel.opponent;
        let Some(opponent_duel) = self
            .social
            .represented_duel_opponent_info_like_cpp(&mut self.hub, opponent_guid)
        else {
            return false;
        };
        if opponent_duel.opponent != player_guid {
            return false;
        }

        set_represented_duel_state_like_cpp(
            &mut self.hub,
            player_guid,
            opponent_guid,
            PlayerDuelStateLikeCpp::Countdown,
        );
        set_represented_duel_state_like_cpp(
            &mut self.hub,
            opponent_guid,
            player_guid,
            PlayerDuelStateLikeCpp::Countdown,
        );

        let packet_bytes = DuelCountdown {
            countdown_ms: duel_countdown_ms_like_cpp(),
        }
        .to_bytes();
        self.hub.shared().core.send_raw_packet(&packet_bytes);
        self.social
            .send_represented_duel_countdown_to_opponent_like_cpp(
                self.hub.shared(),
                opponent_guid,
                packet_bytes,
            );
        #[cfg(any(test, feature = "test-fixtures"))]
        self.social
            .record_represented_duel_accept_for_test_like_cpp(
                wow_world_social::RepresentedDuelAcceptedLikeCpp {
                    opponent_guid,
                    arbiter_guid,
                    countdown_ms: duel_countdown_ms_like_cpp(),
                },
            );
        true
    }
}

/// C++ `DUEL_COUNTDOWN` before `SMSG_DUEL_COUNTDOWN`.
fn duel_countdown_ms_like_cpp() -> u32 {
    wow_world_social::DUEL_COUNTDOWN_MS_LIKE_CPP
}

/// Builds a trade handler context from a host's social and inventory state.
pub trait TradeHandlerHostLikeCpp<C> {
    fn trade_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> TradeHandlerCxLikeCpp<'a>;
}

fn cancel_trade_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_cancel_trade(pkt)
            .await;
    })
}

fn accept_trade_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_accept_trade(pkt)
            .await;
    })
}

fn clear_trade_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_clear_trade_item(pkt)
            .await;
    })
}

fn set_trade_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_set_trade_item(pkt)
            .await;
    })
}

fn set_trade_gold_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_set_trade_gold(pkt)
            .await;
    })
}

fn unaccept_trade_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_unaccept_trade(pkt)
            .await;
    })
}

fn busy_trade_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_busy_trade(pkt)
            .await;
    })
}

fn begin_trade_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_begin_trade(pkt)
            .await;
    })
}

fn ignore_trade_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_ignore_trade(pkt)
            .await;
    })
}

fn handle_can_duel_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_can_duel(pkt);
    })
}

fn handle_duel_response_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_duel_response(pkt);
    })
}

fn handle_sign_petition_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_sign_petition(pkt);
    })
}

fn handle_decline_petition_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_decline_petition(pkt);
    })
}

fn handle_query_petition_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .trade_handler_cx_like_cpp(catalogs)
            .handle_query_petition(pkt);
    })
}

/// Registers the trade handlers on the packet registry.
pub fn register_trade_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: TradeHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CancelTrade,
        status: SessionStatus::LoggedInOrRecentlyLogout,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_cancel_trade",
        handler: cancel_trade_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_trade",
        handler: accept_trade_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ClearTradeItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_clear_trade_item",
        handler: clear_trade_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_item",
        handler: set_trade_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetTradeGold,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_trade_gold",
        handler: set_trade_gold_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UnacceptTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_unaccept_trade",
        handler: unaccept_trade_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BusyTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_busy_trade",
        handler: busy_trade_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BeginTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_begin_trade",
        handler: begin_trade_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::IgnoreTrade,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_ignore_trade",
        handler: ignore_trade_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::CanDuel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_can_duel",
        handler: handle_can_duel_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DuelResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_duel_response",
        handler: handle_duel_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SignPetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_sign_petition",
        handler: handle_sign_petition_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DeclinePetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_decline_petition",
        handler: handle_decline_petition_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPetition,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_petition",
        handler: handle_query_petition_thunk::<S, C>,
    })?;
    Ok(())
}
