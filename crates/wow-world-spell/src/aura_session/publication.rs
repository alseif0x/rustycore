// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Aura slot updates and packets published to the client, moved from the
//! World `session/spell_state/aura_publication.rs` (#1363 slice 9b).

use wow_core::ObjectGuid;
use wow_entities::AuraApplicationLikeCpp as AuraApplication;

use super::AuraHostLikeCpp;

pub fn insert_player_visible_aura_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    aura: AuraApplication,
) -> bool {
    let (state, mut hub) = host.split_spell_state_mut();
    state.insert_player_visible_aura_like_cpp(&mut hub, aura)
}

pub fn next_player_visible_aura_slot_like_cpp<H: AuraHostLikeCpp>(host: &H) -> Option<u8> {
    let (state, hub) = host.split_spell_state_ref();
    state.next_player_visible_aura_slot_like_cpp(hub)
}

pub fn send_aura_update_applied<H: AuraHostLikeCpp>(
    host: &H,
    spell_id: i32,
    slot: u8,
    caster: ObjectGuid,
    duration: u32,
    flags: u32,
    effect_mask: u32,
) {
    let (state, hub) = host.split_spell_state_ref();
    state.send_aura_update_applied(hub, spell_id, slot, caster, duration, flags, effect_mask)
}
