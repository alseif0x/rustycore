use std::collections::{BTreeMap, HashSet};

use wow_core::ObjectGuid;
use wow_entities::Player;
use wow_persistence::{
    PlayerResurrectionSicknessAuraSave, PlayerResurrectionSicknessEffectSave,
    PlayerResurrectionSicknessSave,
};

/// Capture the selected race's persistable sickness aura from canonical Unit state.
/// C++ anchors: Player.cpp `Player::_LoadAuras` (18034), `Player::_SaveAuras`
/// (19811-19865), and SpellAuras.cpp `Aura::CanBeSaved` (1134), `Aura::GenerateKey`
/// (1218).
pub(super) fn capture(
    player: &Player,
    chr_races_store: Option<&wow_data::character_progression::ChrRacesStore>,
    spell_store: Option<&wow_data::SpellStore>,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Option<PlayerResurrectionSicknessSave> {
    let chr_races_store = chr_races_store?;
    let spell_store = spell_store?;
    let difficulty_store = difficulty_store?;
    let auras = &player.unit().subsystems().auras;
    if !auras.persisted_player_aura_authority_complete_like_cpp() {
        return None;
    }
    let race = chr_races_store.get(u32::from(player.race_like_cpp()))?;
    let spell_id = u32::try_from(race.res_sickness_spell_id).ok()?;
    if spell_id == 0 {
        return None;
    }
    let selected_spell_id = i32::try_from(spell_id).ok()?;
    spell_store.get(selected_spell_id)?;

    let player_guid = player.guid();
    let mut caster_guid_binary = player_guid.to_raw_bytes();
    let mut saved_auras = Vec::new();
    let mut effect_masks = HashSet::new();

    for (&slot, application) in auras.runtime_applications_like_cpp() {
        if u32::try_from(application.spell_id).ok() != Some(spell_id)
            || application.caster_guid != player_guid
        {
            continue;
        }
        caster_guid_binary = application.caster_guid.to_raw_bytes();

        let metadata = auras.aura_persistence_metadata(slot);
        if metadata.is_some_and(|metadata| metadata.item_guid != ObjectGuid::EMPTY) {
            continue;
        }

        if spell_store.is_passive_like_cpp(application.spell_id)
            || spell_store.is_channeled_like_cpp(application.spell_id)
        {
            continue;
        }
        if application.duration_total == 0 {
            return None;
        }

        let elapsed_ms =
            u32::try_from(application.applied_at.elapsed().as_millis()).unwrap_or(u32::MAX);
        let remaining_ms = application.duration_remaining.saturating_sub(elapsed_ms);
        if application.duration_total > 0 && remaining_ms == 0 {
            continue;
        }
        let max_duration = i32::try_from(application.duration_total).ok()?;
        let remaining_duration = i32::try_from(remaining_ms).ok()?;
        if application.effect_mask == 0 {
            return None;
        }
        if metadata
            .is_some_and(|metadata| metadata.recalculate_mask & !application.effect_mask != 0)
        {
            return None;
        }

        let snapshot_effects = auras
            .threat_snapshot_like_cpp(slot)
            .map(|snapshot| snapshot.effects());
        if application.effect_mask != 0 && snapshot_effects.is_none() {
            return None;
        }

        let mut effects = Vec::new();
        let mut effect_mask = 0_u32;
        let mut indices = HashSet::new();
        let mut recalculate_mask = metadata.map_or(0, |metadata| metadata.recalculate_mask);
        let mut stored_base_amounts = BTreeMap::new();
        if let Some(metadata) = metadata {
            for &(index, amount) in &metadata.base_amounts {
                if stored_base_amounts.insert(index, amount).is_some() {
                    return None;
                }
            }
        }

        for &(bit, aura_type, amount, _) in snapshot_effects.into_iter().flatten() {
            if bit.count_ones() != 1 || application.effect_mask & bit == 0 {
                return None;
            }
            let index = u8::try_from(bit.trailing_zeros()).ok()?;
            if !indices.insert(index) {
                return None;
            }

            let base_amount = if metadata.is_some() {
                *stored_base_amounts.get(&index)?
            } else {
                let spell_effects = spell_store.effects_for_difficulty_like_cpp(
                    application.spell_id,
                    application.difficulty_id,
                    Some(difficulty_store),
                )?;
                spell_effects
                    .iter()
                    .find(|effect| effect.effect_index == u32::from(index))?
                    .effect_base_points
            };

            if metadata.is_none() {
                if !matches!(aura_type, 79 | 80 | 159) {
                    return None;
                }
                recalculate_mask |= bit;
            }
            effects.push(PlayerResurrectionSicknessEffectSave {
                index,
                amount,
                base_amount,
            });
            effect_mask |= bit;
        }

        if effect_mask != application.effect_mask || !effect_masks.insert(effect_mask) {
            return None;
        }
        effects.sort_by_key(|effect| effect.index);

        saved_auras.push(PlayerResurrectionSicknessAuraSave {
            difficulty: application.difficulty_id,
            stack_count: application.stack_count,
            max_duration,
            remaining_duration,
            charges: metadata.map_or(0, |metadata| metadata.charges),
            recalculate_mask,
            cast_item_id: metadata.map_or(0, |metadata| metadata.cast_item_id),
            cast_item_level: metadata.map_or(-1, |metadata| metadata.cast_item_level),
            effects,
        });
    }

    saved_auras.sort_by_key(|aura| {
        let effect_mask = aura
            .effects
            .iter()
            .fold(0_u32, |mask, effect| mask | (1_u32 << effect.index));
        (aura.difficulty, effect_mask)
    });

    Some(PlayerResurrectionSicknessSave {
        spell_id,
        caster_guid_binary,
        auras: saved_auras,
    })
}
