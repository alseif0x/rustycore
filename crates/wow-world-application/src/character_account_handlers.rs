// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character/account packet-handler registrations (#1263 F5 remaining families).
//!
//! This is the explicit area registrar that replaces the legacy
//! `inventory::submit!` submissions that lived in
//! `crates/wow-world/src/handlers/character/account/registrations/`
//! (`world_services.rs` 14, `inventory_actions.rs` 6, `world_queries.rs` 2,
//! `logout.rs` 1 = **23** effective entries). Only the registration source moved:
//! every entry keeps its opcode, `handler_name`, `SessionStatus`,
//! `PacketProcessing`, packet read and warning text exactly as the shared
//! registry submitted them, and no handler body changed.
//!
//! **Owner.** The application crate owns this family because it is the area that
//! coordinates the logged-in character's interaction and self-service surface for
//! these opcodes, and it already owns their application-side rules
//! ([`crate::vendor`], [`crate::bank`], [`crate::inventory_move_planning`],
//! [`crate::inventory_swap`], [`crate::trainer_purchase`]). C++ source of truth at
//! reference SHA `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`; the C++ opcode table
//! (`src/server/game/Server/Protocol/Opcodes.cpp`) fixes `STATUS_LOGGEDIN` and
//! `PROCESS_INPLACE`/`PROCESS_THREADUNSAFE` for all 23, and the Rust entries
//! reproduce those exactly:
//!
//! - `src/server/game/Handlers/ItemHandler.cpp` (11): `HandleSwapInvItemOpcode`
//!   `:69`, `HandleAutoEquipItemSlotOpcode` `:114`, `HandleSwapItem` `:130`,
//!   `HandleAutoEquipItemOpcode` `:175`, `HandleDestroyItemOpcode` `:294`,
//!   `HandleSellItemOpcode` `:365`, `HandleBuybackItem` `:487`,
//!   `HandleBuyItemOpcode` `:530`, `HandleListInventoryOpcode` `:567`,
//!   `HandleAutoStoreBagItemOpcode` `:699`, `HandleItemRefund` `:1132` — the
//!   translation unit that owns the merchant transaction and the player's own
//!   item-relocation surface (`CMSG_LIST_INVENTORY` `Opcodes.cpp:574`,
//!   `CMSG_BUY_ITEM` `:255`, `CMSG_BUY_BACK_ITEM` `:253`, `CMSG_SELL_ITEM` `:869`,
//!   `CMSG_ITEM_PURCHASE_REFUND` `:549`, `CMSG_SWAP_INV_ITEM` `:959`,
//!   `CMSG_SWAP_ITEM` `:960`, `CMSG_AUTO_EQUIP_ITEM` `:202`,
//!   `CMSG_AUTO_EQUIP_ITEM_SLOT` `:203`, `CMSG_AUTO_STORE_BAG_ITEM` `:205`,
//!   `CMSG_DESTROY_ITEM` `:420`).
//! - `src/server/game/Handlers/BankHandler.cpp` (4): `HandleAutoBankItemOpcode`
//!   `:27`, `HandleBankerActivateOpcode` `:60`, `HandleAutoStoreBankItemOpcode`
//!   `:78`, `HandleBuyBankSlotOpcode` `:122` (`Opcodes.cpp:198/:211/:200/:254`) —
//!   the bank interaction surface, already an application owner.
//! - `src/server/game/Handlers/NPCHandler.cpp` (4): `HandleGossipHelloOpcode`
//!   `:204`, `HandleGossipSelectOptionOpcode` `:250`,
//!   `HandleBinderActivateOpcode` `:373`, `HandleRepairItemOpcode` `:440`
//!   (`Opcodes.cpp:965/:495/:246/:809`) — NPC gossip, binder and repairer
//!   admission and publication.
//! - `src/server/game/Handlers/QuestHandler.cpp` (2):
//!   `HandleQuestgiverStatusMultipleQuery` `:770`,
//!   `HandleQuestgiverStatusTrackedQueryOpcode` `:775`
//!   (`Opcodes.cpp:782/:784`) — quest-giver status publication for the
//!   interacting NPC.
//! - `src/server/game/Handlers/BattleGroundHandler.cpp:703`
//!   `HandleHearthAndResurrect` (`Opcodes.cpp:539`, 1 entry).
//! - `src/server/game/Handlers/MiscHandler.cpp:238` `HandleLogoutRequestOpcode`
//!   (`Opcodes.cpp:583`, 1 entry).
//!
//! The handler bodies stay in their existing owners under
//! `crates/wow-world/src/handlers/character/`.

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::{gossip, item, misc};

