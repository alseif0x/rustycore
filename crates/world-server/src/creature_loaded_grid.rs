// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure loaded-grid Creature lifecycle resolver for the real map insertion path.
//!
//! C++ anchors:
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:1770-1813`
//!   `Creature::CreateFromProto`: template lookup/original entry, creature/vehicle high GUID,
//!   `UpdateEntry`, optional vehicle kit.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:1815-1923`
//!   `Creature::LoadFromDB`: caller/Map ownership handles duplicate/alive guard; resolved
//!   `CreatureData` drives spawn id, respawn compatibility, creature data, wander/respawn,
//!   `Create`, home position, inactive group gates, `SetSpawnHealth`, movement/string id,
//!   optional `AddToMap`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:333-350`
//!   `Creature::AddToWorld`: map object store/spawn-id multimap plus formation/AI/vehicle/script hooks;
//!   this resolver only produces the typed record for that owner.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Grids/ObjectGridLoader.cpp:44-78`
//!   loaded grid helper creates an object and calls `LoadFromDB`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp:519-542`
//!   `Map::AddToMap`: Map creates/binds/adds object and runs object-level `AddToWorld`.
//!
//! Ownership: DB/template/spawn caches are resolved by the caller before taking a `MapManager`/`Map`
//! lock. This module performs no async work, no DB lookups, no live-map mutation, and no fanout.
//! Sync direction is DB/template/spawn-store -> lifecycle record -> `Creature` -> `MapObjectRecord`.

use std::collections::BTreeMap;

