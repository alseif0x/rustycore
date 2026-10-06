// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Guild bank handler family.
//!
//! C++ source of truth: `GuildHandler.cpp` (`HandleGuildBankActivate`,
//! `HandleGuildBankQueryTab`, `HandleGuildBankBuyTab`, `HandleGuildBankUpdateTab`,
//! `HandleGuildBankDepositMoney`, `HandleGuildBankWithdrawMoney`,
//! `HandleGuildBankLogQuery`, `HandleGuildBankTextQuery`,
//! `HandleGuildBankSetTabText`, `HandleAutoGuildBankItem`,
//! `HandleAutoStoreGuildBankItem`). The family owns the packet bodies, the
//! guild-bank GameObject interaction gate and the represented request records;
//! the World session only lends the hub, inventory, social and world-entity
//! state (#1263 F5). Bodies are moved unchanged from the World shell.

use tracing::warn;
use wow_constants::{ClientOpcodes, InventoryResult};
use wow_core::ObjectGuid;
use wow_entities::{GAMEOBJECT_TYPE_GUILD_BANK, INVENTORY_SLOT_BAG_0, NULL_SLOT, is_inventory_pos};
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{
    AutoGuildBankItem, AutoStoreGuildBankItem, GuildBankActivate, GuildBankBuyTab,
    GuildBankDepositMoney, GuildBankLogQuery, GuildBankQueryTab, GuildBankSetTabText,
    GuildBankTextQuery, GuildBankUpdateTab, GuildBankWithdrawMoney, GuildCommandResult,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::HubMut;
use wow_world_entities::WorldEntitiesState;
use wow_world_inventory::{InventoryState, RepresentedGuildBankTabActionKindLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_inventory::{
    RepresentedGuildBankInventoryMoveLikeCpp, RepresentedGuildBankListRequestLikeCpp,
    RepresentedGuildBankMoneyMoveLikeCpp, RepresentedGuildBankTabActionLikeCpp,
};
use wow_world_social::SessionSocialLimits;

/// Borrowed inputs of one guild bank handler invocation.
pub struct GuildBankHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    inventory: &'a mut InventoryState,
    social: &'a SessionSocialLimits,
    world_entities: &'a WorldEntitiesState,
    /// The host's World-test flag (World passes `cfg!(test)`); the represented
    /// request records are World-test evidence only.
    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(dead_code))]
    world_test_consumer: bool,
}

