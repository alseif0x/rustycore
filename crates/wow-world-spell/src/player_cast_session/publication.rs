// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player cast publication commits recipient visibility at each Start/Go
//! phase, and the player spell DisableMgr gate (#1363 slice 9e), from the
//! World `session/player_cast/publication.rs` and `handlers/spell/ops_2.rs`.
//! Delivery uses the existing bounded durable rail and its overflow disconnect.

use wow_constants::{ServerOpcodes, TypeId};
use wow_data::{DISABLE_TYPE_SPELL, DisableWorldObjectRefLikeCpp};
use wow_world_core::session::mailbox::SendPlayerSpellIfVisibleLikeCppCommand;
use wow_world_core::session::{SessionHubHostLikeCpp, SessionState};
use wow_world_lifecycle::loot_template_rules::{MAP_ARENA_LIKE_CPP, MAP_BATTLEGROUND_LIKE_CPP};

pub fn handle_player_cast_publication_like_cpp<H: SessionHubHostLikeCpp>(
    host: &mut H,
    command: SendPlayerSpellIfVisibleLikeCppCommand,
) {
    let opcode = command
        .packet_bytes
        .get(..2)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_le_bytes);
    if !matches!(opcode, Some(value) if value == ServerOpcodes::SpellStart as u16
        || value == ServerOpcodes::SpellGo as u16 || value == ServerOpcodes::SpellFailure as u16
        || value == ServerOpcodes::SpellFailedOther as u16)
    {
        return;
    }
    if host.state() != SessionState::LoggedIn
        || !host
            .core()
            .client_visible_guids_like_cpp
            .shares_storage_like_cpp(&command.committed_visibility_like_cpp)
        || host.core().current_canonical_player_map_key_like_cpp()
            != Some(wow_map::MapKey::new(
                u32::from(command.map_id),
                command.instance_id,
            ))
    {
        return;
    }
    // Honor publication-time membership; reconnect/map changes are rejected.
    host.send_raw_packet(&command.packet_bytes);
}

pub fn is_spell_disabled_for_player_like_cpp<H: SessionHubHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> bool {
    let Some(disable_mgr) = host.hub_ref().catalogs.disable_mgr() else {
        return false;
    };

    let map_id = u32::from(host.core().player_map_id_like_cpp());
    let Some((_, area_id)) = host.hub_ref().player_zone_area_like_cpp() else {
        return true;
    };
    let map_instance_type = host
        .hub_ref()
        .catalogs
        .map_store()
        .and_then(|store| store.get(map_id))
        .map(|entry| entry.instance_type);

    disable_mgr.is_disabled_for_like_cpp(
        DISABLE_TYPE_SPELL,
        spell_id as u32,
        Some(DisableWorldObjectRefLikeCpp {
            type_id: TypeId::Player,
            map_id,
            area_id,
            is_pet: false,
            is_battle_arena: map_instance_type == Some(MAP_ARENA_LIKE_CPP),
            is_battleground: map_instance_type == Some(MAP_BATTLEGROUND_LIKE_CPP),
            player_map_difficulty: None,
        }),
        0,
        host.hub_ref()
            .catalogs
            .map_store()
            .map(|store| store.as_ref()),
    )
}
