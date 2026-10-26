// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Resolve selected execution effects through the canonical difficulty rows.

use super::SpellExecutionHostLikeCpp;
use crate::aura_session::{selected_catalog, selected_spell_metadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectedCastThreatPlan {
    Legacy,
    ExcludedByAttributes,
    Selected {
        whole_spell_positive: bool,
        attributes: [u32; 15],
    },
}

pub(super) fn resolve<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
    spell_info: &mut wow_data::SpellInfo,
) -> Result<SelectedCastThreatPlan, &'static str> {
    let catalogs = host.hub_ref().catalogs;
    let spell_store = catalogs.spell_store().ok_or("Spell not found")?;
    let difficulty = host.core().current_map_difficulty_id_like_cpp();
    let difficulty_store = catalogs.difficulty_store().map(AsRef::as_ref);
    let effects = wow_data::spell::resolve_spell_effect_rows(
        spell_store,
        spell_id,
        difficulty,
        difficulty_store,
    )
    .map_err(|_| "Unable to resolve selected spell effects")?
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();

    if effects.is_empty() {
        return Err("Unable to resolve selected spell effects");
    }

    let attributes = selected_catalog::selected_spell_attributes(
        spell_store,
        spell_id,
        difficulty,
        difficulty_store,
    )
    .map_err(|_| "Unable to resolve selected spell attributes")?;
    let attributes = match attributes {
        Some(attributes) => attributes,
        None => {
            let has_regular_key = match u32::try_from(spell_id) {
                Ok(spell_id) => selected_catalog::selected_spell_info_key_difficulty(
                    spell_store,
                    spell_id,
                    difficulty,
                    difficulty_store,
                )
                .map_err(|_| "Unable to resolve selected spell attributes")?
                .is_some(),
                Err(_) => false,
            };
            if !has_regular_key {
                return Err("Unable to resolve selected spell attributes");
            }
            [0; 15]
        }
    };

    if attributes[1] & wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT != 0
        || attributes[2] & wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT != 0
        || attributes[4] & wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT != 0
    {
        set_selected_effects(spell_info, effects);
        return Ok(SelectedCastThreatPlan::ExcludedByAttributes);
    }

    let metadata = selected_spell_metadata(
        spell_store,
        spell_id,
        difficulty,
        &effects,
        catalogs
            .spell_catalogs
            .spell_class_options_store()
            .map(|store| store.as_ref()),
        difficulty_store,
    )
    .map_err(|_| "Unable to resolve selected cast polarity metadata")?;
    let whole_spell_positive = metadata.polarity.whole_spell_positive;
    let attributes = metadata.attributes;
    set_selected_effects(spell_info, effects);

    Ok(SelectedCastThreatPlan::Selected {
        whole_spell_positive,
        attributes,
    })
}

fn set_selected_effects(
    spell_info: &mut wow_data::SpellInfo,
    effects: Vec<wow_data::SpellEffectInfo>,
) {
    let primary = effects.iter().find(|effect| effect.effect_index == 0);
    spell_info.effect_type = primary.map_or(0, |effect| effect.effect);
    spell_info.effect_base_points = primary.map_or(0, |effect| effect.effect_base_points);
    spell_info.aura_type = primary.and_then(|effect| {
        (effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA)
            .then_some(effect.effect_aura)
    });
    spell_info.effects = effects;
}