impl<'a> GuildBankHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        inventory: &'a mut InventoryState,
        social: &'a SessionSocialLimits,
        world_entities: &'a WorldEntitiesState,
        world_test_consumer: bool,
    ) -> Self {
        Self {
            hub,
            inventory,
            social,
            world_entities,
            world_test_consumer,
        }
    }

    fn resolved_represented_guild_id_like_cpp(&self) -> Option<u64> {
        wow_world_social::resolved_represented_guild_id_like_cpp(&self.hub.shared(), self.social)
    }

    /// C++ `Player::GetGameObjectIfCanInteractWith(guid, GAMEOBJECT_TYPE_GUILD_BANK)`.
    fn represented_guild_bank_gameobject_can_interact_like_cpp(
        &self,
        banker: ObjectGuid,
    ) -> Option<()> {
        let state = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(banker)?;
        if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_GUILD_BANK) {
            return None;
        }

        self.world_entities
            .represented_gameobject_can_interact_with_like_cpp(self.hub.shared(), banker, 10.0)
            .map(|_| ())
    }

    /// CMSG_GUILD_BANK_ACTIVATE — click a guild-bank GameObject.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankActivate`.
    pub async fn handle_guild_bank_activate(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankActivate::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankActivate parse failed: {error}"
                );
                return;
            }
        };

        if self
            .represented_guild_bank_gameobject_can_interact_like_cpp(packet.banker)
            .is_none()
        {
            return;
        }

        match self.resolved_represented_guild_id_like_cpp() {
            Some(0) => {
                self.hub
                    .shared()
                    .core
                    .send_packet(&GuildCommandResult::player_not_in_guild_view_tab_like_cpp());
                return;
            }
            Some(_) => {}
            None => return,
        }

        let _accepted =
            self.record_guild_bank_list_request_like_cpp(packet.banker, 0, packet.full_update);
    }

    /// CMSG_GUILD_BANK_QUERY_TAB — request a single guild-bank tab.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankQueryTab`.
    pub async fn handle_guild_bank_query_tab(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankQueryTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankQueryTab parse failed: {error}"
                );
                return;
            }
        };

        if self
            .represented_guild_bank_gameobject_can_interact_like_cpp(packet.banker)
            .is_none()
        {
            return;
        }

        if self
            .resolved_represented_guild_id_like_cpp()
            .is_none_or(|guild_id| guild_id == 0)
        {
            return;
        }

        let _accepted =
            self.record_guild_bank_list_request_like_cpp(packet.banker, packet.tab, true);
    }

    /// CMSG_GUILD_BANK_BUY_TAB — buy a guild-bank tab.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankBuyTab`.
    pub async fn handle_guild_bank_buy_tab(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankBuyTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankBuyTab parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_buy_tab_like_cpp(packet.banker, packet.bank_tab);
    }

    /// CMSG_GUILD_BANK_UPDATE_TAB — rename/update a guild-bank tab.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankUpdateTab`.
    pub async fn handle_guild_bank_update_tab(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankUpdateTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankUpdateTab parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_update_tab_like_cpp(
            packet.banker,
            packet.bank_tab,
            packet.name,
            packet.icon,
        );
    }

    /// CMSG_GUILD_BANK_DEPOSIT_MONEY — deposit player money into the guild bank.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankDepositMoney`.
    pub async fn handle_guild_bank_deposit_money(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankDepositMoney::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankDepositMoney parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_money_move_like_cpp(packet.banker, true, packet.money);
    }

    /// CMSG_GUILD_BANK_WITHDRAW_MONEY — withdraw money from the guild bank.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankWithdrawMoney`.
    pub async fn handle_guild_bank_withdraw_money(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankWithdrawMoney::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankWithdrawMoney parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_money_move_like_cpp(packet.banker, false, packet.money);
    }

    /// CMSG_GUILD_BANK_LOG_QUERY — request a guild-bank tab log.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankLogQuery`.
    pub async fn handle_guild_bank_log_query(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankLogQuery::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankLogQuery parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_log_query_like_cpp(packet.tab);
    }

    /// CMSG_GUILD_BANK_TEXT_QUERY — request a guild-bank tab text.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankTextQuery`.
    pub async fn handle_guild_bank_text_query(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankTextQuery::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankTextQuery parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_text_query_like_cpp(packet.tab);
    }

    /// CMSG_GUILD_BANK_SET_TAB_TEXT — update a guild-bank tab text.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankSetTabText`.
    pub async fn handle_guild_bank_set_tab_text(&mut self, mut pkt: WorldPacket) {
        let packet = match GuildBankSetTabText::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "GuildBankSetTabText parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_set_tab_text_like_cpp(packet.tab, packet.tab_text);
    }

    /// CMSG_AUTO_GUILD_BANK_ITEM — move from player inventory into a guild-bank slot.
    ///
    /// C++ ref: `WorldSession::HandleAutoGuildBankItem`.
    pub async fn handle_auto_guild_bank_item(&mut self, mut pkt: WorldPacket) {
        let packet = match AutoGuildBankItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "AutoGuildBankItem parse failed: {error}"
                );
                return;
            }
        };

        let player_bag = packet
            .container_slot
            .unwrap_or(wow_entities::INVENTORY_SLOT_BAG_0);
        let _accepted = self.guild_bank_inventory_move_like_cpp(
            packet.banker,
            false,
            packet.bank_tab,
            packet.bank_slot,
            player_bag,
            packet.container_item_slot,
            0,
        );
    }

    /// CMSG_AUTO_STORE_GUILD_BANK_ITEM — auto-store from a guild-bank slot into inventory.
    ///
    /// C++ ref: `WorldSession::HandleAutoStoreGuildBankItem`.
    pub async fn handle_auto_store_guild_bank_item(&mut self, mut pkt: WorldPacket) {
        let packet = match AutoStoreGuildBankItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "AutoStoreGuildBankItem parse failed: {error}"
                );
                return;
            }
        };

        let _accepted = self.guild_bank_inventory_move_like_cpp(
            packet.banker,
            true,
            packet.bank_tab,
            packet.bank_slot,
            wow_entities::INVENTORY_SLOT_BAG_0,
            wow_entities::NULL_SLOT,
            0,
        );
    }

    fn represented_guild_bank_can_interact_like_cpp(&self, banker: ObjectGuid) -> Option<u64> {
        self.represented_guild_bank_gameobject_can_interact_like_cpp(banker)?;
        let guild_id = self.resolved_represented_guild_id_like_cpp()?;
        (guild_id != 0).then_some(guild_id)
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    fn record_guild_bank_list_request_like_cpp(
        &mut self,
        banker: ObjectGuid,
        tab: u8,
        full_update: bool,
    ) -> bool {
        let Some(guild_id) = self.represented_guild_bank_can_interact_like_cpp(banker) else {
            return false;
        };
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            self.inventory
                .record_represented_guild_bank_list_request_like_cpp(
                    RepresentedGuildBankListRequestLikeCpp {
                        banker,
                        guild_id,
                        tab,
                        full_update,
                    },
                );
        }
        true
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    fn guild_bank_inventory_move_like_cpp(
        &mut self,
        banker: ObjectGuid,
        to_char: bool,
        bank_tab: u8,
        bank_slot: u8,
        player_bag: u8,
        player_slot: u8,
        stack_count: u32,
    ) -> bool {
        let Some(guild_id) = self.represented_guild_bank_can_interact_like_cpp(banker) else {
            return false;
        };

        let auto_store_to_char_slot =
            player_bag == INVENTORY_SLOT_BAG_0 && player_slot == NULL_SLOT;
        if !auto_store_to_char_slot && !is_inventory_pos(player_bag, player_slot) {
            self.hub.shared().core.send_equip_error(
                InventoryResult::InternalBagError,
                None,
                None,
                0,
                0,
            );
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            self.inventory
                .record_represented_guild_bank_inventory_move_like_cpp(
                    RepresentedGuildBankInventoryMoveLikeCpp {
                        banker,
                        guild_id,
                        to_char,
                        bank_tab,
                        bank_slot,
                        player_bag,
                        player_slot,
                        stack_count,
                    },
                );
        }
        true
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    fn guild_bank_money_move_like_cpp(
        &mut self,
        banker: ObjectGuid,
        deposit: bool,
        money: u64,
    ) -> bool {
        if money == 0 {
            return false;
        }

        let Some(guild_id) = self.represented_guild_bank_can_interact_like_cpp(banker) else {
            return false;
        };

        if deposit
            && !self
                .inventory
                .resolved_player_money_like_cpp(self.hub.shared())
                .is_some_and(|player_money| player_money >= money)
        {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            self.inventory
                .record_represented_guild_bank_money_move_like_cpp(
                    RepresentedGuildBankMoneyMoveLikeCpp {
                        banker,
                        guild_id,
                        deposit,
                        money,
                    },
                );
        }
        true
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    fn guild_bank_buy_tab_like_cpp(&mut self, banker: ObjectGuid, tab: u8) -> bool {
        if !banker.is_empty()
            && self
                .represented_guild_bank_gameobject_can_interact_like_cpp(banker)
                .is_none()
        {
            return false;
        }

        let Some(guild_id) = self.resolved_represented_guild_id_like_cpp() else {
            return false;
        };
        if guild_id == 0 {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            self.inventory
                .record_represented_guild_bank_tab_action_like_cpp(
                    RepresentedGuildBankTabActionLikeCpp {
                        banker: (!banker.is_empty()).then_some(banker),
                        guild_id,
                        tab: i32::from(tab),
                        action: RepresentedGuildBankTabActionKindLikeCpp::Buy,
                    },
                );
        }
        true
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    fn guild_bank_update_tab_like_cpp(
        &mut self,
        banker: ObjectGuid,
        tab: u8,
        name: String,
        icon: String,
    ) -> bool {
        if name.is_empty() || icon.is_empty() {
            return false;
        }

        let Some(guild_id) = self.represented_guild_bank_can_interact_like_cpp(banker) else {
            return false;
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            self.inventory
                .record_represented_guild_bank_tab_action_like_cpp(
                    RepresentedGuildBankTabActionLikeCpp {
                        banker: Some(banker),
                        guild_id,
                        tab: i32::from(tab),
                        action: RepresentedGuildBankTabActionKindLikeCpp::Update { name, icon },
                    },
                );
        }
        true
    }

    fn guild_bank_log_query_like_cpp(&mut self, tab: i32) -> bool {
        self.guild_bank_tab_action_without_banker_like_cpp(
            tab,
            RepresentedGuildBankTabActionKindLikeCpp::LogQuery,
        )
    }

    fn guild_bank_text_query_like_cpp(&mut self, tab: i32) -> bool {
        self.guild_bank_tab_action_without_banker_like_cpp(
            tab,
            RepresentedGuildBankTabActionKindLikeCpp::TextQuery,
        )
    }

    fn guild_bank_set_tab_text_like_cpp(&mut self, tab: i32, text: String) -> bool {
        self.guild_bank_tab_action_without_banker_like_cpp(
            tab,
            RepresentedGuildBankTabActionKindLikeCpp::SetText { text },
        )
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    fn guild_bank_tab_action_without_banker_like_cpp(
        &mut self,
        tab: i32,
        action: RepresentedGuildBankTabActionKindLikeCpp,
    ) -> bool {
        let Some(guild_id) = self.resolved_represented_guild_id_like_cpp() else {
            return false;
        };
        if guild_id == 0 {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            self.inventory
                .record_represented_guild_bank_tab_action_like_cpp(
                    RepresentedGuildBankTabActionLikeCpp {
                        banker: None,
                        guild_id,
                        tab,
                        action,
                    },
                );
        }
        true
    }
}

/// Builds a guild bank handler context from a host's state.
pub trait GuildBankHandlerHostLikeCpp<C> {
    fn guild_bank_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> GuildBankHandlerCxLikeCpp<'a>;
}

fn handle_guild_bank_activate_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_activate(pkt)
            .await;
    })
}

fn handle_guild_bank_query_tab_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_query_tab(pkt)
            .await;
    })
}

