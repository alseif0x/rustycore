// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Resolve selected execution effects through the canonical difficulty rows.

use super::SpellExecutionHostLikeCpp;

pub(super) fn resolve<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
    spell_info: &mut wow_data::SpellInfo,
) -> Result<(), &'static str> {
    let catalogs = host.hub_ref().catalogs;
    let spell_store = catalogs.spell_store().ok_or("Spell not found")?;
    let difficulty = host.core().current_map_difficulty_id_like_cpp();
    let effects = wow_data::spell::resolve_spell_effect_rows(
        spell_store,
        spell_id,
        difficulty,
        catalogs.difficulty_store().map(AsRef::as_ref),
    )
    .map_err(|_| "Unable to resolve selected spell effects")?
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();

    let primary = effects.iter().find(|effect| effect.effect_index == 0);
    spell_info.effect_type = primary.map_or(0, |effect| effect.effect);
    spell_info.effect_base_points = primary.map_or(0, |effect| effect.effect_base_points);
    spell_info.aura_type = primary.and_then(|effect| {
        (effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA)
            .then_some(effect.effect_aura)
    });
    spell_info.effects = effects;
    Ok(())
}
