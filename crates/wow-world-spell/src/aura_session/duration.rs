// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player aura remaining-duration updates.

use wow_core::ObjectGuid;

use super::AuraHostLikeCpp;

/// Set the remaining duration on matching canonical Player aura applications
/// and publish their updated AuraInfo after releasing the Player mutation.
pub fn set_player_aura_duration<H: AuraHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    duration_ms: u32,
) -> usize {
    let updated = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            player.set_aura_duration(spell_id, caster_guid, duration_ms)
        })
        .unwrap_or_default();
    if updated.is_empty() {
        return 0;
    }

    let count = updated.len();
    let Some(player_guid) = host.player_guid() else {
        return count;
    };
    let (player_level, map_id) = {
        let hub = host.hub_ref();
        (
            hub.player_level_like_cpp(),
            hub.core.player_map_id_like_cpp(),
        )
    };
    let auras = updated
        .into_iter()
        .map(|(application, provenance)| {
            let mut info =
                crate::session::player_aura_info_like_cpp(&application, player_level, map_id);
            if let Some(data) = info.aura_data.as_mut() {
                data.remaining_ms =
                    (application.duration_total > 0).then_some(application.duration_remaining);
                data.cast_id = provenance.cast_id;
                data.spell_visual_id = provenance.spell_visual_id;
            }
            info
        })
        .collect();

    host.send_packet(&wow_packet::packets::misc::AuraUpdate {
        unit_guid: player_guid,
        update_all: false,
        auras,
    });
    count
}