fn handle_guild_bank_buy_tab_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_buy_tab(pkt)
            .await;
    })
}

fn handle_guild_bank_update_tab_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_update_tab(pkt)
            .await;
    })
}

fn handle_guild_bank_deposit_money_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_deposit_money(pkt)
            .await;
    })
}

fn handle_guild_bank_withdraw_money_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_withdraw_money(pkt)
            .await;
    })
}

fn handle_guild_bank_log_query_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_log_query(pkt)
            .await;
    })
}

fn handle_guild_bank_text_query_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_text_query(pkt)
            .await;
    })
}

fn handle_guild_bank_set_tab_text_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_guild_bank_set_tab_text(pkt)
            .await;
    })
}

fn handle_auto_guild_bank_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_auto_guild_bank_item(pkt)
            .await;
    })
}

fn handle_auto_store_guild_bank_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .guild_bank_handler_cx_like_cpp(catalogs)
            .handle_auto_store_guild_bank_item(pkt)
            .await;
    })
}

/// Registers the guild bank handlers on the packet registry.
pub fn register_guild_bank_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: GuildBankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_activate",
        handler: handle_guild_bank_activate_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankQueryTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_query_tab",
        handler: handle_guild_bank_query_tab_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankBuyTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_buy_tab",
        handler: handle_guild_bank_buy_tab_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankUpdateTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_update_tab",
        handler: handle_guild_bank_update_tab_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankDepositMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_deposit_money",
        handler: handle_guild_bank_deposit_money_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankWithdrawMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_withdraw_money",
        handler: handle_guild_bank_withdraw_money_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankLogQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_log_query",
        handler: handle_guild_bank_log_query_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankTextQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_text_query",
        handler: handle_guild_bank_text_query_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankSetTabText,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_set_tab_text",
        handler: handle_guild_bank_set_tab_text_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AutoGuildBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auto_guild_bank_item",
        handler: handle_auto_guild_bank_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AutoStoreGuildBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auto_store_guild_bank_item",
        handler: handle_auto_store_guild_bank_item_thunk::<S, C>,
    })?;
    Ok(())
}
