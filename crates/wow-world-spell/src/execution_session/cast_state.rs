// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The cast-state operations the execution reaches over `SessionSpellState`
//! (#1363 slice 9d), from the World `session/player_cast/{wire,publication,power}.rs`.

use crate::{PlayerCastPublicationPhaseLikeCpp, SpellbookHostLikeCpp};
use wow_core::ObjectGuid;
use wow_entities::SpellCastMetadata;
use wow_packet::packets::spell::SpellCastVisual;

pub fn player_cast_publication_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    spell: &wow_data::SpellInfo,
    metadata: &SpellCastMetadata,
    phase: PlayerCastPublicationPhaseLikeCpp,
) -> (wow_packet::packets::spell::SpellCastData, u32) {
    let (state, hub) = host.split_spell_state_ref();
    state.player_cast_publication_like_cpp(hub, spell, metadata, phase)
}

pub fn publish_player_cast_interrupted_frames_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    metadata: SpellCastMetadata,
    cast_id: ObjectGuid,
    spell_id: i32,
    visual: &wow_packet::packets::spell::SpellCastVisual,
) {
    let (state, mut hub) = host.split_spell_state_mut();
    state.publish_player_cast_interrupted_frames_like_cpp(
        &mut hub, metadata, cast_id, spell_id, visual,
    )
}

pub fn publish_player_cast_frame_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    metadata: SpellCastMetadata,
    bytes: Vec<u8>,
) {
    let (state, mut hub) = host.split_spell_state_mut();
    state.publish_player_cast_frame_like_cpp(&mut hub, metadata, bytes)
}

pub fn take_spell_power_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    spell_info: &wow_data::SpellInfo,
    cast_id: ObjectGuid,
    spell_id: i32,
    visual: &SpellCastVisual,
) -> bool {
    let (state, mut hub) = host.split_spell_state_mut();
    state.take_spell_power_like_cpp(&mut hub, spell_info, cast_id, spell_id, visual)
}
