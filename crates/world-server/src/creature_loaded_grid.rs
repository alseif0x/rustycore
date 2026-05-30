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
    CreatureBaseStatsStoreLikeCpp, CreatureClassificationHealthRatesLikeCpp,
    CreatureDifficultyStoreLikeCpp, CreatureDisplayInfoStore, CreatureModelDataStore,
    CreatureTemplateLifecycleStoreLikeCpp,
};
use wow_entities::{
    Creature, CreatureAddToWorldVehicleResetContextLikeCpp, CreatureCreateLifecycleRecord,
    CreatureFormationInfoLikeCpp, CreatureLifecycleStats, CreatureLoadFromDbLifecycleRecord,
    CreatureModelDimensions, CreatureSpawnLifecycleRecord, CreatureTemplateLifecycleRecord,
    MapObjectRecord, MovementGeneratorType, VehicleKitCreateInputLikeCpp,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCreatureTemplateLikeCpp {
    pub entry: u32,
    pub original_entry: u32,
    pub difficulty_id: u8,
    pub name: String,
    pub unit_class: u8,
    pub faction: u32,
    pub display_id: u32,
    pub model_dimensions: Option<CreatureModelDimensions>,
    pub scale: f32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub spells: [u32; 8],
    pub classification: u32,
    pub flags_extra: u32,
    pub creature_type: u32,
    pub type_flags: u32,
    pub movement_type: MovementGeneratorType,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub equipment_id: u8,
    pub original_equipment_id: i8,
    pub vehicle_id: Option<u32>,
    pub vehicle_kit_create_input: Option<VehicleKitCreateInputLikeCpp>,
    pub add_to_world_vehicle_reset_context: Option<CreatureAddToWorldVehicleResetContextLikeCpp>,
    pub corpse_delay: u32,
    pub ignore_corpse_decay_ratio: bool,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureLoadedGridResolvedLikeCpp {
    pub lifecycle_record: CreatureLoadFromDbLifecycleRecord,
    pub creature: Creature,
    pub map_object_record: Option<MapObjectRecord>,
    pub map_insertion_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatureLoadedGridResolveErrorLikeCpp {
    MissingSpawnData {
        spawn_id: u64,
    },
    MissingTemplate {
        entry: u32,
    },
    MissingDifficulty {
        entry: u32,
        difficulty_id: u8,
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
                corpse_delay: template.corpse_delay,
                ignore_corpse_decay_ratio: template.ignore_corpse_decay_ratio,
            },
            spawn: spawn_lifecycle_record(spawn),
        };

        let mut creature = Creature::load_from_db_lifecycle(lifecycle_record.clone());
        creature.set_formation_info_like_cpp(spawn.formation_info);
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
    display_store: &CreatureDisplayInfoStore,
    model_store: &CreatureModelDataStore,
    difficulty_id: u8,
    instance_id: u32,
    respawn_time: i64,
    add_to_map: bool,
    formation_info: Option<CreatureFormationInfoLikeCpp>,
    mut select_level: impl FnMut(u8, u8) -> u8,
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
    let difficulty = difficulty_store
        .get_like_cpp(template.entry, difficulty_id)
        .ok_or(CreatureLoadedGridResolveErrorLikeCpp::MissingDifficulty {
            entry: template.entry,
            difficulty_id,
        })?;
    let selected_level = if difficulty.min_level == difficulty.max_level {
        difficulty.min_level
    } else {
        select_level(difficulty.min_level, difficulty.max_level)
            .clamp(difficulty.min_level, difficulty.max_level)
    };
    let base_stats = base_stats_store.get_like_cpp(selected_level, template.unit_class);
    // C++ `Creature::UpdateLevelDependantStats`: GenerateHealth(...) produces basehp first,
    // then `uint32(basehp * Creature::GetHealthMod(template.Classification))` becomes
    // create/max/current health before `SetSpawnHealth` applies spawn-row current health.
    let health_rate = health_rates.modifier_for_classification_like_cpp(template.classification);
    let max_health =
        u64::from((base_stats.generate_health_like_cpp(difficulty) as f32 * health_rate) as u32);
    let max_mana = i32::try_from(base_stats.generate_mana_like_cpp(difficulty)).unwrap_or(i32::MAX);
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
    let (health, mana) = if no_health_regen || template.regen_health {
        (max_health, max_mana)
    } else {
        let health = if runtime_row.curhealth == 0 {
            0
        } else {
            ((runtime_row.curhealth as f32) * health_rate).max(1.0) as u64
        };
        (
            health,
            i32::try_from(runtime_row.curmana).unwrap_or(i32::MAX),
        )
    };
    let min_damage =
        base_stats.generate_base_damage_like_cpp(difficulty) * difficulty.damage_modifier;
    let selected_display_id = if runtime_row.model_id != 0 {
        runtime_row.model_id
    } else {
        template
            .first_model_like_cpp()
            .map(|model| model.creature_display_id)
            .ok_or(CreatureLoadedGridResolveErrorLikeCpp::MissingModel {
                entry: template.entry,
            })?
    };
    let selected_model_dimensions = display_store
        .get(selected_display_id)
        .and_then(|display| model_store.get(u32::from(display.model_id)))
        .map(|_model| {
            // Existing Rust DB2 store does not expose C++ bounding radius/combat reach fields yet.
            // Keep dimensions absent rather than inventing a dummy; future DB2 field expansion can
            // replace this represented `None` seam.
            None
        })
        .flatten();
    let equipment_id = u8::try_from(runtime_row.equipment_id.max(0)).unwrap_or(0);
    let original_equipment_id = runtime_row.equipment_id;
    let movement_type = movement_type_like_cpp(runtime_row.movement_type, template.movement_type);

    let resolved_template = ResolvedCreatureTemplateLikeCpp {
        entry: template.entry,
        original_entry: template.entry,
        difficulty_id,
        name: template.name.clone(),
        unit_class: template.unit_class,
        faction: template.faction,
        display_id: selected_display_id,
        model_dimensions: selected_model_dimensions,
        scale: template.scale,
        speed_walk: template.speed_walk,
        speed_run: template.speed_run,
        spells: template.spells,
        classification: template.classification,
        flags_extra: template.flags_extra,
        creature_type: template.creature_type,
        type_flags: difficulty.type_flags,
        movement_type,
        ground_movement_type: runtime_row.ground_movement_type,
        swim_allowed: runtime_row.swim_allowed,
        flight_movement_type: runtime_row.flight_movement_type,
        min_level: difficulty.min_level,
        max_level: difficulty.max_level,
        equipment_id,
        original_equipment_id,
        vehicle_id: (template.vehicle_id != 0).then_some(template.vehicle_id),
        vehicle_kit_create_input: None,
        add_to_world_vehicle_reset_context: None,
        corpse_delay: 0,
        ignore_corpse_decay_ratio: false,
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
            power_type: wow_constants::PowerType::Mana,
            max_mana,
            mana,
            min_damage,
            max_damage: min_damage * 1.5,
        },
        selected_display_id,
        selected_model_dimensions,
        selected_equipment_id: equipment_id,
        selected_original_equipment_id: original_equipment_id,
    };

    Ok((resolved_template, resolved_spawn, runtime_selection))
}

fn movement_type_like_cpp(
    _db_movement_type: u8,
    _template_movement_type: u8,
) -> MovementGeneratorType {
    // `wow-entities` currently represents only Idle movement. Preserve raw DB/template values in
    // the DB-backed stores and collapse here only at the existing entity seam; no live movement
    // runtime is claimed by this builder.
    MovementGeneratorType::Idle
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
        unit_class: template.unit_class,
        faction: template.faction,
        display_id: template.display_id,
        model_dimensions: template.model_dimensions,
        scale: template.scale,
        speed_walk: template.speed_walk,
        speed_run: template.speed_run,
        spells: template.spells,
        classification: template.classification,
        flags_extra: template.flags_extra,
        creature_type: template.creature_type,
        type_flags: template.type_flags,
        movement_type: template.movement_type,
        ground_movement_type: template.ground_movement_type,
        swim_allowed: template.swim_allowed,
        flight_movement_type: template.flight_movement_type,
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

#[cfg(test)]
mod tests {
    use super::*;
    use wow_constants::PowerType;

    fn position(x: f32, y: f32, z: f32, orientation: f32) -> Position {
        Position {
            x,
            y,
            z,
            orientation,
        }
    }

    fn template(entry: u32) -> ResolvedCreatureTemplateLikeCpp {
        ResolvedCreatureTemplateLikeCpp {
            entry,
            original_entry: entry - 1,
            difficulty_id: 2,
            name: "Loaded Grid Test Creature".to_string(),
            unit_class: 1,
            faction: 35,
            display_id: 9001,
            model_dimensions: Some(CreatureModelDimensions {
                bounding_radius: 0.7,
                combat_reach: 1.5,
            }),
            scale: 1.25,
            speed_walk: 1.0,
            speed_run: 1.14286,
            spells: [11, 22, 33, 44, 55, 66, 77, 88],
            classification: 4,
            flags_extra: 0x10,
            creature_type: 0,
            type_flags: 0x20,
            movement_type: MovementGeneratorType::Idle,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            min_level: 18,
            max_level: 20,
            equipment_id: 3,
            original_equipment_id: -2,
            vehicle_id: None,
            vehicle_kit_create_input: None,
            add_to_world_vehicle_reset_context: None,
            corpse_delay: 61,
            ignore_corpse_decay_ratio: true,
        }
    }

    fn vehicle_template(entry: u32, vehicle_id: u32) -> ResolvedCreatureTemplateLikeCpp {
        ResolvedCreatureTemplateLikeCpp {
            vehicle_id: Some(vehicle_id),
            vehicle_kit_create_input: Some(VehicleKitCreateInputLikeCpp {
                vehicle_id,
                creature_entry: entry,
                loading: true,
                seat_defs: Vec::new(),
            }),
            ..template(entry)
        }
    }

    fn map_creature_guid(entry: u32, map_id: u16, counter: i64) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, map_id, 1, entry, counter)
    }

    fn map_vehicle_guid(entry: u32, map_id: u16, counter: i64) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Vehicle, 0, 1, map_id, 1, entry, counter)
    }

    fn spawn(spawn_id: u64, entry: u32, add_to_map: bool) -> ResolvedCreatureSpawnLikeCpp {
        ResolvedCreatureSpawnLikeCpp {
            spawn_id,
            entry,
            map_id: 571,
            instance_id: 9,
            position: position(100.0, 200.0, 30.0, 1.57),
            home_position: position(101.0, 201.0, 31.0, 2.57),
            phase_id: Some(5),
            phase_group: Some(6),
            terrain_swap_map: Some(7),
            spawn_group_id: Some(8),
            spawn_group_name: Some("wintergrasp-test".to_string()),
            pool_id: Some(9),
            equipment_id: Some(4),
            original_equipment_id: Some(-4),
            wander_distance: 12.5,
            respawn_delay: 300,
            respawn_time: 123_456,
            movement_type: MovementGeneratorType::Idle,
            string_id: Some("loaded_grid_string".to_string()),
            is_active: false,
            inactive_by_spawn_group: true,
            duplicate_spawn_found: true,
            add_to_map,
            respawn_compatibility_mode: true,
            formation_info: None,
        }
    }

    fn selection(entry: u32) -> (u32, ResolvedCreatureRuntimeSelectionLikeCpp) {
        (
            entry,
            ResolvedCreatureRuntimeSelectionLikeCpp {
                selected_level: 19,
                stats: CreatureLifecycleStats {
                    max_health: 1_234,
                    health: 777,
                    power_type: PowerType::Mana,
                    max_mana: 456,
                    mana: 123,
                    min_damage: 12.0,
                    max_damage: 34.0,
                },
                selected_display_id: 9002,
                selected_model_dimensions: None,
                selected_equipment_id: 6,
                selected_original_equipment_id: -6,
            },
        )
    }

    fn db_backed_spawn(entry: u32) -> wow_map::SpawnData {
        wow_map::SpawnData {
            object_type: wow_map::SpawnObjectType::Creature,
            spawn_id: 70,
            map_id: 571,
            db_data: true,
            spawn_group: wow_map::SpawnGroupTemplateData {
                group_id: 22,
                name: "compat-group".to_string(),
                map_id: 571,
                flags: wow_map::SpawnGroupFlags::COMPATIBILITY_MODE,
            },
            id: entry,
            spawn_point: wow_map::SpawnPosition::new(1.0, 2.0, 3.0, 4.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 6,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 90,
            spawn_difficulties: Vec::new(),
            script_id: 0,
            string_id: "spawn-string".to_string(),
        }
    }

    fn db_backed_template_store(entry: u32) -> CreatureTemplateLifecycleStoreLikeCpp {
        db_backed_template_store_with_regen(entry, true)
    }

    fn db_backed_template_store_with_regen(
        entry: u32,
        regen_health: bool,
    ) -> CreatureTemplateLifecycleStoreLikeCpp {
        db_backed_template_store_with_regen_and_vehicle(entry, regen_health, 0)
    }

    fn db_backed_template_store_with_regen_and_vehicle(
        entry: u32,
        regen_health: bool,
        vehicle_id: u32,
    ) -> CreatureTemplateLifecycleStoreLikeCpp {
        CreatureTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                entry,
                name: "DB Creature".to_string(),
                faction: 35,
                speed_walk: 1.0,
                speed_run: 1.14286,
                scale: 1.25,
                classification: 1,
                creature_type: 0,
                unit_class: 1,
                vehicle_id,
                movement_type: 1,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                flags_extra: 0x40,
                string_id: "template-string".to_string(),
                regen_health,
                spells: [10, 20, 0, 0, 0, 0, 0, 0],
                models: vec![
                    wow_data::CreatureTemplateLifecycleModelLikeCpp {
                        creature_display_id: 111,
                        display_scale: 1.0,
                        probability: 50.0,
                    },
                    wow_data::CreatureTemplateLifecycleModelLikeCpp {
                        creature_display_id: 222,
                        display_scale: 1.0,
                        probability: 50.0,
                    },
                ],
            },
        ])
    }

    fn db_backed_difficulty_store(entry: u32) -> CreatureDifficultyStoreLikeCpp {
        db_backed_difficulty_store_with_static_flags(entry, [0; 8])
    }

    fn db_backed_difficulty_store_with_static_flags(
        entry: u32,
        static_flags: [u32; 8],
    ) -> CreatureDifficultyStoreLikeCpp {
        CreatureDifficultyStoreLikeCpp::from_records(
            [wow_data::CreatureDifficultyRecordLikeCpp {
                entry,
                difficulty_id: 2,
                min_level: 18,
                max_level: 20,
                health_scaling_expansion: -1,
                health_modifier: 2.0,
                mana_modifier: 3.0,
                armor_modifier: 1.0,
                damage_modifier: 4.0,
                creature_difficulty_id: 0,
                type_flags: 0x55,
                type_flags2: 0,
                loot_id: 0,
                pickpocket_loot_id: 0,
                skin_loot_id: 0,
                gold_min: 0,
                gold_max: 0,
                static_flags,
            }],
            |_| 1.0,
        )
    }

    fn db_backed_base_stats_store() -> CreatureBaseStatsStoreLikeCpp {
        CreatureBaseStatsStoreLikeCpp::from_records([(
            19,
            1,
            wow_data::CreatureBaseStatsRecordLikeCpp {
                base_health: [10, 20, 100],
                base_mana: 50,
                base_armor: 0,
                attack_power: 0,
                ranged_attack_power: 0,
                base_damage: [1.0, 2.0, 5.0],
            },
        )])
    }

    fn empty_display_stores() -> (CreatureDisplayInfoStore, CreatureModelDataStore) {
        (
            CreatureDisplayInfoStore::from_entries([]),
            CreatureModelDataStore::from_entries([]),
        )
    }

    #[test]
    fn loaded_grid_db_backed_builder_maps_spawn_template_runtime_like_cpp() {
        let entry = 12_400;
        let spawn = db_backed_spawn(entry);
        let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
            spawn_id: spawn.spawn_id,
            model_id: 999,
            equipment_id: -7,
            wander_distance: 15.0,
            curhealth: 77,
            curmana: 33,
            movement_type: 2,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: wow_constants::CreatureFlightMovementType::CanFly as u8,
            string_id: "runtime-string".to_string(),
            spawn_time_secs: 300,
        };
        let (display_store, model_store) = empty_display_stores();

        let (template, resolved_spawn, runtime) =
            build_loaded_grid_creature_inputs_from_db_like_cpp(
                &spawn,
                &runtime_row,
                &db_backed_template_store(entry),
                &db_backed_difficulty_store(entry),
                &db_backed_base_stats_store(),
                &CreatureClassificationHealthRatesLikeCpp::default(),
                &display_store,
                &model_store,
                2,
                9,
                123,
                true,
                None,
                |min, max| {
                    assert_eq!((min, max), (18, 20));
                    19
                },
            )
            .expect("DB-backed builder should compose resolver inputs");

        assert_eq!(template.entry, entry);
        assert_eq!(template.name, "DB Creature");
        assert_eq!(template.faction, 35);
        assert_eq!(template.spells[0..2], [10, 20]);
        assert_eq!(template.display_id, 999);
        assert_eq!(
            template.flight_movement_type,
            wow_constants::CreatureFlightMovementType::CanFly as u8
        );
        assert_eq!(template.equipment_id, 0);
        assert_eq!(template.original_equipment_id, -7);
        assert_eq!(resolved_spawn.spawn_id, 70);
        assert_eq!(resolved_spawn.map_id, 571);
        assert_eq!(resolved_spawn.instance_id, 9);
        assert_eq!(resolved_spawn.phase_id, None);
        assert_eq!(resolved_spawn.phase_group, Some(6));
        assert_eq!(resolved_spawn.terrain_swap_map, None);
        assert_eq!(resolved_spawn.pool_id, None);
        assert_eq!(resolved_spawn.spawn_group_id, Some(22));
        assert!(resolved_spawn.respawn_compatibility_mode);
        assert_eq!(resolved_spawn.equipment_id, Some(0));
        assert_eq!(resolved_spawn.original_equipment_id, Some(-7));
        assert_eq!(resolved_spawn.wander_distance, 15.0);
        assert_eq!(resolved_spawn.respawn_delay, 300);
        assert_eq!(resolved_spawn.respawn_time, 123);
        assert_eq!(resolved_spawn.string_id.as_deref(), Some("runtime-string"));
        assert!(resolved_spawn.add_to_map);
        assert_eq!(runtime.selected_level, 19);
        assert_eq!(runtime.selected_display_id, 999);
        assert_eq!(runtime.stats.max_health, 200);
        assert_eq!(runtime.stats.health, 200);
        assert_eq!(runtime.stats.max_mana, 150);
        assert_eq!(runtime.stats.mana, 150);
        assert_eq!(runtime.stats.min_damage, 20.0);
        assert_eq!(runtime.stats.max_damage, 30.0);
    }

    #[test]
    fn loaded_grid_db_backed_builder_regen_true_scales_max_and_current_health_like_cpp() {
        let entry = 12_405;
        let spawn = db_backed_spawn(entry);
        let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
            spawn_id: spawn.spawn_id,
            model_id: 999,
            equipment_id: 1,
            wander_distance: 0.0,
            curhealth: 77,
            curmana: 33,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            string_id: String::new(),
            spawn_time_secs: 20,
        };
        let health_rates = CreatureClassificationHealthRatesLikeCpp {
            elite: 2.0,
            ..CreatureClassificationHealthRatesLikeCpp::default()
        };
        let (display_store, model_store) = empty_display_stores();

        let (_, _, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &runtime_row,
            &db_backed_template_store_with_regen(entry, true),
            &db_backed_difficulty_store(entry),
            &db_backed_base_stats_store(),
            &health_rates,
            &display_store,
            &model_store,
            2,
            0,
            0,
            false,
            None,
            |_, _| 19,
        )
        .expect("regen=true should use health-rate-scaled max health");

        assert_eq!(runtime.stats.max_health, 400);
        assert_eq!(runtime.stats.health, 400);
        assert_eq!(runtime.stats.max_mana, 150);
        assert_eq!(runtime.stats.mana, 150);
    }

    #[test]
    fn loaded_grid_db_backed_builder_flags5_no_health_regen_preserves_initial_stats_like_cpp() {
        let entry = 12_406;
        let spawn = db_backed_spawn(entry);
        let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
            spawn_id: spawn.spawn_id,
            model_id: 999,
            equipment_id: 1,
            wander_distance: 0.0,
            curhealth: 77,
            curmana: 33,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            string_id: String::new(),
            spawn_time_secs: 20,
        };
        let health_rates = CreatureClassificationHealthRatesLikeCpp {
            elite: 2.0,
            ..CreatureClassificationHealthRatesLikeCpp::default()
        };
        let mut static_flags = [0; 8];
        static_flags[4] = wow_constants::creature::CreatureStaticFlags5::NO_HEALTH_REGEN.bits();
        let (display_store, model_store) = empty_display_stores();

        let (_, _, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &runtime_row,
            &db_backed_template_store_with_regen(entry, false),
            &db_backed_difficulty_store_with_static_flags(entry, static_flags),
            &db_backed_base_stats_store(),
            &health_rates,
            &display_store,
            &model_store,
            2,
            0,
            0,
            false,
            None,
            |_, _| 19,
        )
        .expect("flags5 NO_HEALTH_REGEN should preserve initial spawned stats");

        assert_eq!(runtime.stats.max_health, 400);
        assert_eq!(runtime.stats.health, runtime.stats.max_health);
        assert_eq!(runtime.stats.max_mana, 150);
        assert_eq!(runtime.stats.mana, runtime.stats.max_mana);
        assert_ne!(runtime.stats.health, u64::from(runtime_row.curhealth) * 2);
        assert_ne!(
            runtime.stats.mana,
            i32::try_from(runtime_row.curmana).unwrap()
        );
    }

    #[test]
    fn loaded_grid_db_backed_builder_errors_without_silent_fallbacks_like_cpp() {
        let entry = 12_401;
        let spawn = db_backed_spawn(entry);
        let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
            spawn_id: spawn.spawn_id,
            model_id: 0,
            equipment_id: 1,
            wander_distance: 0.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            string_id: String::new(),
            spawn_time_secs: 10,
        };
        let (display_store, model_store) = empty_display_stores();

        assert_eq!(
            build_loaded_grid_creature_inputs_from_db_like_cpp(
                &spawn,
                &runtime_row,
                &CreatureTemplateLifecycleStoreLikeCpp::default(),
                &db_backed_difficulty_store(entry),
                &db_backed_base_stats_store(),
                &CreatureClassificationHealthRatesLikeCpp::default(),
                &display_store,
                &model_store,
                2,
                0,
                0,
                false,
                None,
                |_, _| 19,
            ),
            Err(CreatureLoadedGridResolveErrorLikeCpp::MissingTemplate { entry })
        );
        assert_eq!(
            build_loaded_grid_creature_inputs_from_db_like_cpp(
                &spawn,
                &runtime_row,
                &db_backed_template_store(entry),
                &CreatureDifficultyStoreLikeCpp::default(),
                &db_backed_base_stats_store(),
                &CreatureClassificationHealthRatesLikeCpp::default(),
                &display_store,
                &model_store,
                2,
                0,
                0,
                false,
                None,
                |_, _| 19,
            ),
            Err(CreatureLoadedGridResolveErrorLikeCpp::MissingDifficulty {
                entry,
                difficulty_id: 2
            })
        );
    }

    #[test]
    fn loaded_grid_db_backed_builder_uses_first_template_model_and_full_health_fallback_like_cpp() {
        let entry = 12_402;
        let spawn = db_backed_spawn(entry);
        let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
            spawn_id: spawn.spawn_id,
            model_id: 0,
            equipment_id: 3,
            wander_distance: 0.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            string_id: String::new(),
            spawn_time_secs: 20,
        };
        let (display_store, model_store) = empty_display_stores();
        let (_, resolved_spawn, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &runtime_row,
            &db_backed_template_store(entry),
            &CreatureDifficultyStoreLikeCpp::from_records(
                [wow_data::CreatureDifficultyRecordLikeCpp {
                    min_level: 19,
                    max_level: 19,
                    ..db_backed_difficulty_store(entry)
                        .get_like_cpp(entry, 2)
                        .unwrap()
                        .clone()
                }],
                |_| 1.0,
            ),
            &db_backed_base_stats_store(),
            &CreatureClassificationHealthRatesLikeCpp::default(),
            &display_store,
            &model_store,
            2,
            0,
            0,
            false,
            None,
            |_, _| panic!("equal-level path must not call selector"),
        )
        .expect("first template model/full health fallback should resolve");

        assert_eq!(runtime.selected_display_id, 111);
        assert_eq!(runtime.stats.health, runtime.stats.max_health);
        assert_eq!(runtime.stats.mana, runtime.stats.max_mana);
        assert_eq!(resolved_spawn.string_id.as_deref(), Some("spawn-string"));
        assert!(!resolved_spawn.add_to_map);
    }

    #[test]
    fn loaded_grid_db_backed_builder_regen_false_preserves_zero_health_and_db_mana_like_cpp() {
        let entry = 12_403;
        let spawn = db_backed_spawn(entry);
        let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
            spawn_id: spawn.spawn_id,
            model_id: 0,
            equipment_id: 3,
            wander_distance: 0.0,
            curhealth: 0,
            curmana: 33,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            string_id: String::new(),
            spawn_time_secs: 20,
        };
        let (display_store, model_store) = empty_display_stores();
        let health_rates = CreatureClassificationHealthRatesLikeCpp {
            elite: 2.0,
            ..CreatureClassificationHealthRatesLikeCpp::default()
        };

        let (_, _, runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &runtime_row,
            &db_backed_template_store_with_regen(entry, false),
            &db_backed_difficulty_store(entry),
            &db_backed_base_stats_store(),
            &health_rates,
            &display_store,
            &model_store,
            2,
            0,
            0,
            false,
            None,
            |_, _| 19,
        )
        .expect("regen=false zero current health should preserve dead DB health");

        assert_eq!(runtime.stats.max_health, 400);
        assert_eq!(runtime.stats.health, 0);
        assert_eq!(runtime.stats.max_mana, 150);
        assert_eq!(runtime.stats.mana, 33);
    }

    #[test]
    fn loaded_grid_db_backed_builder_regen_false_scales_current_health_and_min_one_like_cpp() {
        let entry = 12_404;
        let spawn = db_backed_spawn(entry);
        let (display_store, model_store) = empty_display_stores();
        let health_rates = CreatureClassificationHealthRatesLikeCpp {
            elite: 0.25,
            ..CreatureClassificationHealthRatesLikeCpp::default()
        };

        let low_health_row = CreatureSpawnRuntimeRowLikeCpp {
            spawn_id: spawn.spawn_id,
            model_id: 0,
            equipment_id: 3,
            wander_distance: 0.0,
            curhealth: 1,
            curmana: 44,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            string_id: String::new(),
            spawn_time_secs: 20,
        };
        let (_, _, low_health_runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &low_health_row,
            &db_backed_template_store_with_regen(entry, false),
            &db_backed_difficulty_store(entry),
            &db_backed_base_stats_store(),
            &health_rates,
            &display_store,
            &model_store,
            2,
            0,
            0,
            false,
            None,
            |_, _| 19,
        )
        .expect("regen=false non-zero current health should min-clamp after scaling");
        assert_eq!(low_health_runtime.stats.max_health, 50);
        assert_eq!(low_health_runtime.stats.health, 1);
        assert_eq!(low_health_runtime.stats.mana, 44);

        let scaled_health_row = CreatureSpawnRuntimeRowLikeCpp {
            curhealth: 80,
            curmana: 55,
            ..low_health_row
        };
        let (_, _, scaled_health_runtime) = build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &scaled_health_row,
            &db_backed_template_store_with_regen(entry, false),
            &db_backed_difficulty_store(entry),
            &db_backed_base_stats_store(),
            &health_rates,
            &display_store,
            &model_store,
            2,
            0,
            0,
            false,
            None,
            |_, _| 19,
        )
        .expect("regen=false current health should scale by classification health rate");
        assert_eq!(scaled_health_runtime.stats.max_health, 50);
        assert_eq!(scaled_health_runtime.stats.health, 20);
        assert_eq!(scaled_health_runtime.stats.mana, 55);
    }

    #[test]
    fn loaded_grid_db_backed_builder_preserves_vehicle_template_id_like_cpp() {
        let entry = 12_407;
        let spawn = db_backed_spawn(entry);
        let runtime_row = CreatureSpawnRuntimeRowLikeCpp {
            spawn_id: spawn.spawn_id,
            model_id: 999,
            equipment_id: 1,
            wander_distance: 0.0,
            curhealth: 0,
            curmana: 0,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            string_id: String::new(),
            spawn_time_secs: 20,
        };
        let (display_store, model_store) = empty_display_stores();

        let (template, _, _) = build_loaded_grid_creature_inputs_from_db_like_cpp(
            &spawn,
            &runtime_row,
            &db_backed_template_store_with_regen_and_vehicle(entry, true, 77),
            &db_backed_difficulty_store(entry),
            &db_backed_base_stats_store(),
            &CreatureClassificationHealthRatesLikeCpp::default(),
            &display_store,
            &model_store,
            2,
            0,
            0,
            false,
            None,
            |_, _| 19,
        )
        .expect("DB-backed vehicle template should compose resolver inputs");

        assert_eq!(template.vehicle_id, Some(77));
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_maps_spawn_template_and_selection_like_cpp() {
        let entry = 12_345;
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [spawn(55, entry, true)],
            [selection(entry)],
        );

        let map_object_guid = map_creature_guid(entry, 571, 99_001);
        let resolved = resolver
            .resolve_loaded_grid_creature_like_cpp(55, map_object_guid)
            .expect("resolver should build lifecycle record");
        let record = &resolved.lifecycle_record;
        let creature = &resolved.creature;
        let metadata = creature.lifecycle_metadata();

        assert_eq!(record.create.entry, entry);
        assert_eq!(record.create.guid, map_object_guid);
        assert_eq!(record.create.guid.high_type(), HighGuid::Creature);
        assert_eq!(u32::from(record.create.guid.map_id()), 571);
        assert_eq!(record.create.template.original_entry, entry - 1);
        assert_eq!(record.create.map_id, 571);
        assert_eq!(record.create.instance_id, 9);
        assert_eq!(record.spawn.spawn_id, 55);
        assert_eq!(record.spawn.position, position(100.0, 200.0, 30.0, 1.57));
        assert_eq!(
            record.spawn.home_position,
            position(101.0, 201.0, 31.0, 2.57)
        );
        assert_eq!(record.spawn.respawn_delay, 300);
        assert_eq!(record.spawn.respawn_time, 123_456);
        assert_eq!(record.spawn.movement_type, MovementGeneratorType::Idle);
        assert_eq!(
            record.spawn.string_id.as_deref(),
            Some("loaded_grid_string")
        );
        assert_eq!(record.spawn.spawn_group_id, Some(8));
        assert_eq!(record.spawn.pool_id, Some(9));
        assert!(record.spawn.inactive_by_spawn_group);
        assert!(record.spawn.duplicate_spawn_found);
        assert_eq!(record.spawn.equipment_id, Some(4));
        assert_eq!(record.spawn.original_equipment_id, Some(-4));
        assert_eq!(record.create.selected_level, 19);
        assert_eq!(record.create.stats.health, 777);
        assert_eq!(record.create.selected_display_id, 9002);
        assert_eq!(record.create.selected_model_dimensions, None);

        assert_eq!(metadata.spawn_id, 55);
        assert_eq!(metadata.spawn_map_id, 571);
        assert_eq!(metadata.spawn_instance_id, 9);
        assert_eq!(metadata.spawn_position, position(100.0, 200.0, 30.0, 1.57));
        assert_eq!(metadata.home_position, position(101.0, 201.0, 31.0, 2.57));
        assert_eq!(metadata.phase_id, Some(5));
        assert_eq!(metadata.terrain_swap_map, Some(7));
        assert_eq!(
            metadata.spawn_group_name.as_deref(),
            Some("wintergrasp-test")
        );
        assert_eq!(metadata.pool_id, Some(9));
        assert!(!metadata.is_spawn_active);
        assert!(metadata.inactive_by_spawn_group);
        assert!(metadata.duplicate_spawn_found);
        assert_eq!(metadata.equipment_id, 4);
        assert_eq!(metadata.original_equipment_id, -4);
        assert_eq!(creature.ai_current_health(), 777);
        assert_eq!(creature.ai_max_health(), 1_234);
        assert_eq!(creature.ai_level(), 19);
        assert!(resolved.map_insertion_requested);
        assert!(resolved.map_object_record.is_some());
        assert!(
            resolved
                .map_object_record
                .as_ref()
                .and_then(MapObjectRecord::creature)
                .is_some()
        );
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_uses_caller_map_guid_not_spawn_id_low() {
        let entry = 12_349;
        let spawn_id = 61;
        let caller_low_guid = 345_678;
        let map_object_guid = map_creature_guid(entry, 571, caller_low_guid);
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [spawn(spawn_id, entry, true)],
            [selection(entry)],
        );

        let resolved = resolver
            .resolve_loaded_grid_creature_like_cpp(spawn_id, map_object_guid)
            .expect("caller-owned map guid should be preserved");

        assert_ne!(spawn_id as i64, caller_low_guid);
        assert_eq!(resolved.lifecycle_record.create.guid, map_object_guid);
        assert_eq!(resolved.creature.guid(), map_object_guid);
        let recorded = resolved
            .map_object_record
            .as_ref()
            .and_then(MapObjectRecord::creature)
            .expect("map insertion record should contain the created creature");
        assert_eq!(recorded.guid(), map_object_guid);
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_propagates_formation_info_to_record_like_cpp() {
        let entry = 12_350;
        let spawn_id = 62;
        let formation_info = CreatureFormationInfoLikeCpp {
            leader_spawn_id: 62,
            follow_dist: 0.0,
            follow_angle_radians: 0.0,
            group_ai: 7,
            leader_waypoint_ids: [101, 102],
        };
        let mut resolved_spawn = spawn(spawn_id, entry, true);
        resolved_spawn.formation_info = Some(formation_info);
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [resolved_spawn],
            [selection(entry)],
        );
        let resolved = resolver
            .resolve_loaded_grid_creature_like_cpp(spawn_id, map_creature_guid(entry, 571, 99_062))
            .expect("formation metadata should not block loaded-grid resolver");

        assert_eq!(
            resolved.creature.formation_info_like_cpp(),
            Some(&formation_info)
        );
        assert_eq!(
            resolved
                .map_object_record
                .as_ref()
                .and_then(MapObjectRecord::creature)
                .and_then(Creature::formation_info_like_cpp),
            Some(&formation_info)
        );
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_preserves_absent_formation_info_like_cpp() {
        let entry = 12_351;
        let spawn_id = 63;
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [spawn(spawn_id, entry, true)],
            [selection(entry)],
        );
        let resolved = resolver
            .resolve_loaded_grid_creature_like_cpp(spawn_id, map_creature_guid(entry, 571, 99_063))
            .expect("absence of formation metadata is the previous behavior");

        assert!(resolved.creature.formation_info_like_cpp().is_none());
        assert!(
            resolved
                .map_object_record
                .as_ref()
                .and_then(MapObjectRecord::creature)
                .and_then(Creature::formation_info_like_cpp)
                .is_none()
        );
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_respects_add_to_map_request_flag() {
        let entry = 12_346;
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [spawn(56, entry, false)],
            [selection(entry)],
        );

        let map_object_guid = map_creature_guid(entry, 571, 99_002);
        let resolved = resolver
            .resolve_loaded_grid_creature_like_cpp(56, map_object_guid)
            .expect("resolver should build creature without insertion request");

        assert!(!resolved.map_insertion_requested);
        assert!(resolved.map_object_record.is_none());
        assert!(
            !resolved
                .creature
                .lifecycle_metadata()
                .map_insertion_requested
        );
        assert!(!resolved.creature.lifecycle_metadata().add_to_map_requested);
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_errors_without_dummy_for_missing_inputs() {
        let entry = 12_347;
        let map_object_guid = map_creature_guid(entry, 571, 99_003);
        let missing_spawn = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [],
            [selection(entry)],
        );
        assert_eq!(
            missing_spawn.resolve_loaded_grid_creature_like_cpp(57, map_object_guid),
            Err(CreatureLoadedGridResolveErrorLikeCpp::MissingSpawnData { spawn_id: 57 })
        );

        let missing_template = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [],
            [spawn(58, entry, true)],
            [selection(entry)],
        );
        assert_eq!(
            missing_template.resolve_loaded_grid_creature_like_cpp(58, map_object_guid),
            Err(CreatureLoadedGridResolveErrorLikeCpp::MissingTemplate { entry })
        );

        let missing_selection = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [spawn(59, entry, true)],
            [],
        );
        assert_eq!(
            missing_selection.resolve_loaded_grid_creature_like_cpp(59, map_object_guid),
            Err(CreatureLoadedGridResolveErrorLikeCpp::MissingRuntimeSelection { entry })
        );
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_rejects_wrong_map_or_high_guid() {
        let entry = 12_350;
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [spawn(62, entry, true)],
            [selection(entry)],
        );
        let wrong_map_guid = map_creature_guid(entry, 530, 99_004);
        assert_eq!(
            resolver.resolve_loaded_grid_creature_like_cpp(62, wrong_map_guid),
            Err(
                CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                    guid: wrong_map_guid,
                    expected_high: HighGuid::Creature,
                    expected_map_id: 571,
                    expected_entry: entry,
                }
            )
        );

        let wrong_high_guid =
            ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 1, entry, 99_005);
        assert_eq!(
            resolver.resolve_loaded_grid_creature_like_cpp(62, wrong_high_guid),
            Err(
                CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                    guid: wrong_high_guid,
                    expected_high: HighGuid::Creature,
                    expected_map_id: 571,
                    expected_entry: entry,
                }
            )
        );
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_rejects_same_map_wrong_entry_guid() {
        let entry = 12_352;
        let wrong_entry = entry + 1;
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [spawn(64, entry, true)],
            [selection(entry)],
        );
        let wrong_entry_guid = map_creature_guid(wrong_entry, 571, 99_008);

        assert_eq!(
            resolver.resolve_loaded_grid_creature_like_cpp(64, wrong_entry_guid),
            Err(
                CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                    guid: wrong_entry_guid,
                    expected_high: HighGuid::Creature,
                    expected_map_id: 571,
                    expected_entry: entry,
                }
            )
        );
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_accepts_vehicle_template_with_vehicle_guid_like_cpp()
    {
        let entry = 12_351;
        let map_object_guid = map_vehicle_guid(entry, 571, 99_006);
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [vehicle_template(entry, 77)],
            [spawn(63, entry, true)],
            [selection(entry)],
        );

        let resolved = resolver
            .resolve_loaded_grid_creature_like_cpp(63, map_object_guid)
            .expect("vehicle-template Creature should resolve with HighGuid::Vehicle");

        assert_eq!(resolved.lifecycle_record.create.vehicle_id, Some(77));
        assert_eq!(
            resolved.lifecycle_record.create.guid.high_type(),
            HighGuid::Vehicle
        );
        assert_eq!(resolved.creature.guid(), map_object_guid);
        assert_eq!(resolved.creature.lifecycle_metadata().vehicle_id, Some(77));
        let kit = resolved
            .creature
            .unit()
            .subsystems()
            .vehicle
            .kit
            .as_ref()
            .unwrap();
        assert_eq!(kit.kit_id(), 77);
        assert!(kit.active());
        assert!(!kit.installed());
        assert_eq!(kit.seat_count(), 0);
        let recorded = resolved
            .map_object_record
            .as_ref()
            .and_then(MapObjectRecord::creature)
            .expect("vehicle-template map record should remain typed Creature");
        assert_eq!(recorded.guid().high_type(), HighGuid::Vehicle);
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_rejects_creature_guid_for_vehicle_template_like_cpp()
    {
        let entry = 12_353;
        let wrong_guid = map_creature_guid(entry, 571, 99_009);
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [vehicle_template(entry, 88)],
            [spawn(65, entry, true)],
            [selection(entry)],
        );

        assert_eq!(
            resolver.resolve_loaded_grid_creature_like_cpp(65, wrong_guid),
            Err(
                CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                    guid: wrong_guid,
                    expected_high: HighGuid::Vehicle,
                    expected_map_id: 571,
                    expected_entry: entry,
                }
            )
        );
    }

    #[test]
    fn loaded_grid_creature_lifecycle_resolver_is_pure_ordered_bridge_like_cpp() {
        let plan = wow_entities::CreatureLifecyclePlan::trinity_create_load_from_db();
        assert!(plan.occurs_before(
            wow_entities::CreatureLifecycleStep::LookupTemplateAndDifficulty,
            wow_entities::CreatureLifecycleStep::InitEntryAndCreateFromProto,
        ));
        assert!(plan.occurs_before(
            wow_entities::CreatureLifecycleStep::LoadFromDbSpawnHomeRespawnInactiveChecks,
            wow_entities::CreatureLifecycleStep::AddToMap,
        ));

        let entry = 12_348;
        let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
            [template(entry)],
            [spawn(60, entry, true)],
            [selection(entry)],
        );
        let map_object_guid = map_creature_guid(entry, 571, 99_007);
        let first = resolver
            .resolve_loaded_grid_creature_like_cpp(60, map_object_guid)
            .unwrap();
        let second = resolver
            .resolve_loaded_grid_creature_like_cpp(60, map_object_guid)
            .unwrap();

        assert_eq!(first.lifecycle_record, second.lifecycle_record);
        assert_eq!(
            first.creature.lifecycle_metadata(),
            second.creature.lifecycle_metadata()
        );
        assert!(first.map_insertion_requested);
        assert!(second.map_insertion_requested);
    }
}
