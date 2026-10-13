// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-giver interaction packet-handler registrations (#1263 F5 remaining families).
//!
//! This is the explicit area registrar that replaces the eleven legacy
//! `inventory::submit!` submissions that lived in
//! `crates/wow-world/src/handlers/quest/handlers.rs`. Only the registration
//! source moved: every entry keeps its opcode, `handler_name`, `SessionStatus`,
//! `PacketProcessing`, packet read and warning text exactly as the shared
//! registry submitted them, and no handler body changed.
//!
//! **Owner.** The application crate owns the quest-giver interaction surface
//! because it already owns the quest-side rules those handlers drive
//! ([`crate::quest`]: dialog status, objective progress, reward plan and
//! publication; [`crate::quest_query_handlers`], which also holds
//! `handle_quest_giver_complete_quest_like_cpp`). C++ source of truth at
//! reference SHA `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` (anchors read in
//! `/home/server/woltk-trinity-legacy`):
//!
//! - `src/server/game/Handlers/QuestHandler.cpp` (9):
//!   `HandleQuestgiverStatusQueryOpcode:41`, `HandleQuestgiverHelloOpcode:76`,
//!   `HandleQuestgiverAcceptQuestOpcode:105`,
//!   `HandleQuestgiverQueryQuestOpcode:228`,
//!   `HandleQuestgiverChooseRewardOpcode:269`,
//!   `HandleQuestgiverRequestRewardOpcode:410`, `HandleQuestConfirmAccept:499`,
//!   `HandleQuestgiverCompleteQuest:533`, `HandlePushQuestToParty:603` — the
//!   quest-giver dialog and reward translation unit.
//! - `src/server/game/Handlers/AdventureMapHandler.cpp:24`
//!   `HandleAdventureMapStartQuest` — starts a quest through the same
//!   `AddQuestAndCheckCompletion` path and the same quest store.
//! - `src/server/game/Handlers/QueryHandler.cpp:280` `HandleQuestPOIQuery` —
//!   answers the quest-POI query against the same quest store.
//!
//! `src/server/game/Server/Protocol/Opcodes.cpp` fixes `STATUS_LOGGEDIN` for
//! all eleven (`:156`, `:774`, `:775`, `:776`, `:778`, `:779`, `:780`, `:781`,
//! `:783`, `:786`, `:746`) and `PROCESS_INPLACE` for nine of them, with
//! `PROCESS_THREADUNSAFE` for `CMSG_ADVENTURE_MAP_START_QUEST` (`:156`),
//! `CMSG_QUEST_CONFIRM_ACCEPT` (`:774`) and `CMSG_PUSH_QUEST_TO_PARTY`
//! (`:746`); the Rust entries reproduce those exactly.
//!
//! The handler bodies stay in their existing owners under
//! `crates/wow-world/src/handlers/quest/**`; the World-side adapter
//! `crates/wow-world/src/handlers/quest/handlers/host.rs` lends this registrar
//! the session operations and the catalog view the legacy closures destructured
//! (`adventure_map_pois`, `quest_info`, `id_generators.item`).

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;
use wow_packet::packets::query::QuestPoiQuery;

/// The narrow host entry points this registration family needs.
///
/// The World session owns the represented quest-giver interaction and its
/// publication; this crate reaches those operations only through these calls.
/// `C` is the session's catalog view, borrowed exactly as the legacy
/// registration closures borrowed it, so no catalog type leaks into this crate.
///
/// `handle_quest_giver_complete_quest_like_cpp` is a supertrait because its body
/// already lives in this crate ([`crate::quest_query_handlers`]); the
/// registration calls that moved body directly instead of duplicating it behind
/// another entry point.
pub trait QuestHandlerHostLikeCpp<C>: crate::QuestGiverCompleteQuestHostLikeCpp {
    /// C++ `AdventureMapHandler.cpp:24` `HandleAdventureMapStartQuest`.
    fn handle_adventure_map_start_quest_with_catalog_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:41` `HandleQuestgiverStatusQueryOpcode`.
    fn handle_quest_giver_status_query_with_catalog_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:76` `HandleQuestgiverHelloOpcode`.
    fn handle_quest_giver_hello<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:228` `HandleQuestgiverQueryQuestOpcode`.
    fn handle_quest_giver_query_quest<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:105` `HandleQuestgiverAcceptQuestOpcode`.
    fn handle_quest_giver_accept_quest_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `QueryHandler.cpp:280` `HandleQuestPOIQuery`.
    fn handle_quest_poi_query<'a>(&'a mut self, query: QuestPoiQuery) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:410` `HandleQuestgiverRequestRewardOpcode`.
    fn handle_quest_giver_request_reward_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:269` `HandleQuestgiverChooseRewardOpcode`.
    fn handle_quest_giver_choose_reward_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:499` `HandleQuestConfirmAccept`.
    fn handle_quest_confirm_accept_with_generator_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `QuestHandler.cpp:603` `HandlePushQuestToParty`.
    fn handle_push_quest_to_party<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()>;
}

fn handle_adventure_map_start_quest_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_adventure_map_start_quest_with_catalog_like_cpp(catalogs, pkt)
            .await
    })
}

fn handle_quest_giver_status_query_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_quest_giver_status_query_with_catalog_like_cpp(catalogs, pkt)
            .await
    })
}

fn handle_quest_giver_hello_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move { session.handle_quest_giver_hello(pkt).await })
}

fn handle_quest_giver_query_quest_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move { session.handle_quest_giver_query_quest(pkt).await })
}

fn handle_quest_giver_accept_quest_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_quest_giver_accept_quest_with_generator_like_cpp(catalogs, pkt)
            .await
    })
}

fn handle_quest_poi_query_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::query::QuestPoiQuery::read(&mut pkt) {
            Ok(query) => session.handle_quest_poi_query(query).await,
            Err(e) => tracing::warn!("Failed to read QuestPoiQuery: {e}"),
        }
    })
}

fn handle_quest_giver_request_reward_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_quest_giver_request_reward_with_generator_like_cpp(catalogs, pkt)
            .await
    })
}

fn handle_quest_giver_complete_quest_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move { crate::handle_quest_giver_complete_quest_like_cpp(session, pkt).await })
}

fn handle_quest_giver_choose_reward_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_quest_giver_choose_reward_with_generator_like_cpp(catalogs, pkt)
            .await
    })
}

fn handle_quest_confirm_accept_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_quest_confirm_accept_with_generator_like_cpp(catalogs, pkt)
            .await
    })
}

fn handle_push_quest_to_party_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move { session.handle_push_quest_to_party(pkt).await })
}

/// Registers the quest-giver interaction handlers on the packet registry.
pub fn register_quest_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: QuestHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AdventureMapStartQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_adventure_map_start_quest",
        handler: handle_adventure_map_start_quest_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_query",
        handler: handle_quest_giver_status_query_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverHello,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_hello",
        handler: handle_quest_giver_hello_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverQueryQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_query_quest",
        handler: handle_quest_giver_query_quest_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverAcceptQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_accept_quest",
        handler: handle_quest_giver_accept_quest_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestPoiQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_poi_query",
        handler: handle_quest_poi_query_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverRequestReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_request_reward",
        handler: handle_quest_giver_request_reward_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCompleteQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_complete_quest",
        handler: handle_quest_giver_complete_quest_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverChooseReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_choose_reward",
        handler: handle_quest_giver_choose_reward_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QuestConfirmAccept,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_confirm_accept",
        handler: handle_quest_confirm_accept_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::PushQuestToParty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_push_quest_to_party",
        handler: handle_push_quest_to_party_thunk::<S, C>,
    })?;
    Ok(())
}