use crate::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp;
use anyhow::Result;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::{
    CreatureAddonStoreLikeCpp, CreatureBaseStatsStoreLikeCpp,
    CreatureClassificationHealthRatesLikeCpp, CreatureDifficultyStoreLikeCpp,
    CreatureDisplayInfoStore, CreatureEquipmentStoreLikeCpp, CreatureModelDataStore,
    CreatureModelInfoStoreLikeCpp, CreatureModelSelectionRandomLikeCpp,
    CreatureTemplateLifecycleModelLikeCpp, CreatureTemplateLifecycleStoreLikeCpp,
    character_progression::{ChrClassesStore, PowerTypeStore},
};
use wow_entities::{
    Creature, CreatureAddToWorldVehicleResetContextLikeCpp, CreatureAddonLifecycleRecordLikeCpp,
    CreatureCombatLogStatsLikeCpp, CreatureCreateLifecycleRecord, CreatureFormationInfoLikeCpp,
    CreatureLifecycleStats, CreatureLoadFromDbLifecycleRecord, CreatureModelDimensions,
    CreatureSpawnLifecycleRecord, CreatureTemplateLifecycleRecord, DEFAULT_CORPSE_DELAY_SECS,
    MapObjectRecord, MovementGeneratorType, VehicleKitCreateInputLikeCpp,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCreatureTemplateLikeCpp {
    pub entry: u32,
    pub original_entry: u32,
    pub difficulty_id: u8,
    pub name: String,
    pub ai_name: String,
    pub script_name: String,
    pub required_expansion: u8,
    pub unit_class: u8,
    pub trainer_class: u8,
    pub faction: u32,
    pub npc_flags: u64,
    pub display_id: u32,
    pub model_dimensions: Option<CreatureModelDimensions>,
    pub scale: f32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub spells: [u32; 8],
    pub classification: u32,
    pub damage_school: u8,
    pub sparring_health_pct: Option<f32>,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
    pub flags_extra: u32,
    pub static_flags: [u32; 8],
    pub creature_type: u32,
    pub type_flags: u32,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub movement_type: MovementGeneratorType,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub equipment_id: u8,
    pub original_equipment_id: i8,
    pub vehicle_id: Option<u32>,
    pub vehicle_kit_create_input: Option<VehicleKitCreateInputLikeCpp>,
    pub add_to_world_vehicle_reset_context: Option<CreatureAddToWorldVehicleResetContextLikeCpp>,
    pub corpse_delay: u32,
    pub ignore_corpse_decay_ratio: bool,
    pub addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCreatureSpawnLikeCpp {
    pub spawn_id: u64,
    pub entry: u32,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub home_position: Position,
    pub phase_id: Option<u32>,
    pub phase_group: Option<u32>,
    pub terrain_swap_map: Option<u32>,
    pub spawn_group_id: Option<u32>,
    pub spawn_group_name: Option<String>,
    pub pool_id: Option<u32>,
    pub equipment_id: Option<u8>,
    pub original_equipment_id: Option<i8>,
    pub wander_distance: f32,
    pub respawn_delay: u32,
    pub respawn_time: i64,
    pub movement_type: MovementGeneratorType,
    pub string_id: Option<String>,
    pub is_active: bool,
    pub inactive_by_spawn_group: bool,
    pub duplicate_spawn_found: bool,
    pub add_to_map: bool,
    pub respawn_compatibility_mode: bool,
    pub formation_info: Option<CreatureFormationInfoLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedCreatureRuntimeSelectionLikeCpp {
    pub selected_level: u8,
    pub stats: CreatureLifecycleStats,
    pub selected_display_id: u32,
    /// Explicit fallback seam for model data not yet available in a complete live store.
    /// `None` is preserved honestly; no dummy dimensions are invented.
    pub selected_model_dimensions: Option<CreatureModelDimensions>,
    pub selected_equipment_id: u8,
    pub selected_original_equipment_id: i8,
    pub selected_virtual_items: [(i32, u16, u16); 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureLoadedGridResolvedLikeCpp {
    pub lifecycle_record: CreatureLoadFromDbLifecycleRecord,
    pub creature: Creature,
    pub map_object_record: Option<MapObjectRecord>,
    pub map_insertion_requested: bool,
}

pub trait LoadedGridCreatureRandomSourceLikeCpp: CreatureModelSelectionRandomLikeCpp {
    fn select_creature_level_like_cpp(&mut self, min_level: u8, max_level: u8) -> u8;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatureLoadedGridResolveErrorLikeCpp {
    MissingSpawnData {
        spawn_id: u64,
    },
    MissingTemplate {
        entry: u32,
    },
    MissingModel {
        entry: u32,
    },
    MissingRuntimeSelection {
        entry: u32,
    },
    InvalidMapObjectGuid {
        guid: ObjectGuid,
        expected_high: HighGuid,
        expected_map_id: u32,
        expected_entry: u32,
    },
    MapObjectRecord(String),
}

#[derive(Debug, Clone, Default)]
pub struct CreatureLoadedGridLifecycleResolverLikeCpp {
    templates: BTreeMap<u32, ResolvedCreatureTemplateLikeCpp>,
    spawns: BTreeMap<u64, ResolvedCreatureSpawnLikeCpp>,
    runtime_selections: BTreeMap<u32, ResolvedCreatureRuntimeSelectionLikeCpp>,
}

impl CreatureLoadedGridLifecycleResolverLikeCpp {
    pub fn new(
        templates: impl IntoIterator<Item = ResolvedCreatureTemplateLikeCpp>,
        spawns: impl IntoIterator<Item = ResolvedCreatureSpawnLikeCpp>,
        runtime_selections: impl IntoIterator<Item = (u32, ResolvedCreatureRuntimeSelectionLikeCpp)>,
    ) -> Self {
        Self {
            templates: templates
                .into_iter()
                .map(|template| (template.entry, template))
                .collect(),
            spawns: spawns
                .into_iter()
                .map(|spawn| (spawn.spawn_id, spawn))
                .collect(),
            runtime_selections: runtime_selections.into_iter().collect(),
        }
    }

    pub fn resolve_loaded_grid_creature_like_cpp(
        &self,
        spawn_id: u64,
        map_object_guid: ObjectGuid,
    ) -> Result<CreatureLoadedGridResolvedLikeCpp, CreatureLoadedGridResolveErrorLikeCpp> {
        let spawn = self
            .spawns
            .get(&spawn_id)
            .ok_or(CreatureLoadedGridResolveErrorLikeCpp::MissingSpawnData { spawn_id })?;
        let template = self
            .templates
            .get(&spawn.entry)
            .ok_or(CreatureLoadedGridResolveErrorLikeCpp::MissingTemplate { entry: spawn.entry })?;
        let selection = self.runtime_selections.get(&spawn.entry).ok_or(
            CreatureLoadedGridResolveErrorLikeCpp::MissingRuntimeSelection { entry: spawn.entry },
        )?;
        validate_map_object_guid_like_cpp(spawn, template, map_object_guid)?;

        let lifecycle_record = CreatureLoadFromDbLifecycleRecord {
            create: CreatureCreateLifecycleRecord {
                guid: map_object_guid,
                entry: template.entry,
                map_id: spawn.map_id,
                instance_id: spawn.instance_id,
                position: spawn.position,
                dynamic: false,
                vehicle_id: template.vehicle_id,
                vehicle_kit_create_input: template.vehicle_kit_create_input.clone(),
                add_to_world_vehicle_reset_context: template
                    .add_to_world_vehicle_reset_context
                    .clone(),
                template: template_lifecycle_record(template),
                spawn: Some(spawn_lifecycle_record(spawn)),
                selected_level: selection.selected_level,
                stats: selection.stats,
                selected_display_id: selection.selected_display_id,
                selected_model_dimensions: selection.selected_model_dimensions,
                selected_equipment_id: selection.selected_equipment_id,
                selected_original_equipment_id: selection.selected_original_equipment_id,
                selected_virtual_items: selection.selected_virtual_items,
                corpse_delay: template.corpse_delay,
                ignore_corpse_decay_ratio: template.ignore_corpse_decay_ratio,
                addon: template.addon.clone(),
            },
            spawn: spawn_lifecycle_record(spawn),
        };

        let mut creature = Creature::load_from_db_lifecycle(lifecycle_record.clone());
        creature.set_formation_info_like_cpp(spawn.formation_info);
        if let Some(sparring_health_pct) = template.sparring_health_pct {
            creature.set_sparring_health_pct_like_cpp(sparring_health_pct);
        }
        // This is the DB-backed boundary that has resolved and applied the
        // selected creature_addon/template_addon source. Empty local aura
        // containers can therefore be accredited as inert for the stat and
        // power-cost fields in `SpellCastLogData`; any represented aura keeps
        // the completeness check closed and every later mutation revokes it.
        creature
            .unit_mut()
            .subsystems_mut()
            .auras
            .set_spell_cast_log_aura_authority_inert_like_cpp(true);
        let map_insertion_requested = spawn.add_to_map;
        let map_object_record = if map_insertion_requested {
            Some(
                MapObjectRecord::new_creature(creature.clone()).map_err(|error| {
                    CreatureLoadedGridResolveErrorLikeCpp::MapObjectRecord(format!("{error:?}"))
                })?,
            )
        } else {
            None
        };

        Ok(CreatureLoadedGridResolvedLikeCpp {
            lifecycle_record,
            creature,
            map_object_record,
            map_insertion_requested,
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_loaded_grid_creature_inputs_from_db_like_cpp(
    spawn: &wow_map::SpawnData,
    runtime_row: &CreatureSpawnRuntimeRowLikeCpp,
    template_store: &CreatureTemplateLifecycleStoreLikeCpp,
    difficulty_store: &CreatureDifficultyStoreLikeCpp,
    base_stats_store: &CreatureBaseStatsStoreLikeCpp,
    health_rates: &CreatureClassificationHealthRatesLikeCpp,
    _display_store: &CreatureDisplayInfoStore,
    _model_store: &CreatureModelDataStore,
    model_info_store: &CreatureModelInfoStoreLikeCpp,
    equipment_store: Option<&CreatureEquipmentStoreLikeCpp>,
    addon_store: &CreatureAddonStoreLikeCpp,
    difficulty_id: u8,
    instance_id: u32,
    respawn_time: i64,
    add_to_map: bool,
    formation_info: Option<CreatureFormationInfoLikeCpp>,
    random: &mut impl LoadedGridCreatureRandomSourceLikeCpp,
) -> Result<
    (
        ResolvedCreatureTemplateLikeCpp,
        ResolvedCreatureSpawnLikeCpp,
        ResolvedCreatureRuntimeSelectionLikeCpp,
    ),
    CreatureLoadedGridResolveErrorLikeCpp,
> {
    build_loaded_grid_creature_inputs_with_power_stores_from_db_like_cpp(
        spawn,
        runtime_row,
        template_store,
        difficulty_store,
        base_stats_store,
        health_rates,
        _display_store,
        _model_store,
        model_info_store,
        equipment_store,
        addon_store,
        None,
        None,
        difficulty_id,
        instance_id,
        respawn_time,
        add_to_map,
        formation_info,
        random,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_loaded_grid_creature_inputs_with_power_stores_from_db_like_cpp(
    spawn: &wow_map::SpawnData,
    runtime_row: &CreatureSpawnRuntimeRowLikeCpp,
    template_store: &CreatureTemplateLifecycleStoreLikeCpp,
    difficulty_store: &CreatureDifficultyStoreLikeCpp,
    base_stats_store: &CreatureBaseStatsStoreLikeCpp,
    health_rates: &CreatureClassificationHealthRatesLikeCpp,
    _display_store: &CreatureDisplayInfoStore,
    _model_store: &CreatureModelDataStore,
    model_info_store: &CreatureModelInfoStoreLikeCpp,
    equipment_store: Option<&CreatureEquipmentStoreLikeCpp>,
    addon_store: &CreatureAddonStoreLikeCpp,
    chr_classes_store: Option<&ChrClassesStore>,
    power_type_store: Option<&PowerTypeStore>,
    difficulty_id: u8,
    instance_id: u32,
    respawn_time: i64,
    add_to_map: bool,
    formation_info: Option<CreatureFormationInfoLikeCpp>,
    random: &mut impl LoadedGridCreatureRandomSourceLikeCpp,
) -> Result<
    (
        ResolvedCreatureTemplateLikeCpp,
        ResolvedCreatureSpawnLikeCpp,
        ResolvedCreatureRuntimeSelectionLikeCpp,
    ),
    CreatureLoadedGridResolveErrorLikeCpp,
> {
    let template = template_store
        .get(spawn.id)
        .ok_or(CreatureLoadedGridResolveErrorLikeCpp::MissingTemplate { entry: spawn.id })?;
    let difficulty = difficulty_store.get_like_cpp(template.entry, difficulty_id);
    let selected_level = if difficulty.min_level == difficulty.max_level {
        difficulty.min_level
    } else {
        random
            .select_creature_level_like_cpp(difficulty.min_level, difficulty.max_level)
            .clamp(difficulty.min_level, difficulty.max_level)
    };
    let base_stats = base_stats_store.get_like_cpp(selected_level, template.unit_class);
    // C++ `Creature::UpdateLevelDependantStats`: GenerateHealth(...) produces basehp first,
    // then `uint32(basehp * Creature::GetHealthMod(template.Classification))` becomes
    // create/max/current health before `SetSpawnHealth` applies spawn-row current health.
    let health_rate = health_rates.modifier_for_classification_like_cpp(template.classification);
    let max_health =
        u64::from((base_stats.generate_health_like_cpp(difficulty) as f32 * health_rate) as u32);
    let base_mana = i32::try_from(base_stats.base_mana).unwrap_or(i32::MAX);
    let display_power = chr_classes_store
        .and_then(|store| store.get(u32::from(template.unit_class)))
        .map(|entry| entry.display_power)
        // C++ `Unit::CalculateDisplayPowerType` starts at POWER_MANA and only
        // replaces it when `sChrClassesStore.LookupEntry(GetClass())` succeeds.
        .unwrap_or(wow_constants::PowerType::Mana as u8);
    let power_type = power_type_from_u8_like_cpp(display_power);
    let initial_power = power_type_store.map_or_else(
        || {
            let max_power = if power_type == wow_constants::PowerType::Mana {
                i32::try_from(base_stats.generate_mana_like_cpp(difficulty)).unwrap_or(i32::MAX)
            } else {
                0
            };
            wow_data::character_progression::CreatureInitialPowerLikeCpp {
                max_power,
                power: max_power,
            }
        },
        |store| {
            store.creature_initial_power_like_cpp(
                power_type as i8,
                base_mana,
                difficulty.mana_modifier,
            )
        },
    );
    // C++ `Creature::SetSpawnHealth`: flags5 `NO_HEALTH_REGEN` returns before reading
    // `_regenerateHealth` or DB `curhealth`/`curmana`, preserving the Create/UpdateLevel-
    // DependantStats current health/mana. Otherwise `_regenerateHealth` selects full spawned
    // health/mana; DB current health is scaled by `GetHealthMod(template.Classification)` and
    // min-clamped only when non-zero.
    let flags5 = wow_constants::creature::CreatureStaticFlags5::from_bits_truncate(
        difficulty.static_flags[4],
    );
    let no_health_regen =
        flags5.contains(wow_constants::creature::CreatureStaticFlags5::NO_HEALTH_REGEN);
    let (health, power) = if no_health_regen || template.regen_health {
        (max_health, initial_power.power)
    } else {
        let health = if runtime_row.curhealth == 0 {
            0
        } else {
            ((runtime_row.curhealth as f32) * health_rate).max(1.0) as u64
        };
        (
            health,
            if power_type == wow_constants::PowerType::Mana {
                i32::try_from(runtime_row.curmana).unwrap_or(i32::MAX)
            } else {
                initial_power.power
            },
        )
    };
    let min_damage =
        base_stats.generate_base_damage_like_cpp(difficulty) * difficulty.damage_modifier;
    let spawn_model =
        (runtime_row.model_id != 0).then_some(CreatureTemplateLifecycleModelLikeCpp {
            creature_display_id: runtime_row.model_id,
            display_scale: 1.0,
            probability: 1.0,
        });
    let selected_model = template
        .choose_display_model_like_cpp(model_info_store, spawn_model, random)
        .ok_or(CreatureLoadedGridResolveErrorLikeCpp::MissingModel {
            entry: template.entry,
        })?;
    let selected_display_id = selected_model.creature_display_id;
    let selected_model_dimensions =
        model_info_store
            .get(selected_display_id)
            .map(|model_info| CreatureModelDimensions {
                bounding_radius: model_info.bounding_radius,
                combat_reach: model_info.combat_reach,
            });
    let equipment_id = u8::try_from(runtime_row.equipment_id).unwrap_or(0);
    let original_equipment_id = if equipment_id == 0 {
        0
    } else {
        runtime_row.equipment_id
    };
    let selected_virtual_items = creature_virtual_items_for_selected_equipment_like_cpp(
        template.entry,
        equipment_id,
        equipment_store,
    );
    let addon = addon_store.get_for_creature_like_cpp(spawn.spawn_id, template.entry);
    let selected_db_movement_type = addon_store
        .movement_type_after_spawn_addon_load_like_cpp(spawn.spawn_id, runtime_row.movement_type);
    let movement_type = movement_type_like_cpp(
        selected_db_movement_type,
        template.movement_type,
        runtime_row.wander_distance,
    );
    let npc_flags = runtime_row.npc_flags.unwrap_or(template.npc_flags);
    // C++ `ObjectMgr::LoadCreatureTemplates` / `LoadCreatureData` strips
    // disallowed SQL-backed creature unit flags before `ChooseCreatureFlags`
    // feeds `Creature::UpdateEntry`.
    let unit_flags = runtime_row.unit_flags.unwrap_or(template.unit_flags)
        & wow_constants::unit::UNIT_FLAGS_ALLOWED_LIKE_CPP;
    let unit_flags2 = runtime_row.unit_flags2.unwrap_or(template.unit_flags2)
        & wow_constants::unit::UNIT_FLAGS2_ALLOWED_LIKE_CPP;
    let unit_flags3 = runtime_row.unit_flags3.unwrap_or(template.unit_flags3)
        & wow_constants::unit::UNIT_FLAGS3_ALLOWED_LIKE_CPP;

    let resolved_template = ResolvedCreatureTemplateLikeCpp {
        entry: template.entry,
        original_entry: template.entry,
        difficulty_id,
        name: template.name.clone(),
        ai_name: template.ai_name.clone(),
        script_name: template.script_name.clone(),
        required_expansion: template.required_expansion,
        unit_class: template.unit_class,
        trainer_class: template.trainer_class,
        faction: template.faction,
        npc_flags,
        display_id: selected_display_id,
        model_dimensions: selected_model_dimensions,
        scale: template.scale,
        speed_walk: template.speed_walk,
        speed_run: template.speed_run,
        spells: template.spells,
        classification: template.classification,
        damage_school: template.damage_school,
        sparring_health_pct: None,
        unit_flags,
        unit_flags2,
        unit_flags3,
        flags_extra: template.flags_extra,
        static_flags: difficulty.static_flags,
        creature_type: template.creature_type,
        type_flags: difficulty.type_flags,
        loot_id: difficulty.loot_id,
        skin_loot_id: difficulty.skin_loot_id,
        gold_min: difficulty.gold_min,
        gold_max: difficulty.gold_max,
        movement_type,
        ground_movement_type: runtime_row.ground_movement_type,
        swim_allowed: runtime_row.swim_allowed,
        flight_movement_type: runtime_row.flight_movement_type,
        rooted: runtime_row.rooted,
        chase_movement_type: runtime_row.chase_movement_type,
        random_movement_type: runtime_row.random_movement_type,
        interaction_pause_timer_ms: runtime_row.interaction_pause_timer_ms,
        min_level: difficulty.min_level,
        max_level: difficulty.max_level,
        equipment_id,
        original_equipment_id,
        vehicle_id: (template.vehicle_id != 0).then_some(template.vehicle_id),
        vehicle_kit_create_input: None,
        add_to_world_vehicle_reset_context: None,
        // C++ `Creature::Creature` initializes `m_corpseDelay` to 60 seconds.
        // The DB-backed template path has no corpse-delay column, so preserve
        // that constructor default instead of turning every corpse into a
        // next-tick removal.
        corpse_delay: DEFAULT_CORPSE_DELAY_SECS,
        ignore_corpse_decay_ratio: false,
        addon,
    };
    let position = Position {
        x: spawn.spawn_point.x,
        y: spawn.spawn_point.y,
        z: spawn.spawn_point.z,
        orientation: spawn.spawn_point.orientation,
    };
    let spawn_group_id = (spawn.spawn_group.group_id != 0).then_some(spawn.spawn_group.group_id);
    let pool_id = (spawn.pool_id != 0).then_some(spawn.pool_id);
    let string_id = if runtime_row.string_id.is_empty() {
        (!spawn.string_id.is_empty()).then(|| spawn.string_id.clone())
    } else {
        Some(runtime_row.string_id.clone())
    };
    let resolved_spawn = ResolvedCreatureSpawnLikeCpp {
        spawn_id: spawn.spawn_id,
        entry: spawn.id,
        map_id: spawn.map_id,
        instance_id,
        position,
        home_position: position,
        phase_id: (spawn.phase_id != 0).then_some(spawn.phase_id),
        phase_group: (spawn.phase_group != 0).then_some(spawn.phase_group),
        terrain_swap_map: u32::try_from(spawn.terrain_swap_map).ok(),
        spawn_group_id,
        spawn_group_name: spawn_group_id.map(|_| spawn.spawn_group.name.clone()),
        pool_id,
        equipment_id: Some(equipment_id),
        original_equipment_id: Some(original_equipment_id),
        wander_distance: runtime_row.wander_distance,
        respawn_delay: u32::try_from(runtime_row.spawn_time_secs.max(0)).unwrap_or(0),
        respawn_time,
        movement_type,
        string_id,
        is_active: true,
        inactive_by_spawn_group: false,
        duplicate_spawn_found: false,
        add_to_map,
        respawn_compatibility_mode: spawn
            .spawn_group
            .flags
            .contains(wow_map::SpawnGroupFlags::COMPATIBILITY_MODE),
        formation_info,
    };
    let runtime_selection = ResolvedCreatureRuntimeSelectionLikeCpp {
        selected_level,
        stats: CreatureLifecycleStats {
            max_health,
            health,
            power_type,
            base_mana,
            max_power: initial_power.max_power,
            power,
            min_damage,
            max_damage: min_damage * 1.5,
            combat_log: CreatureCombatLogStatsLikeCpp {
                // C++ seeds the UnitMods base value through float before the
                // signed total is read by `SpellCastLogData::Initialize`.
                attack_power: (base_stats.attack_power as f32) as i32,
                ranged_attack_power: (base_stats.ranged_attack_power as f32) as i32,
                spell_power: 0,
                armor: (base_stats.generate_armor_like_cpp(difficulty) as f32) as i32,
            },
        },
        selected_display_id,
        selected_model_dimensions,
        selected_equipment_id: equipment_id,
        selected_original_equipment_id: original_equipment_id,
        selected_virtual_items,
    };

    Ok((resolved_template, resolved_spawn, runtime_selection))
}

const fn power_type_from_u8_like_cpp(power: u8) -> wow_constants::PowerType {
    use wow_constants::PowerType;

    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

fn movement_type_like_cpp(
    db_movement_type: u8,
    _template_movement_type: u8,
    wander_distance: f32,
) -> MovementGeneratorType {
    // C++ `Creature::CreateFromProto` takes concrete spawn `CreatureData::movementType` when a
    // spawn exists. The spawn-addon PathId=0 waypoint downgrade has already been applied by
    // `CreatureAddonStoreLikeCpp::movement_type_after_spawn_addon_load_like_cpp`.
    const RANDOM_MOTION_TYPE_LIKE_CPP: u8 = 1;
    const WAYPOINT_MOTION_TYPE_LIKE_CPP: u8 = 2;
    match db_movement_type {
        WAYPOINT_MOTION_TYPE_LIKE_CPP => MovementGeneratorType::Waypoint,
        RANDOM_MOTION_TYPE_LIKE_CPP if wander_distance > 0.0 => MovementGeneratorType::Random,
        _ => MovementGeneratorType::Idle,
    }
}

fn validate_map_object_guid_like_cpp(
    spawn: &ResolvedCreatureSpawnLikeCpp,
    template: &ResolvedCreatureTemplateLikeCpp,
    map_object_guid: ObjectGuid,
) -> Result<(), CreatureLoadedGridResolveErrorLikeCpp> {
    let expected_high = if template.vehicle_id.is_some() {
        HighGuid::Vehicle
    } else {
        HighGuid::Creature
    };

    if map_object_guid.high_type() != expected_high
        || u32::from(map_object_guid.map_id()) != spawn.map_id
        || map_object_guid.entry() != template.entry
    {
        return Err(
            CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                guid: map_object_guid,
                expected_high,
                expected_map_id: spawn.map_id,
                expected_entry: template.entry,
            },
        );
    }

    Ok(())
}

fn template_lifecycle_record(
    template: &ResolvedCreatureTemplateLikeCpp,
) -> CreatureTemplateLifecycleRecord {
    CreatureTemplateLifecycleRecord {
        entry: template.entry,
        original_entry: template.original_entry,
        difficulty_id: template.difficulty_id,
        name: template.name.clone(),
        ai_name: template.ai_name.clone(),
        script_name: template.script_name.clone(),
        required_expansion: template.required_expansion,
        unit_class: template.unit_class,
        trainer_class: template.trainer_class,
        faction: template.faction,
        npc_flags: template.npc_flags,
        display_id: template.display_id,
        model_dimensions: template.model_dimensions,
        scale: template.scale,
        speed_walk: template.speed_walk,
        speed_run: template.speed_run,
        spells: template.spells,
        classification: template.classification,
        damage_school: template.damage_school,
        unit_flags: template.unit_flags,
        unit_flags2: template.unit_flags2,
        unit_flags3: template.unit_flags3,
        flags_extra: template.flags_extra,
        static_flags: template.static_flags,
        creature_type: template.creature_type,
        type_flags: template.type_flags,
        loot_id: template.loot_id,
        skin_loot_id: template.skin_loot_id,
        gold_min: template.gold_min,
        gold_max: template.gold_max,
        movement_type: template.movement_type,
        ground_movement_type: template.ground_movement_type,
        swim_allowed: template.swim_allowed,
        flight_movement_type: template.flight_movement_type,
        rooted: template.rooted,
        chase_movement_type: template.chase_movement_type,
        random_movement_type: template.random_movement_type,
        interaction_pause_timer_ms: template.interaction_pause_timer_ms,
        min_level: template.min_level,
        max_level: template.max_level,
        equipment_id: template.equipment_id,
        original_equipment_id: template.original_equipment_id,
    }
}

fn spawn_lifecycle_record(spawn: &ResolvedCreatureSpawnLikeCpp) -> CreatureSpawnLifecycleRecord {
    CreatureSpawnLifecycleRecord {
        spawn_id: spawn.spawn_id,
        map_id: spawn.map_id,
        instance_id: spawn.instance_id,
        position: spawn.position,
        home_position: spawn.home_position,
        phase_id: spawn.phase_id,
        phase_group: spawn.phase_group,
        terrain_swap_map: spawn.terrain_swap_map,
        spawn_group_id: spawn.spawn_group_id,
        spawn_group_name: spawn.spawn_group_name.clone(),
        pool_id: spawn.pool_id,
        equipment_id: spawn.equipment_id,
        original_equipment_id: spawn.original_equipment_id,
        wander_distance: spawn.wander_distance,
        respawn_delay: spawn.respawn_delay,
        respawn_time: spawn.respawn_time,
        movement_type: spawn.movement_type,
        string_id: spawn.string_id.clone(),
        is_active: spawn.is_active,
        inactive_by_spawn_group: spawn.inactive_by_spawn_group,
        duplicate_spawn_found: spawn.duplicate_spawn_found,
        add_to_map: spawn.add_to_map,
        respawn_compatibility_mode: spawn.respawn_compatibility_mode,
    }
}

fn creature_virtual_items_for_selected_equipment_like_cpp(
    entry: u32,
    equipment_id: u8,
    equipment_store: Option<&CreatureEquipmentStoreLikeCpp>,
) -> [(i32, u16, u16); 3] {
    if equipment_id == 0 {
        return [(0, 0, 0); 3];
    }

    let Some(equipment) = equipment_store.and_then(|store| store.get(entry, equipment_id)) else {
        return [(0, 0, 0); 3];
    };

    equipment.items.map(|item| {
        (
            i32::try_from(item.item_id).unwrap_or(0),
            item.appearance_mod_id,
            item.item_visual,
        )
    })
}

#[cfg(test)]
#[path = "creature_loaded_grid_tests/mod.rs"]
mod tests;
