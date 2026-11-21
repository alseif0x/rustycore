// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::quest_session::QuestSessionHostLikeCpp;
use wow_conditions::{
    ConditionAuraEffectSnapshot, ConditionSourceInfo, condition_meets_basic_like_cpp,
    is_object_meeting_not_grouped_conditions_like_cpp,
};
use wow_constants::ConditionSourceType;
use wow_data::{ConditionEntriesByTypeStore, ConditionId};
use wow_entities::PhaseShift;

/// C++ `PhasingHandler::OnConditionChange` over the canonical Player owner.
/// Unsupported conditions fail closed and cannot force a phase transition.
pub fn update_phase_shift_on_condition_change_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
) -> Result<(PhaseShift, bool), &'static str> {
    let hub = host.hub_ref();
    let phase_store = hub
        .catalogs
        .phase_store
        .clone()
        .ok_or("Phase store is unavailable")?;
    let phase_group_store = hub
        .catalogs
        .phase_group_store
        .clone()
        .ok_or("Phase group store is unavailable")?;
    let terrain_swap_store = hub
        .catalogs
        .terrain_swap_store
        .clone()
        .ok_or("Terrain swap store is unavailable")?;
    let condition_store = hub
        .catalogs
        .condition_store
        .clone()
        .ok_or("Phase condition store is unavailable")?;
    let area_store = hub
        .catalogs
        .area_table_store
        .clone()
        .ok_or("Area store is unavailable")?;
    let spell_store = hub
        .catalogs
        .spell_catalogs
        .spell_store
        .clone()
        .ok_or("Spell store is unavailable")?;
    let unit_snapshot = hub
        .condition_player_unit_snapshot_like_cpp()
        .ok_or("Phase Player unit snapshot is unavailable")?;

    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            let aura_effects = player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .values()
                .flat_map(|aura| {
                    spell_store
                        .get(aura.spell_id)
                        .into_iter()
                        .flat_map(|spell| spell.effects())
                        .map(move |effect| ConditionAuraEffectSnapshot {
                            spell_id: aura.spell_id as u32,
                            effect_index: effect.effect_index,
                        })
                })
                .collect::<Vec<_>>();
            let aura_phase_ids = aura_phase_ids_like_cpp(player, &spell_store, false);
            let aura_phase_group_ids = aura_phase_ids_like_cpp(player, &spell_store, true);

            let mut evaluate = |source_type, entry, object: &wow_entities::WorldObject| {
                let mut source_info = ConditionSourceInfo::from_targets(Some(object), None, None);
                source_info.set_unit_target_snapshot(0, unit_snapshot);
                source_info.set_unit_aura_target_snapshot(0, &aura_effects);
                is_object_meeting_not_grouped_conditions_like_cpp(
                    &condition_store,
                    source_type,
                    entry,
                    &mut source_info,
                    |condition, source_info| {
                        condition_meets_basic_like_cpp(
                            condition,
                            source_info,
                            |current, required| area_store.is_in_area_like_cpp(current, required),
                        )
                        .value()
                        .unwrap_or(false)
                    },
                )
            };
            let has_phase_conditions = |phase_id| {
                condition_store
                    .conditions_for_like_cpp(
                        ConditionSourceType::Phase,
                        ConditionId::new(0, phase_id as i32, 0),
                    )
                    .is_some()
            };

            let update = wow_world_core::phasing::on_condition_change_like_cpp(
                player.unit_mut().world_mut(),
                &phase_store,
                &phase_group_store,
                &terrain_swap_store,
                true,
                |phase_id, object| {
                    has_phase_conditions(phase_id)
                        .then(|| evaluate(ConditionSourceType::Phase, phase_id, object))
                },
                |phase_id, object| evaluate(ConditionSourceType::Phase, phase_id, object),
                |map_id, object| evaluate(ConditionSourceType::TerrainSwap, map_id, object),
                aura_phase_ids,
                aura_phase_group_ids,
            );
            (player.unit().world().phase_shift().clone(), update.changed)
        })
        .ok_or("Phase Player owner is unavailable")
}

fn aura_phase_ids_like_cpp(
    player: &wow_entities::Player,
    spell_store: &wow_data::SpellStore,
    phase_group: bool,
) -> Vec<u32> {
    const SPELL_AURA_PHASE_LIKE_CPP: i32 = 261;
    const SPELL_AURA_PHASE_GROUP_LIKE_CPP: i32 = 326;
    let aura_type = if phase_group {
        SPELL_AURA_PHASE_GROUP_LIKE_CPP
    } else {
        SPELL_AURA_PHASE_LIKE_CPP
    };
    wow_world_core::session::player_aura_effects_full_by_spell_aura_type_like_cpp(
        player
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp(),
        spell_store,
        aura_type,
    )
    .into_iter()
    .filter_map(|effect| u32::try_from(effect.misc_value_b).ok())
    .collect()
}
