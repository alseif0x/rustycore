// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The single ordered packet-handler composition for the process.
//!
//! This crate-level module owns the one ordered list of packet-handler
//! registrars and the legacy inventory registration that closes it. Both the
//! fixture dispatch builder in [`crate::session::registry`] and the
//! process-wide composer published by `world-server` consume this function, so
//! a new handler owner is registered exactly once, in contract order.

use std::sync::Arc;

use wow_handler::DuplicateHandlerRegistrationLikeCpp;

use crate::session::registry::register_remaining_handlers_like_cpp;
use crate::session::registry::{WorldPacketHandlerRegistry, WorldPacketHandlerRegistryBuilder};
use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

/// The single ordered composition of every world-session packet handler.
///
/// This is the one ordered-list authority for the world packet-handler
/// registry: the fixture dispatch table below and the production composer
/// published by `world-server` both consume it, so a new handler owner is
/// registered exactly once, in contract order.
pub fn compose_packet_handlers_like_cpp()
-> Result<Arc<WorldPacketHandlerRegistry>, DuplicateHandlerRegistrationLikeCpp> {
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
    wow_world_lifecycle::register_account_data_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_reputation_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_lifecycle::register_support_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_client_state_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_social::register_calendar_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_social::register_chat_handlers_like_cpp::<WorldSession, SessionHandlerCatalogsLikeCpp>(
        &mut builder,
    )?;
    wow_world_social::register_social_contacts_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_social::register_arena_team_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_lifecycle::register_battlenet_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_data_service_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_social::register_social_group_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_group_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_social::register_guild_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_quest_query_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_combat_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_player_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_collections_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_travel_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_battleground_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_dungeon_finding_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_gameobject_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_vehicle_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_loot_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_character_query_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_trade_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_spell_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_guild_bank_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    wow_world_application::register_character_handlers_like_cpp::<
        WorldSession,
        SessionHandlerCatalogsLikeCpp,
    >(&mut builder)?;
    register_remaining_handlers_like_cpp(&mut builder)?;
    Ok(Arc::new(builder.build()))
}
