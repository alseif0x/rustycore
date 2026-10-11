// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void-storage packet-handler registrations (#1263 F5, deferred families).
//!
//! C++ source of truth: `src/server/game/Handlers/VoidStorageHandler.cpp:33`
//! `HandleVoidStorageUnlock`, `:52` `HandleVoidStorageQuery`, `:90`
//! `HandleVoidStorageTransfer` and `:206` `HandleVoidSwapItem`;
//! `src/server/game/Server/Protocol/Opcodes.cpp` fixes `CMSG_UNLOCK_VOID_STORAGE`
//! `:990` and `CMSG_QUERY_VOID_STORAGE` `:773` (`STATUS_LOGGEDIN`,
//! `PROCESS_INPLACE`), `CMSG_VOID_STORAGE_TRANSFER` `:1015` (`STATUS_LOGGEDIN`,
//! `PROCESS_THREADUNSAFE`) and `CMSG_SWAP_VOID_ITEM` `:963` (`STATUS_LOGGEDIN`,
//! `PROCESS_INPLACE`).
//!
//! The inventory crate owns this registration family because it already owns
//! the void-storage vocabulary (`void_storage`, `void_transfer`,
//! `void_transfer_contracts`). Only the registration source moved: the four
//! entries keep their opcode, session status, packet processing and handler
//! name exactly as the World legacy inventory submitted them, and every packet
//! is handed to the host unread, as the legacy closures did.

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::WorldPacket;

/// The adapter contract between the void-storage registrations and the World
/// session that executes them.
///
/// **Inventory/money persistence stays with the host.** C++ mutates the Player
/// in place (`ModifyMoney` + `UnlockVoidStorage` at
/// `VoidStorageHandler.cpp:48-49`, `DestroyItem`/`AddVoidStorageItem`/
/// `StoreNewItem` and `ModifyMoney` in `HandleVoidStorageTransfer`) and leaves
/// persistence to the next `Player::SaveToDB` (`_SaveVoidStorage`). The host
/// keeps the Rust equivalent it already owns: NPC admission, the exclusive
/// Player-money persistence fence, the void-storage persistence port and its
/// COMMIT/Unknown-COMMIT outcome, the canonical Player money/flag stage after
/// COMMIT, and every client publication. This crate owns only the registration
/// and dispatch of the four opcodes; it never sees the session, its storage or
/// the persistence port.
pub trait VoidStorageHandlerHostLikeCpp<C> {
    /// C++ `HandleVoidStorageUnlock` (`VoidStorageHandler.cpp:33`). The legacy
    /// closure destructured the catalog view (`id_generators.item`), so the host
    /// receives that view here.
    fn handle_void_storage_unlock_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `HandleVoidStorageQuery` (`VoidStorageHandler.cpp:52`). Read-only
    /// publication; the legacy closure did not read the catalog view.
    fn handle_void_storage_query_like_cpp<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `HandleVoidStorageTransfer` (`VoidStorageHandler.cpp:90`). The legacy
    /// closure passed the whole identifier-generator view (`id_generators`), so
    /// the host receives the catalog view here.
    fn handle_void_storage_transfer_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `HandleVoidSwapItem` (`VoidStorageHandler.cpp:206`). The legacy
    /// closure did not read the catalog view.
    fn handle_void_storage_swap_item_like_cpp<'a>(
        &'a mut self,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;
}

fn handle_void_storage_unlock_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: VoidStorageHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_void_storage_unlock_with_catalogs_like_cpp(catalogs, pkt)
}

fn handle_void_storage_query_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: VoidStorageHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_void_storage_query_like_cpp(pkt)
}

fn handle_void_storage_transfer_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: VoidStorageHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_void_storage_transfer_with_catalogs_like_cpp(catalogs, pkt)
}

fn handle_void_storage_swap_item_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: VoidStorageHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_void_storage_swap_item_like_cpp(pkt)
}

/// Register the four C++ `VoidStorageHandler.cpp` opcodes.
pub fn register_void_storage_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: VoidStorageHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UnlockVoidStorage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_unlock",
        handler: handle_void_storage_unlock_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryVoidStorage,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_query",
        handler: handle_void_storage_query_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::VoidStorageTransfer,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_void_storage_transfer",
        handler: handle_void_storage_transfer_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SwapVoidItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_void_storage_swap_item",
        handler: handle_void_storage_swap_item_thunk::<S, C>,
    })?;
    Ok(())
}
