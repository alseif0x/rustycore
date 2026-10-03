// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry,
    PacketProcessing, RegistryBuilder, SessionStatus,
};
use wow_packet::WorldPacket;

use crate::instances::{
    InstancesHandlerHostLikeCpp, handle_instance_lock_response_like_cpp,
    handle_request_raid_info_like_cpp, handle_reset_instances_like_cpp,
};
use crate::instances::difficulty::{
    handle_set_difficulty_id_like_cpp, handle_set_dungeon_difficulty_like_cpp,
    handle_set_raid_difficulty_like_cpp, handle_toggle_difficulty_like_cpp,
};

fn handle_request_raid_info_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InstancesHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut cx = session.instance_raid_info_handler_cx_like_cpp(catalogs);
        handle_request_raid_info_like_cpp(&mut cx, pkt).await;
    })
}

fn handle_reset_instances_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InstancesHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut cx = session.instance_lock_operations_handler_cx_like_cpp(catalogs);
        handle_reset_instances_like_cpp(&mut cx, pkt).await;
    })
}

fn handle_instance_lock_response_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InstancesHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut cx = session.instance_lock_operations_handler_cx_like_cpp(catalogs);
        let _ = handle_instance_lock_response_like_cpp(&mut cx, pkt).await;
    })
}

fn handle_set_difficulty_id_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InstancesHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut cx = session.instance_difficulty_handler_cx_like_cpp(catalogs);
        handle_set_difficulty_id_like_cpp(&mut cx, pkt).await;
    })
}

fn handle_toggle_difficulty_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InstancesHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut cx = session.instance_difficulty_handler_cx_like_cpp(catalogs);
        handle_toggle_difficulty_like_cpp(&mut cx, pkt).await;
    })
}

fn handle_set_dungeon_difficulty_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InstancesHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut cx = session.instance_difficulty_handler_cx_like_cpp(catalogs);
        handle_set_dungeon_difficulty_like_cpp(&mut cx, pkt).await;
    })
}

fn handle_set_raid_difficulty_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InstancesHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut cx = session.instance_difficulty_handler_cx_like_cpp(catalogs);
        handle_set_raid_difficulty_like_cpp(&mut cx, pkt).await;
    })
}

/// Add the three instance operations and four existing difficulty operations
/// to the shared opcode registry exactly once.
pub fn register_instance_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: InstancesHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestRaidInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_raid_info",
        handler: handle_request_raid_info_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ResetInstances,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_reset_instances",
        handler: handle_reset_instances_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::InstanceLockResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_instance_lock_response",
        handler: handle_instance_lock_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetDifficultyId,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_difficulty_id",
        handler: handle_set_difficulty_id_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ToggleDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_toggle_difficulty",
        handler: handle_toggle_difficulty_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetDungeonDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_dungeon_difficulty",
        handler: handle_set_dungeon_difficulty_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetRaidDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_raid_difficulty",
        handler: handle_set_raid_difficulty_thunk::<S, C>,
    })?;
    Ok(())
}

#[cfg(test)]
#[path = "registration_tests.rs"]
mod tests;
