// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Process-wide composition of world-session packet handlers.

use std::sync::Arc;

use wow_handler::DuplicateHandlerRegistrationLikeCpp;
use wow_world::session::registry::{
    WorldPacketHandlerRegistry, WorldPacketHandlerRegistryBuilder,
    register_remaining_handlers_like_cpp,
};
use wow_world::{WorldSession, session::SessionHandlerCatalogsLikeCpp};

/// Compose the immutable packet-handler registry shared by world sessions.
pub fn compose_packet_handlers_like_cpp() -> Result<
    Arc<WorldPacketHandlerRegistry>,
    DuplicateHandlerRegistrationLikeCpp,
> {
    let mut builder = WorldPacketHandlerRegistryBuilder::new();
    wow_world_inventory::register_inventory_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_instance_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_equipment_set_use_handler_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_bank_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_social::register_social_inspect_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    register_remaining_handlers_like_cpp(&mut builder)?;
    Ok(Arc::new(builder.build()))
}
