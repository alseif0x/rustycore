// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private guild capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    AcceptGuildInvite, AutoGuildBankItem, AutoStoreGuildBankItem, DeclineGuildInvites,
    GuildBankActivate, GuildBankBuyTab, GuildBankDepositMoney, GuildBankLogQuery,
    GuildBankQueryTab, GuildBankSetTabText, GuildBankTextQuery, GuildBankUpdateTab,
    GuildBankWithdrawMoney, GuildCommandResult, GuildSetAchievementTracking,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildSetAchievementTracking,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_set_achievement_tracking",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_set_achievement_tracking(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DeclineGuildInvites,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_decline_guild_invites",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_decline_guild_invites(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildDeclineInvitation,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_decline_invitation",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_decline_invitation(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AcceptGuildInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_accept_guild_invite",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_accept_guild_invite(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankRemainingWithdrawMoneyQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_remaining_withdraw_money_query",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_remaining_withdraw_money_query(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_activate",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_activate(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankQueryTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_query_tab",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_query_tab(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankBuyTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_buy_tab",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_buy_tab(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankUpdateTab,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_update_tab",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_update_tab(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankDepositMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_deposit_money",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_deposit_money(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankWithdrawMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_withdraw_money",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_withdraw_money(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankLogQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_log_query",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_log_query(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankTextQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_text_query",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_text_query(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GuildBankSetTabText,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_guild_bank_set_tab_text",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_guild_bank_set_tab_text(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoGuildBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auto_guild_bank_item",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_auto_guild_bank_item(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoStoreGuildBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auto_store_guild_bank_item",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_auto_store_guild_bank_item(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    pub async fn handle_guild_set_achievement_tracking(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        if let Err(error) = GuildSetAchievementTracking::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "GuildSetAchievementTracking parse failed: {error}"
            );
            return;
        }

        // C++ only delegates when GetPlayer()->GetGuild() resolves a live guild.
        // Rust has no represented guild-achievement manager here yet, so the
        // no-guild branch remains silent.
    }

    pub async fn handle_decline_guild_invites(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match DeclineGuildInvites::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DeclineGuildInvites parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_auto_decline_guild_invites_like_cpp(request.allow);
    }

    pub async fn handle_guild_decline_invitation(&mut self, _pkt: wow_packet::WorldPacket) {
        self.decline_guild_invitation_like_cpp();
    }

    pub async fn handle_accept_guild_invite(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = AcceptGuildInvite::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "AcceptGuildInvite parse failed: {error}"
            );
            return;
        }

        self.accept_guild_invitation_like_cpp();
    }

    pub async fn handle_guild_bank_remaining_withdraw_money_query(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        // C++ only sends GuildBankRemainingWithdrawMoney when GetPlayer()->GetGuild()
        // resolves a live guild. Rust has no represented guild-bank manager here
        // yet, so the no-guild branch is correctly silent.
    }

    /// CMSG_GUILD_BANK_ACTIVATE — click a guild-bank GameObject.
    ///
    /// C++ ref: `WorldSession::HandleGuildBankActivate`.

    pub async fn handle_guild_bank_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankActivate::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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
                self.send_packet(&GuildCommandResult::player_not_in_guild_view_tab_like_cpp());
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

    pub async fn handle_guild_bank_query_tab(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankQueryTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_guild_bank_buy_tab(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankBuyTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_guild_bank_update_tab(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankUpdateTab::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_guild_bank_deposit_money(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankDepositMoney::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_guild_bank_withdraw_money(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankWithdrawMoney::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_guild_bank_log_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankLogQuery::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_guild_bank_text_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankTextQuery::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_guild_bank_set_tab_text(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match GuildBankSetTabText::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_auto_guild_bank_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AutoGuildBankItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_auto_store_guild_bank_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match AutoStoreGuildBankItem::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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
}
