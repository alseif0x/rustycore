use std::collections::{BTreeMap, HashSet};

use wow_core::ObjectGuid;
use wow_entities::Player;
use wow_persistence::{
    PlayerLifecycleAuraEffectSave, PlayerLifecycleAuraFamilySave, PlayerLifecycleAuraSave,
};

const GHOST_SPELL_ID: i32 = 8326;
const WISP_SPELL_ID: i32 = 20584;
// C++ SharedDefines.h `SPELL_ATTR5_LIMIT_N`; SpellInfo::IsSingleTarget
// (`SpellInfo.cpp:1864`).
const SPELL_ATTR5_LIMIT_N: u32 = 0x0000_0020;

const GHOST_EFFECTS: [(u8, i32, bool); 3] = [
    (0, wow_data::spell::aura_types::SPELL_AURA_GHOST, true),
    (
        1,
        wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        true,
    ),
    (
        2,
        wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        true,
    ),
];

const WISP_EFFECTS: [(u8, i32, bool); 3] = [
    (
        0,
        wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        true,
    ),
    (
        1,
        wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        true,
    ),
    (2, wow_data::spell::aura_types::SPELL_AURA_TRANSFORM, false),
];

enum CapturedAura {
    Skipped,
    Save(PlayerLifecycleAuraSave),
}

fn expected_effects(spell_id: i32) -> Option<&'static [(u8, i32, bool)]> {
    match spell_id {
        GHOST_SPELL_ID => Some(&GHOST_EFFECTS),
        WISP_SPELL_ID => Some(&WISP_EFFECTS),
        _ => None,
    }
}

fn catalog_effects<'a>(
    spell_id: i32,
    actual_difficulty_id: u8,
    spell_store: &'a wow_data::SpellStore,
    difficulty_store: &wow_data::DifficultyStore,
) -> Option<BTreeMap<u8, &'a wow_data::SpellEffectInfo>> {
    let expected = expected_effects(spell_id)?;
    let effects = wow_data::spell::resolve_spell_effect_rows(
        spell_store,
        spell_id,
        actual_difficulty_id,
        Some(difficulty_store),
    )
    .ok()?;
    if effects.len() != expected.len() {
        return None;
    }
    let mut aura_effects = BTreeMap::new();
    for effect in effects {
        // The selected records are exactly three APPLY_AURA effects. C++
        // Aura::CanBeSaved rejects area-targeting/area-aura effects for a
        // foreign caster; the inspected selected data uses only no target
        // (0), self (1), and no second target (0). Unknown target ids fail
        // closed instead of being treated as harmless.
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            || !matches!(effect.implicit_target_1, 0 | 1)
            || effect.implicit_target_2 != 0
        {
            return None;
        }
        let index = u8::try_from(effect.effect_index).ok()?;
        if aura_effects.insert(index, effect).is_some() {
            return None;
        }
    }
    if aura_effects.len() != expected.len() {
        return None;
    }
    for (index, aura_type, _) in expected {
        let effect = aura_effects.get(index)?;
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            || effect.effect_aura != *aura_type
        {
            return None;
        }
    }
    Some(aura_effects)
}

