// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::RemovedAuraLikeCpp;
use wow_entities::RepresentedAuraEffectLikeCpp;
use wow_world_core::session::PlayerAuraRemovalAccessLikeCpp;
use wow_world_spell::SessionSpellState;

pub(super) fn remove_initial_phase_like_cpp(
    spell: &mut SessionSpellState,
    player: &mut PlayerAuraRemovalAccessLikeCpp<'_>,
    spell_store: Option<&wow_data::SpellStore>,
    slot: u8,
) -> Result<RemovedAuraLikeCpp, &'static str> {
    let mounted_aura = player
        .visible_auras_snapshot_like_cpp()
        .and_then(|auras| auras.get(&slot).cloned())
        .is_some_and(|aura| aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted));
    let was_mounted = if mounted_aura {
        player
            .resolved_player_mounted_like_cpp()
            .ok_or("Missing Player presentation owner")?
    } else {
        false
    };
    let Some(aura) = spell.remove_player_visible_aura_with_access_like_cpp(player, slot) else {
        return Err("Aura slot not found");
    };
    if mounted_aura && !player.set_player_mount_presentation_like_cpp(0, false) {
        let _ = spell.insert_player_visible_aura_with_access_like_cpp(player, spell_store, aura);
        return Err("Missing Player presentation owner");
    }
    // C++ AuraEffect::HandleAuraTransform removal clears only the transform
    // owned by the application that is already gone (2129–2131).
    let _ = spell.remove_represented_transform_aura_with_access_like_cpp(player, &aura);
    Ok(RemovedAuraLikeCpp {
        aura,
        mounted_aura,
        was_mounted,
    })
}
