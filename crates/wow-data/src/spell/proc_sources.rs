//! Prepare exact regular and server-side spell metadata for proc loading.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};

use super::*;

pub(super) fn load_from_sources<I>(
    rows: I,
    spells: &SpellStore,
    serverside_spells: &ServersideSpellStoreLikeCpp,
    spell_chains: &SpellChainStoreLikeCpp,
    difficulties: &crate::difficulty::DifficultyStore,
    spell_aura_options: &crate::spell_db2::SpellAuraOptionsStore,
    spell_class_options: &crate::spell_db2::SpellClassOptionsStore,
    spell_procs_per_minute: &crate::spell_db2::SpellProcsPerMinuteStore,
) -> Result<SpellProcLoadOutcomeLikeCpp>
where
    I: IntoIterator<Item = SpellProcRowLikeCpp>,
{
    let rows = rows.into_iter().collect::<Vec<_>>();
    let sources = prepare_sources(
        spells,
        serverside_spells,
        spell_chains,
        difficulties,
        spell_aura_options,
        spell_class_options,
        spell_procs_per_minute,
    )?;

    let spell_info_keys = prepare_spell_info_keys(&rows, &sources, difficulties)?;

    let all_sources = sources.values().cloned().collect::<Vec<_>>();
    Ok(SpellProcStoreLikeCpp::from_rows_and_spell_infos_like_cpp(
        rows,
        |spell_id| {
            spell_info_keys
                .get(&spell_id)
                .copied()
                .flatten()
                .and_then(|key| sources.get(&key).cloned())
        },
        all_sources,
    ))
}

fn prepare_spell_info_keys(
    rows: &[SpellProcRowLikeCpp],
    sources: &BTreeMap<(u32, u8), SpellProcSourceSpellInfoLikeCpp>,
    difficulties: &crate::difficulty::DifficultyStore,
) -> Result<BTreeMap<u32, Option<(u32, u8)>>> {
    let mut spell_info_keys = BTreeMap::new();
    let mut inserted_proc_keys = BTreeSet::new();

    for row in rows {
        let root_spell_id = row.spell_id.unsigned_abs();
        let source_key = spell_info_key(root_spell_id, sources, difficulties, &mut spell_info_keys)
            .with_context(|| format!("failed proc source lookup for spell {root_spell_id}"))?;
        let Some(source_key) = source_key else {
            continue;
        };
        let mut source = sources
            .get(&source_key)
            .context("prepared proc source key disappeared during construction")?;

        let all_ranks = row.spell_id < 0;
        if all_ranks && source.first_rank_spell_id != root_spell_id {
            continue;
        }

        loop {
            let proc_key = SpellProcKeyLikeCpp {
                spell_id: source.spell_id,
                difficulty: source.difficulty,
            };
            if !inserted_proc_keys.insert(proc_key) {
                break;
            }
            if !all_ranks {
                break;
            }

            let Some(next_spell_id) = source.next_rank_spell_id else {
                break;
            };
            let next_source_key = spell_info_key(
                next_spell_id,
                sources,
                difficulties,
                &mut spell_info_keys,
            )
            .with_context(|| {
                format!(
                    "failed proc rank source lookup for spell {next_spell_id} requested by all-ranks spell {root_spell_id}"
                )
            })?;
            let Some(next_source_key) = next_source_key else {
                break;
            };
            source = sources
                .get(&next_source_key)
                .context("prepared proc rank source key disappeared during construction")?;
        }
    }

    Ok(spell_info_keys)
}

fn spell_info_key(
    spell_id: u32,
    sources: &BTreeMap<(u32, u8), SpellProcSourceSpellInfoLikeCpp>,
    difficulties: &crate::difficulty::DifficultyStore,
    spell_info_keys: &mut BTreeMap<u32, Option<(u32, u8)>>,
) -> Result<Option<(u32, u8)>> {
    if let Some(key) = spell_info_keys.get(&spell_id) {
        return Ok(*key);
    }

    let key = source_key_at_difficulty_none(spell_id, sources, difficulties)?;
    spell_info_keys.insert(spell_id, key);
    Ok(key)
}

