use std::collections::{BTreeMap, HashSet};

use wow_core::ObjectGuid;
use wow_entities::Player;
use wow_persistence::{
    PlayerLifecycleAuraEffectSave, PlayerLifecycleAuraFamilySave, PlayerLifecycleAuraSave,
    PlayerLifecycleAuraSpellSave,
};

const SICKNESS_EFFECTS: [(u8, i32); 3] = [
    (0, wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT),
    (
        1,
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
    ),
    (2, wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT),
];
const SPELL_ATTR5_LIMIT_N: u32 = 0x0000_0020;

enum CapturedAura {
    Skipped,
    Save(PlayerLifecycleAuraSave),
}

fn catalog_effects<'a>(
    spell_id: i32,
    difficulty_id: u8,
    spell_store: &'a wow_data::SpellStore,
    difficulty_store: &wow_data::DifficultyStore,
) -> Option<BTreeMap<u8, &'a wow_data::SpellEffectInfo>> {
    if difficulty_id != 0 && !difficulty_store.contains(u32::from(difficulty_id)) {
        return None;
    }
    let effects = wow_data::spell::resolve_spell_effect_rows(
        spell_store,
        spell_id,
        difficulty_id,
        Some(difficulty_store),
    )
    .ok()?;
    if effects.len() != SICKNESS_EFFECTS.len() {
        return None;
    }

    let mut catalog = BTreeMap::new();
    for effect in effects {
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            || !matches!(effect.implicit_target_1, 0 | 1)
            || effect.implicit_target_2 != 0
        {
            return None;
        }
        let index = u8::try_from(effect.effect_index).ok()?;
        if catalog.insert(index, effect).is_some() {
            return None;
        }
    }
    for (index, aura_type) in SICKNESS_EFFECTS {
        if catalog.get(&index)?.effect_aura != aura_type {
            return None;
        }
    }
    Some(catalog)
}

fn capture_aura(
    player: &Player,
    slot: u8,
    application: &wow_entities::AuraApplicationLikeCpp,
    spell_effects: &BTreeMap<u8, &wow_data::SpellEffectInfo>,
    attributes: [u32; 15],
) -> Option<CapturedAura> {
    if application.stack_count == 0 || application.effect_mask == 0 {
        return None;
    }
    let supported_mask = SICKNESS_EFFECTS
        .iter()
        .fold(0_u32, |mask, (index, _)| mask | (1_u32 << index));
    if application.effect_mask & !supported_mask != 0 {
        return None;
    }

    let auras = &player.unit().subsystems().auras;
    let metadata = auras.aura_persistence_metadata(slot);
    let mut stored_base_amounts = BTreeMap::new();
    if let Some(metadata) = metadata {
        if metadata.recalculate_mask & !application.effect_mask != 0 {
            return None;
        }
        for &(index, amount) in &metadata.base_amounts {
            let bit = 1_u32.checked_shl(u32::from(index))?;
            if application.effect_mask & bit == 0
                || stored_base_amounts.insert(index, amount).is_some()
            {
                return None;
            }
        }
        if stored_base_amounts.len() != application.effect_mask.count_ones() as usize {
            return None;
        }
    }

    let snapshot = auras.threat_snapshot_like_cpp(slot)?;
    if snapshot.effects().len() != application.effect_mask.count_ones() as usize {
        return None;
    }
    let mut effects = Vec::with_capacity(snapshot.effects().len());
    let mut effect_mask = 0_u32;
    let mut seen_indices = HashSet::new();
    let mut recalculate_mask = metadata.map_or(0, |metadata| metadata.recalculate_mask);
    for &(bit, aura_type, amount, _) in snapshot.effects() {
        if bit.count_ones() != 1 || application.effect_mask & bit == 0 || effect_mask & bit != 0 {
            return None;
        }
        let index = u8::try_from(bit.trailing_zeros()).ok()?;
        if !seen_indices.insert(index) {
            return None;
        }
        let effect_info = spell_effects.get(&index)?;
        if effect_info.effect_aura != aura_type
            || !SICKNESS_EFFECTS
                .iter()
                .any(|(known_index, known_type)| *known_index == index && *known_type == aura_type)
        {
            return None;
        }
        let base_amount = match metadata {
            Some(_) => *stored_base_amounts.get(&index)?,
            None => {
                recalculate_mask |= bit;
                effect_info.effect_base_points
            }
        };
        effects.push(PlayerLifecycleAuraEffectSave {
            index,
            amount,
            base_amount,
        });
        effect_mask |= bit;
    }
    if effect_mask != application.effect_mask {
        return None;
    }

    // The source CanBeSaved path skips known foreign single-target auras and
    // permanent item-triggered applications. Other CanBeSaved predicates
    // (custom non-saveable attributes, AuraOptions-backed charge usage, and
    // runtime single-target state) are not fully represented by these
    // catalogs, so this capture does not claim general CanBeSaved parity.
    if application.caster_guid != player.guid() && attributes[5] & SPELL_ATTR5_LIMIT_N != 0 {
        return Some(CapturedAura::Skipped);
    }
    if application.duration_total == -1
        && metadata.is_some_and(|metadata| metadata.item_guid != ObjectGuid::EMPTY)
    {
        return Some(CapturedAura::Skipped);
    }

    let max_duration = application.duration_total;
    let remaining_duration = application.duration_remaining;
    effects.sort_by_key(|effect| effect.index);
    Some(CapturedAura::Save(PlayerLifecycleAuraSave {
        difficulty: application.difficulty_id,
        stack_count: application.stack_count,
        max_duration,
        remaining_duration,
        charges: metadata.map_or(0, |metadata| metadata.charges),
        recalculate_mask,
        cast_item_id: metadata.map_or(0, |metadata| metadata.cast_item_id),
        cast_item_level: metadata.map_or(-1, |metadata| metadata.cast_item_level),
        effects,
    }))
}

