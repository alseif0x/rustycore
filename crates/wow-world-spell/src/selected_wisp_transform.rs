// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Apply the selected Wisp transform using spell-wide metadata at its cast difficulty.

use wow_data::{DifficultyStore, SpellClassOptionsStore, SpellEffectInfo, SpellStore};
use wow_entities::AuraApplicationLikeCpp;
use wow_world_core::session::PlayerAuraRemovalAccessLikeCpp as PlayerAuraRemovalAccess;

const WISP_SPELL_ID: i32 = 20_584;

/// Apply one selected Wisp transform after resolving both spell-wide polarity facts.
///
/// C++ `SpellAuraEffects.cpp:1944-1951` resolves the current transform's
/// `SpellInfo` at the incoming Aura base's cast difficulty.
pub fn apply_selected_wisp_transform(
    player: &mut PlayerAuraRemovalAccess<'_>,
    spells: &SpellStore,
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
    aura: &AuraApplicationLikeCpp,
) -> Result<(), &'static str> {
    if aura.spell_id != WISP_SPELL_ID {
        return Err("Selected Wisp transform requires spell 20584");
    }

    let wisp_key = wow_data::spell::resolve_spell_info_key_difficulty(
        spells,
        WISP_SPELL_ID as u32,
        aura.difficulty_id,
        difficulties,
    )
    .map_err(|_| "Unable to resolve the selected Wisp SpellInfo key")?
    .ok_or("Missing selected Wisp SpellInfo key")?;
    let wisp_effects =
        wow_data::spell::resolve_spell_effect_rows(spells, WISP_SPELL_ID, wisp_key, difficulties)
            .map_err(|_| "Unable to resolve selected Wisp effect rows")?;
    if !wisp_effects.iter().any(|effect| {
        1_u32
            .checked_shl(effect.effect_index)
            .is_some_and(|bit| aura.effect_mask & bit != 0)
            && effect.is_aura_like_cpp()
            && effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_TRANSFORM
    }) {
        return Err("Selected Wisp Aura has no active transform effect");
    }
    let wisp_effect_values = wisp_effects
        .iter()
        .map(|effect| (*effect).clone())
        .collect::<Vec<SpellEffectInfo>>();
    let wisp_metadata = crate::aura_session::selected_spell_metadata(
        spells,
        WISP_SPELL_ID,
        wisp_key,
        &wisp_effect_values,
        class_options,
        difficulties,
    )
    .map_err(|_| "Unable to classify selected Wisp polarity")?;

    let current_transform = player
        .aura_subsystem_snapshot_like_cpp()
        .ok_or("Missing canonical Player aura subsystem for Wisp transform")?
        .transform_spell_like_cpp();
    let current_is_positive = if current_transform == 0 {
        None
    } else {
        let current_key = wow_data::spell::resolve_spell_info_key_difficulty(
            spells,
            current_transform as u32,
            aura.difficulty_id,
            difficulties,
        )
        .map_err(|_| "Unable to resolve current transform SpellInfo key")?;
        if let Some(current_key) = current_key {
            let current_effects = wow_data::spell::resolve_spell_effect_rows(
                spells,
                current_transform,
                current_key,
                difficulties,
            )
            .map_err(|_| "Unable to resolve current transform effect rows")?;
            let current_effect_values = current_effects
                .iter()
                .map(|effect| (*effect).clone())
                .collect::<Vec<SpellEffectInfo>>();
            Some(
                crate::aura_session::selected_spell_metadata(
                    spells,
                    current_transform,
                    current_key,
                    &current_effect_values,
                    class_options,
                    difficulties,
                )
                .map_err(|_| "Unable to classify current transform polarity")?
                .polarity
                .whole_spell_positive,
            )
        } else {
            None
        }
    };

    if !player.apply_transform_canonical_like_cpp(
        WISP_SPELL_ID,
        wisp_metadata.polarity.whole_spell_positive,
        current_is_positive,
    ) {
        return Err("Canonical Player disappeared while applying Wisp transform");
    }
    Ok(())
}
