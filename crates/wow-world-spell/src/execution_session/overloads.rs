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
    let mut spell_visual = SpellCastVisual {
        spell_visual_id: 0,
        script_visual_id: 0,
    };
    let caster_guid = metadata.caster_guid_override.unwrap_or(player_guid);
    if !metadata.from_client
        && metadata.triggered_ignores_global_cooldown_like_cpp
        && target_guid == player_guid
        && caster_guid == player_guid
        && super::resurrection_sickness::is_resurrection_sickness_spell(host, spell_id)
    {
        let (visual_id, cast_flags) =
            super::resurrection_sickness::resurrection_sickness_cast_metadata(host, spell_id)?;
        spell_visual.spell_visual_id = visual_id;
        metadata.cast_flags = cast_flags;
    } else if super::resurrection_auras::is_selected_self_cast(
        host,
        spell_id,
        target_guid,
        caster_guid,
        metadata.from_client,
        metadata.triggered_ignores_global_cooldown_like_cpp,
    ) {
        spell_visual.spell_visual_id = super::resurrection_auras::cast_visual_id(host, spell_id)?;
    }
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
        spell_visual,
        Default::default(),
        metadata,
    )
    .await
}
