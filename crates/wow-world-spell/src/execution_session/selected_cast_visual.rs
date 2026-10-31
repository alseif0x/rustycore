// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Resolve the visual for the bounded selected self-cast execution path.

use super::SpellExecutionHostLikeCpp;

pub(super) fn resolve<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> Result<u32, &'static str> {
    let spell_id = u32::try_from(spell_id).map_err(|_| "Invalid selected cast spell id")?;
    let core = host.core();
    let hub = host.hub_ref();
    let catalogs = hub.catalogs;
    let owner = core
        .player_handle_like_cpp
        .ok_or("Missing canonical Player owner for selected cast visual")?;
    let manager = core
        .canonical_map_manager
        .as_ref()
        .ok_or("Missing canonical MapManager for selected cast visual")?
        .lock()
        .map_err(|_| "Canonical MapManager is unavailable for selected cast visual")?;
    let (map_key, _) = manager
        .player_active_residence_revision_like_cpp(owner)
        .ok_or("Canonical Player is not admitted on a Map for selected cast visual")?;
    let difficulty_id = manager
        .find_map(map_key.map_id, map_key.instance_id)
        .ok_or("Canonical Player Map is unavailable for selected cast visual")?
        .difficulty();

    crate::aura_session::selected_aura_spell_visual_id(
        &manager,
        owner,
        owner.guid(),
        spell_id,
        difficulty_id,
        catalogs.spell_catalogs.spell_store.as_deref(),
        catalogs.spell_catalogs.serverside_spell_store.as_deref(),
        hub.config
            .creature_aggro_config_like_cpp
            .spell_x_spell_visual_store
            .as_deref(),
        catalogs.condition_presence.as_deref(),
        catalogs.difficulty_store.as_deref(),
    )
}