/// Capture the selected race's complete persistable sickness spell keyset.
/// C++ anchors: Player.cpp Player::_LoadAuras (18034), Player::_SaveAuras
/// (19811-19865), and SpellAuras.cpp Aura::CanBeSaved (1134), Aura::GenerateKey
/// (1218). The snapshot retains the canonical signed maximum and remaining
/// duration; Unit removal runs before the periodic Player save phase.
pub(super) fn capture(
    player: &Player,
    chr_races_store: Option<&wow_data::character_progression::ChrRacesStore>,
    spell_store: Option<&wow_data::SpellStore>,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Option<PlayerLifecycleAuraSpellSave> {
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
    if !spell_store.contains_spell_info_any_difficulty_like_cpp(spell_id) {
        return None;
    }
    let selected_spell_id = i32::try_from(spell_id).ok()?;

    let mut families =
        BTreeMap::<([u8; 16], [u8; 16]), (Vec<PlayerLifecycleAuraSave>, HashSet<u32>)>::new();
    for (&slot, application) in auras.runtime_applications_like_cpp() {
        if application.spell_id != selected_spell_id {
            continue;
        }
        let actual_difficulty_id = wow_data::spell::resolve_spell_info_key_difficulty(
            spell_store,
            spell_id,
            application.difficulty_id,
            Some(difficulty_store),
        )
        .ok()??;
        let attributes = wow_data::spell::resolve_spell_attributes(
            spell_store,
            selected_spell_id,
            actual_difficulty_id,
            Some(difficulty_store),
        )
        .ok()??;
        if attributes[0] & wow_data::spell::attributes::SPELL_ATTR0_PASSIVE != 0
            || attributes[1]
                & (wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED
                    | wow_data::spell::attributes::SPELL_ATTR1_IS_SELF_CHANNELLED)
                != 0
        {
            continue;
        }
        let spell_effects = catalog_effects(
            selected_spell_id,
            actual_difficulty_id,
            spell_store,
            difficulty_store,
        )?;
        let aura = match capture_aura(player, slot, application, &spell_effects, attributes)? {
            CapturedAura::Skipped => continue,
            CapturedAura::Save(aura) => aura,
        };
        let metadata = auras.aura_persistence_metadata(slot);
        let caster = application.caster_guid.to_raw_bytes();
        let item = metadata.map_or(ObjectGuid::EMPTY.to_raw_bytes(), |metadata| {
            metadata.item_guid.to_raw_bytes()
        });
        let (saved_auras, effect_masks) = families.entry((caster, item)).or_default();
        if !effect_masks.insert(application.effect_mask) {
            return None;
        }
        saved_auras.push(aura);
    }

    let families = families
        .into_iter()
        .map(|((caster_guid_binary, item_guid_binary), (mut auras, _))| {
            auras.sort_by_key(|aura| {
                let effect_mask = aura
                    .effects
                    .iter()
                    .fold(0_u32, |mask, effect| mask | (1_u32 << effect.index));
                (aura.difficulty, effect_mask)
            });
            PlayerLifecycleAuraFamilySave {
                spell_id,
                caster_guid_binary,
                item_guid_binary,
                auras,
            }
        })
        .collect();
    Some(PlayerLifecycleAuraSpellSave { spell_id, families })
}
