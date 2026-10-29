// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The production overload over the represented spell execution entry point
//! and the server-triggered cast (#1363 slice 9d), from the World
//! `session/spell_effects/{execution_overloads,effects}.rs`.

use super::{
    SpellExecutionHostLikeCpp,
    execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp,
};
use wow_core::ObjectGuid;
use wow_entities::SpellCastMetadata;
use wow_packet::packets::spell::SpellTargetData;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

pub async fn execute_spell_with_visual_and_target_data_and_generator_like_cpp<
    H: SpellExecutionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    spell_id: i32,
    target_guid: ObjectGuid,
    cast_id: ObjectGuid,
    spell_visual: wow_packet::packets::spell::SpellCastVisual,
    target_data: SpellTargetData,
) -> Result<(), &'static str> {
    execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        spell_id,
        target_guid,
        cast_id,
        spell_visual,
        target_data,
        SpellCastMetadata::default(),
    )
    .await
}

/// Server-triggered cast with an explicit C++ trigger contract.
///
/// C++ `Unit::CastSpell` always constructs a `Spell`, so `m_castId` is a
/// real `Map::GenerateLowGuid<HighGuid::Cast>` value even when no client
/// requested the cast. Publishing an empty CastID leaves the client unable
/// to correlate the resulting `SMSG_SPELL_GO`, so this entry point
/// allocates from the admitted Map's shared Cast sequence just like the
/// normal request path and the represented creature consumer.
///
/// `metadata` carries the caller's own contract; normal client defaults are
/// never imposed on these consumers.
pub async fn execute_server_triggered_spell_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    spell_id: i32,
    target_guid: ObjectGuid,
    mut metadata: SpellCastMetadata,
) -> Result<(), &'static str> {
    use wow_packet::packets::spell::SpellCastVisual;
    let player_guid = host.player_guid().ok_or("No player GUID")?;
    let caster_guid = metadata.caster_guid_override.unwrap_or(player_guid);
    let selected_routes = super::execution_support::selected_aura_execution_routes_like_cpp(
        host,
        spell_id,
        player_guid,
        target_guid,
        caster_guid,
        metadata,
    );
    if selected_routes.selected_full_server_aura() {
        metadata.cast_flags =
            super::resurrection_sickness::resurrection_sickness_cast_flags(host, spell_id)?;
        // Spell.cpp:560–563 derives IGNORE_COOLDOWN from the FULL trigger flags.
        metadata.cast_flags_ex |= 0x0000_0200;
    }
    let target_data = if selected_routes.server_triggered_player_ghost_wisp && spell_id == 8326 {
        // Observed R6 Ghost Go: UNIT flag with an empty explicit Unit.
        // observed-ghost-go-contract.json, PKT SHA-256:
        // 116b724cafe4639647d5b2814ba4074a53c2bfc045205d4721b0498556a22f11.
        SpellTargetData {
            flags: 0x0000_0002,
            ..Default::default()
        }
    } else {
        Default::default()
    };
    let Some(cast_id) = host.next_represented_spell_cast_guid_like_cpp(spell_id) else {
        return Err("canonical Map cast identity unavailable for a triggered cast");
    };
    execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        spell_id,
        target_guid,
        cast_id,
        SpellCastVisual {
            spell_visual_id: 0,
            script_visual_id: 0,
        },
        target_data,
        metadata,
    )
    .await
}