/// The narrow host entry points this registration family needs.
///
/// The World session owns the represented character, its interaction target and
/// the publication of both; this crate reaches those operations only through
/// these calls. `C` is the session's catalog view, borrowed exactly as the legacy
/// registration closures borrowed it, so no catalog type leaks into this crate.
pub trait CharacterAccountHandlerHostLikeCpp<C> {
    /// C++ `ItemHandler.cpp:567` `HandleListInventoryOpcode`.
    fn list_inventory_like_cpp<'a>(&'a mut self, hello: gossip::Hello) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:530` `HandleBuyItemOpcode`.
    fn buy_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        buy: misc::BuyItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:487` `HandleBuybackItem`.
    fn buy_back_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        buyback: misc::BuyBackItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:365` `HandleSellItemOpcode`.
    fn sell_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        sell: misc::SellItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:1132` `HandleItemRefund`.
    fn item_purchase_refund_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        refund: item::ItemPurchaseRefund,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `BankHandler.cpp:60` `HandleBankerActivateOpcode`.
    fn banker_activate_like_cpp<'a>(&'a mut self, hello: gossip::Hello) -> HandlerFuture<'a, ()>;

    /// C++ `BankHandler.cpp:27` `HandleAutoBankItemOpcode`.
    fn autobank_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        packet: misc::AutoBankItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `BankHandler.cpp:78` `HandleAutoStoreBankItemOpcode`.
    fn autostore_bank_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        packet: misc::AutoStoreBankItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `BankHandler.cpp:122` `HandleBuyBankSlotOpcode`.
    fn buy_bank_slot_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        buy: misc::BuyBankSlot,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `NPCHandler.cpp:373` `HandleBinderActivateOpcode`.
    fn binder_activate_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        hello: gossip::Hello,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `BattleGroundHandler.cpp:703` `HandleHearthAndResurrect`, which
    /// consumes the raw packet without decoding it.
    fn hearth_and_resurrect_like_cpp<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()>;

    /// C++ `NPCHandler.cpp:440` `HandleRepairItemOpcode`.
    fn repair_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        repair: misc::RepairItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:770` `HandleQuestgiverStatusMultipleQuery`, which
    /// reads no packet body.
    fn quest_giver_status_multiple_query_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:775` `HandleQuestgiverStatusTrackedQueryOpcode`.
    fn quest_giver_status_tracked_query_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:69` `HandleSwapInvItemOpcode`.
    fn swap_inv_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        swap: item::SwapInvItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:175` `HandleAutoEquipItemOpcode`.
    fn auto_equip_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        equip: item::AutoEquipItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:114` `HandleAutoEquipItemSlotOpcode`.
    fn auto_equip_item_slot_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        equip: item::AutoEquipItemSlot,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:130` `HandleSwapItem`.
    fn swap_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        swap: item::SwapItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:699` `HandleAutoStoreBagItemOpcode`.
    fn auto_store_bag_item_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        store: item::AutoStoreBagItem,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `ItemHandler.cpp:294` `HandleDestroyItemOpcode`.
    fn destroy_item_like_cpp<'a>(
        &'a mut self,
        destroy: item::DestroyItemPkt,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `NPCHandler.cpp:204` `HandleGossipHelloOpcode`.
    fn gossip_hello_like_cpp<'a>(&'a mut self, hello: gossip::Hello) -> HandlerFuture<'a, ()>;

    /// C++ `NPCHandler.cpp:250` `HandleGossipSelectOptionOpcode`.
    fn gossip_select_option_like_cpp<'a>(
        &'a mut self,
        select: gossip::GossipSelectOption,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `MiscHandler.cpp:238` `HandleLogoutRequestOpcode`.
    fn logout_request_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        req: misc::LogoutRequest,
    ) -> HandlerFuture<'a, ()>;
}

fn list_inventory_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match gossip::Hello::read(&mut pkt) {
            Ok(hello) => session.list_inventory_like_cpp(hello).await,
            Err(e) => tracing::warn!("Failed to read ListInventory: {e}"),
        }
    })
}

fn buy_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match misc::BuyItem::read(&mut pkt) {
            Ok(buy) => session.buy_item_like_cpp(catalogs, buy).await,
            Err(e) => tracing::warn!("Failed to read BuyItem: {e}"),
        }
    })
}

fn buy_back_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match misc::BuyBackItem::read(&mut pkt) {
            Ok(buyback) => session.buy_back_item_like_cpp(catalogs, buyback).await,
            Err(e) => tracing::warn!("Failed to read BuyBackItem: {e}"),
        }
    })
}

fn sell_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match misc::SellItem::read(&mut pkt) {
            Ok(sell) => session.sell_item_like_cpp(catalogs, sell).await,
            Err(e) => tracing::warn!("Failed to read SellItem: {e}"),
        }
    })
}

fn item_purchase_refund_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match item::ItemPurchaseRefund::read(&mut pkt) {
            Ok(refund) => {
                session
                    .item_purchase_refund_like_cpp(catalogs, refund)
                    .await
            }
            Err(e) => tracing::warn!("Failed to read ItemPurchaseRefund: {e}"),
        }
    })
}

fn banker_activate_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match gossip::Hello::read(&mut pkt) {
            Ok(hello) => session.banker_activate_like_cpp(hello).await,
            Err(e) => tracing::warn!("Failed to read BankerActivate: {e}"),
        }
    })
}

fn autobank_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match misc::AutoBankItem::read(&mut pkt) {
            Ok(packet) => session.autobank_item_like_cpp(catalogs, packet).await,
            Err(e) => tracing::warn!("Failed to read AutobankItem: {e}"),
        }
    })
}

fn autostore_bank_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match misc::AutoStoreBankItem::read(&mut pkt) {
            Ok(packet) => session.autostore_bank_item_like_cpp(catalogs, packet).await,
            Err(e) => tracing::warn!("Failed to read AutostoreBankItem: {e}"),
        }
    })
}

fn buy_bank_slot_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match misc::BuyBankSlot::read(&mut pkt) {
            Ok(buy) => session.buy_bank_slot_like_cpp(catalogs, buy).await,
            Err(e) => tracing::warn!("Failed to read BuyBankSlot: {e}"),
        }
    })
}

fn binder_activate_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match gossip::Hello::read(&mut pkt) {
            Ok(hello) => session.binder_activate_like_cpp(catalogs, hello).await,
            Err(e) => tracing::warn!("Failed to read BinderActivate: {e}"),
        }
    })
}

/// C++ `BattleGroundHandler.cpp:703` consumes the raw packet, so this thunk
/// performs no read and no warning text exists to keep.
fn hearth_and_resurrect_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move { session.hearth_and_resurrect_like_cpp(pkt).await })
}

fn repair_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match misc::RepairItem::read(&mut pkt) {
            Ok(repair) => session.repair_item_like_cpp(catalogs, repair).await,
            Err(e) => tracing::warn!("Failed to read RepairItem: {e}"),
        }
    })
}

/// C++ `QuestHandler.cpp:770` reads no packet body; the catalog carries the
/// quest-info store the publication reads from.
fn quest_giver_status_multiple_query_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    _pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .quest_giver_status_multiple_query_like_cpp(catalogs)
            .await
    })
}

/// C++ `QuestHandler.cpp:775` passes the raw packet on, because the publication
/// reads the tracked GUID list itself.
fn quest_giver_status_tracked_query_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .quest_giver_status_tracked_query_like_cpp(catalogs, pkt)
            .await
    })
}

fn swap_inv_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match item::SwapInvItem::read(&mut pkt) {
            Ok(swap) => session.swap_inv_item_like_cpp(catalogs, swap).await,
            Err(e) => tracing::warn!("Failed to read SwapInvItem: {e}"),
        }
    })
}

fn auto_equip_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match item::AutoEquipItem::read(&mut pkt) {
            Ok(equip) => session.auto_equip_item_like_cpp(catalogs, equip).await,
            Err(e) => tracing::warn!("Failed to read AutoEquipItem: {e}"),
        }
    })
}

fn auto_equip_item_slot_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match item::AutoEquipItemSlot::read(&mut pkt) {
            Ok(equip) => session.auto_equip_item_slot_like_cpp(catalogs, equip).await,
            Err(e) => tracing::warn!("Failed to read AutoEquipItemSlot: {e}"),
        }
    })
}

fn swap_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match item::SwapItem::read(&mut pkt) {
            Ok(swap) => session.swap_item_like_cpp(catalogs, swap).await,
            Err(e) => tracing::warn!("Failed to read SwapItem: {e}"),
        }
    })
}

fn auto_store_bag_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match item::AutoStoreBagItem::read(&mut pkt) {
            Ok(store) => session.auto_store_bag_item_like_cpp(catalogs, store).await,
            Err(e) => tracing::warn!("Failed to read AutoStoreBagItem: {e}"),
        }
    })
}

fn destroy_item_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match item::DestroyItemPkt::read(&mut pkt) {
            Ok(destroy) => session.destroy_item_like_cpp(destroy).await,
            Err(e) => tracing::warn!("Failed to read DestroyItem: {e}"),
        }
    })
}

fn gossip_hello_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match gossip::Hello::read(&mut pkt) {
            Ok(hello) => session.gossip_hello_like_cpp(hello).await,
            Err(e) => tracing::warn!("Failed to read TalkToGossip: {e}"),
        }
    })
}

fn gossip_select_option_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match gossip::GossipSelectOption::read(&mut pkt) {
            Ok(select) => session.gossip_select_option_like_cpp(select).await,
            Err(e) => tracing::warn!("Failed to read GossipSelectOption: {e}"),
        }
    })
}

fn logout_request_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match misc::LogoutRequest::read(&mut pkt) {
            Ok(req) => session.logout_request_like_cpp(catalogs, req).await,
            Err(e) => tracing::warn!("Failed to read LogoutRequest: {e}"),
        }
    })
}

/// Register the 23 character/account opcodes that previously reached the shared
/// registry through the legacy inventory drain.
///
/// The per-entry order follows the four legacy registration files so the
/// migration can be reviewed one to one; the registry is opcode-keyed and the
/// reviewed contract row set is unaffected by statement order.
pub fn register_character_account_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: CharacterAccountHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    // ── world_services.rs (14) ──────────────────────────────────────────
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ListInventory,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_list_inventory",
        handler: list_inventory_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BuyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_item",
        handler: buy_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BuyBackItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_back_item",
        handler: buy_back_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SellItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_sell_item",
        handler: sell_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ItemPurchaseRefund,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_item_purchase_refund",
        handler: item_purchase_refund_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BankerActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_banker_activate",
        handler: banker_activate_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AutobankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_autobank_item",
        handler: autobank_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AutostoreBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_autostore_bank_item",
        handler: autostore_bank_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BuyBankSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_bank_slot",
        handler: buy_bank_slot_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BinderActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_binder_activate",
        handler: binder_activate_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::HearthAndResurrect,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_hearth_and_resurrect",
        handler: hearth_and_resurrect_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RepairItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_repair_item",
        handler: repair_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusMultipleQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_status_multiple_query",
        handler: quest_giver_status_multiple_query_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusTrackedQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_tracked_query",
        handler: quest_giver_status_tracked_query_thunk::<S, C>,
    })?;

    // ── inventory_actions.rs (6) ────────────────────────────────────────
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SwapInvItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_swap_inv_item",
        handler: swap_inv_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AutoEquipItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_equip_item",
        handler: auto_equip_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AutoEquipItemSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_equip_item_slot",
        handler: auto_equip_item_slot_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SwapItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_swap_item",
        handler: swap_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AutoStoreBagItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_store_bag_item",
        handler: auto_store_bag_item_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DestroyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_destroy_item",
        handler: destroy_item_thunk::<S, C>,
    })?;

    // ── world_queries.rs (2) ────────────────────────────────────────────
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TalkToGossip,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gossip_hello",
        handler: gossip_hello_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::GossipSelectOption,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_gossip_select_option",
        handler: gossip_select_option_thunk::<S, C>,
    })?;

    // ── logout.rs (1) ───────────────────────────────────────────────────
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_request",
        handler: logout_request_thunk::<S, C>,
    })?;
    Ok(())
}