fn capture_aura(
    player: &Player,
    slot: u8,
    application: &wow_entities::AuraApplicationLikeCpp,
    spell_effects: &BTreeMap<u8, &wow_data::SpellEffectInfo>,
    attributes: [u32; 15],
) -> Option<CapturedAura> {
    let spell_id = application.spell_id;
    let expected = expected_effects(spell_id)?;

    let metadata = player
        .unit()
        .subsystems()
        .auras
        .aura_persistence_metadata(slot);
    if application.caster_guid != player.guid() && attributes[5] & SPELL_ATTR5_LIMIT_N != 0 {
        return Some(CapturedAura::Skipped);
    }
    // C++ Aura::CanBeSaved excludes permanent item-triggered auras. A
    // complete selected-spell rewrite removes their former SQL row.
    if application.duration_total == -1
        && metadata.is_some_and(|metadata| metadata.item_guid != ObjectGuid::EMPTY)
    {
        return Some(CapturedAura::Skipped);
    }
    if application.stack_count == 0 || application.effect_mask == 0 {
        return None;
    }
    let expected_mask = expected
        .iter()
        .fold(0_u32, |mask, (index, _, _)| mask | (1_u32 << index));
    if application.effect_mask & !expected_mask != 0 {
        return None;
    }
    let max_duration = application.duration_total;
    let remaining_duration = application.duration_remaining;

    let mut stored_base_amounts = BTreeMap::new();
    if let Some(metadata) = metadata {
        let recalculable_mask = expected
            .iter()
            .fold(0_u32, |mask, (index, _, recalculable)| {
                if *recalculable {
                    mask | (1_u32 << index)
                } else {
                    mask
                }
            });
        if metadata.recalculate_mask & !application.effect_mask != 0
            || metadata.recalculate_mask & !recalculable_mask != 0
        {
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

    let snapshot = player
        .unit()
        .subsystems()
        .auras
        .threat_snapshot_like_cpp(slot)?;
    if snapshot.effects().len() != application.effect_mask.count_ones() as usize {
        return None;
    }
    let mut effects = Vec::with_capacity(snapshot.effects().len());
    let mut effect_mask = 0_u32;
    let mut recalculate_mask = metadata.map_or(0, |metadata| metadata.recalculate_mask);
    let mut indices = HashSet::new();
    for &(bit, aura_type, amount, _) in snapshot.effects() {
        if bit.count_ones() != 1 || application.effect_mask & bit == 0 || effect_mask & bit != 0 {
            return None;
        }
        let index = u8::try_from(bit.trailing_zeros()).ok()?;
        if !indices.insert(index) {
            return None;
        }
        let (_, expected_aura_type, recalculable) = expected
            .iter()
            .find(|(expected_index, _, _)| *expected_index == index)?;
        let effect_info = spell_effects.get(&index)?;
        if aura_type != *expected_aura_type || effect_info.effect_aura != aura_type {
            return None;
        }
        let base_amount = if let Some(metadata) = metadata {
            *stored_base_amounts.get(&index)?
        } else {
            if *recalculable {
                recalculate_mask |= bit;
            }
            effect_info.effect_base_points
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

pub(super) fn capture(
    player: &Player,
    spell_store: Option<&wow_data::SpellStore>,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Option<Vec<PlayerLifecycleAuraFamilySave>> {
    let spell_store = spell_store?;
    let difficulty_store = difficulty_store?;
    let auras = &player.unit().subsystems().auras;
    if !auras.persisted_player_aura_authority_complete_like_cpp() {
        return None;
    }
    for spell_id in [GHOST_SPELL_ID, WISP_SPELL_ID] {
        let Ok(spell_id_u32) = u32::try_from(spell_id) else {
            return None;
        };
        if !spell_store.contains_spell_info_any_difficulty_like_cpp(spell_id_u32) {
            return None;
        }
    }

    let player_guid = player.guid();
    let player_guid_binary = player_guid.to_raw_bytes();
    let empty_item_guid_binary = ObjectGuid::EMPTY.to_raw_bytes();
    let mut families =
        BTreeMap::<(i32, [u8; 16], [u8; 16]), (Vec<PlayerLifecycleAuraSave>, HashSet<u32>)>::new();
    for spell_id in [GHOST_SPELL_ID, WISP_SPELL_ID] {
        families.insert(
            (spell_id, player_guid_binary, empty_item_guid_binary),
            (Vec::new(), HashSet::new()),
        );
    }
    for (&slot, application) in auras.runtime_applications_like_cpp() {
        if !matches!(application.spell_id, GHOST_SPELL_ID | WISP_SPELL_ID) {
            continue;
        }
        let spell_id_u32 = u32::try_from(application.spell_id).ok()?;
        if application.difficulty_id != 0
            && !difficulty_store.contains(u32::from(application.difficulty_id))
        {
            return None;
        }
        // C++ `_SaveAuras` reads the active Aura's original SpellInfo. Resolve
        // its raw difficulty once and use the actual key for all component rows.
        let actual_difficulty_id = wow_data::spell::resolve_spell_info_key_difficulty(
            spell_store,
            spell_id_u32,
            application.difficulty_id,
            Some(difficulty_store),
        )
        .ok()??;
        let attributes = wow_data::spell::resolve_spell_attributes(
            spell_store,
            application.spell_id,
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
            application.spell_id,
            actual_difficulty_id,
            spell_store,
            difficulty_store,
        )?;
        let aura_metadata = auras.aura_persistence_metadata(slot);
        let item_guid_binary = aura_metadata.map_or(empty_item_guid_binary, |metadata| {
            metadata.item_guid.to_raw_bytes()
        });
        let aura = match capture_aura(player, slot, application, &spell_effects, attributes)? {
            CapturedAura::Skipped => continue,
            CapturedAura::Save(aura) => aura,
        };
        let key = (
            application.spell_id,
            application.caster_guid.to_raw_bytes(),
            item_guid_binary,
        );
        let (family_auras, effect_masks) = families.entry(key).or_default();
        if !effect_masks.insert(application.effect_mask) {
            return None;
        }
        family_auras.push(aura);
    }
    let families = families
        .into_iter()
        .map(
            |((spell_id, caster_guid_binary, item_guid_binary), (mut auras, _))| {
                auras.sort_by_key(|aura| {
                    let effect_mask = aura
                        .effects
                        .iter()
                        .fold(0_u32, |mask, effect| mask | (1_u32 << effect.index));
                    (aura.difficulty, effect_mask)
                });
                Some(PlayerLifecycleAuraFamilySave {
                    spell_id: u32::try_from(spell_id).ok()?,
                    caster_guid_binary,
                    item_guid_binary,
                    auras,
                })
            },
        )
        .collect::<Option<Vec<_>>>()?;
    Some(families)
}