fn prepare_sources(
    spells: &SpellStore,
    serverside_spells: &ServersideSpellStoreLikeCpp,
    spell_chains: &SpellChainStoreLikeCpp,
    difficulties: &crate::difficulty::DifficultyStore,
    spell_aura_options: &crate::spell_db2::SpellAuraOptionsStore,
    spell_class_options: &crate::spell_db2::SpellClassOptionsStore,
    spell_procs_per_minute: &crate::spell_db2::SpellProcsPerMinuteStore,
) -> Result<BTreeMap<(u32, u8), SpellProcSourceSpellInfoLikeCpp>> {
    for key in serverside_spells.spell_infos_by_spell_and_difficulty.keys() {
        let difficulty_id = u8::try_from(key.difficulty_id)
            .context("accepted server spell difficulty does not fit its canonical key")?;
        if spells.contains_spell_info_exact_like_cpp(key.spell_id, difficulty_id) {
            bail!(
                "regular and server-side spell producers overlap at ({}, {})",
                key.spell_id,
                difficulty_id
            );
        }
    }

    let mut sources = BTreeMap::new();
    for (spell_id, difficulty_id) in spells.spell_info_keys_in_order_like_cpp() {
        let source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
            spell_id,
            difficulty_id,
            spells,
            difficulties,
            spell_chains,
            spell_aura_options,
            spell_class_options,
            spell_procs_per_minute,
        )
        .with_context(|| {
            format!(
                "failed to prepare proc source for exact SpellInfo key ({spell_id}, {difficulty_id})"
            )
        })?
        .with_context(|| {
            format!(
                "exact regular SpellInfo key ({spell_id}, {difficulty_id}) produced no proc source"
            )
        })?;
        sources.insert((spell_id, difficulty_id), source);
    }

    for (key, spell_info) in &serverside_spells.spell_infos_by_spell_and_difficulty {
        let difficulty_id = u8::try_from(key.difficulty_id)
            .context("accepted server spell difficulty does not fit its canonical key")?;
        let source = server_source(*key, spell_info, spell_chains).with_context(|| {
            format!(
                "failed to prepare server proc source for ({}, {})",
                key.spell_id, difficulty_id
            )
        })?;
        sources.insert((key.spell_id, difficulty_id), source);
    }

    Ok(sources)
}

fn source_key_at_difficulty_none(
    spell_id: u32,
    sources: &BTreeMap<(u32, u8), SpellProcSourceSpellInfoLikeCpp>,
    difficulties: &crate::difficulty::DifficultyStore,
) -> Result<Option<(u32, u8)>> {
    let has_any_row = sources
        .range((spell_id, u8::MIN)..=(spell_id, u8::MAX))
        .next()
        .is_some();

    resolve_spell_catalog_value(0, Some(difficulties), has_any_row, |difficulty_id| {
        sources
            .contains_key(&(spell_id, difficulty_id))
            .then_some((spell_id, difficulty_id))
    })
    .map_err(|error| {
        anyhow::anyhow!("failed to resolve proc source for spell {spell_id}: {error:?}")
    })
}

fn server_source(
    key: ServersideSpellEffectKeyLikeCpp,
    spell_info: &ServersideSpellInfoLikeCpp,
    spell_chains: &SpellChainStoreLikeCpp,
) -> Result<SpellProcSourceSpellInfoLikeCpp> {
    let difficulty_id = u8::try_from(key.difficulty_id)
        .context("accepted server spell difficulty does not fit its canonical key")?;
    let mut effects = Vec::with_capacity(spell_info.effects.len());
    for effect in &spell_info.effects {
        let effect_index = u32::try_from(effect.effect_index)
            .context("accepted server spell has a negative effect index")?;
        let effect_type = u32::try_from(effect.effect)
            .context("accepted server spell has a negative effect type")?;
        effects.push(SpellEffectInfo {
            effect_index,
            effect: effect_type,
            effect_aura: effect.effect_aura,
            effect_base_points: effect.effect_base_points,
            effect_mechanic: effect.effect_mechanic,
            effect_amplitude: effect.effect_amplitude,
            effect_die_sides: 0,
            effect_bonus_coefficient_from_ap: effect.bonus_coefficient_from_ap,
            effect_spell_class_mask: effect.effect_spell_class_mask.map(|mask| mask as u32),
            effect_misc_value_1: effect.effect_misc_value[0],
            effect_misc_value_2: effect.effect_misc_value[1],
            effect_trigger_spell: effect.effect_trigger_spell,
            effect_radius_index_1: effect.effect_radius_index[0],
            position_facing: effect.effect_pos_facing,
            chain_targets: effect.effect_chain_targets,
            implicit_target_1: effect.implicit_target[0] as u32,
            implicit_target_2: effect.implicit_target[1] as u32,
            polarity_metadata: Some(SpellEffectPolarityMetadata {
                attributes: effect.effect_attributes as u32,
                real_points_per_level: effect.effect_real_points_per_level,
                scaling_coefficient: effect.coefficient,
                scaling_variance: effect.variance,
            }),
        });
    }

    Ok(SpellProcSourceSpellInfoLikeCpp {
        spell_id: key.spell_id,
        difficulty: u32::from(difficulty_id),
        first_rank_spell_id: spell_chains.first_spell_in_chain_like_cpp(key.spell_id),
        next_rank_spell_id: match spell_chains.next_spell_in_chain_like_cpp(key.spell_id) {
            0 => None,
            next => Some(next),
        },
        spell_family_name: spell_info.row.spell_family_name,
        proc_flags: spell_info.row.proc_flags,
        proc_charges: spell_info.row.proc_charges,
        proc_chance: spell_info.row.proc_chance as f32,
        proc_cooldown_ms: spell_info.row.proc_cooldown,
        proc_base_ppm: spell_info.row.proc_base_ppm,
        attributes3: spell_info.row.attributes_ex[2],
        effects,
    })
}
