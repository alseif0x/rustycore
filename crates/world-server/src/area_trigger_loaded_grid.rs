// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#![allow(dead_code)]

//! Pure loaded-grid AreaTrigger lifecycle resolver for DB-backed static spawns.
//!
//! C++ anchors:
//! - `/home/server/woltk-trinity-legacy/src/server/game/Grids/ObjectGridLoader.cpp:145-151`
//!   `ObjectGridLoader::Visit(AreaTriggerMapType&)`: static area trigger spawn ids are loaded by cell.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp:104-255`
//!   `AreaTrigger::Create`: map binding, GUID entry, static duration/data fields and AI init.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp:272-296`
//!   `AreaTrigger::LoadFromDB`: spawn/create-properties lookup plus SpellForVisuals visual id.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.cpp:38-75`
//!   `AreaTriggerShapeInfo::GetMaxSearchRadius`.

use std::collections::BTreeMap;

use crate::spawn_store_loader::AreaTriggerSpawnRuntimeRowLikeCpp;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::area_trigger_template::AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK3_LIKE_CPP;
use wow_data::{
    AreaTriggerCreatePropertiesLikeCpp, AreaTriggerIdLikeCpp, AreaTriggerShapeInfoLikeCpp,
    AreaTriggerTemplateLikeCpp,
};
use wow_entities::{
    AREA_TRIGGER_FLAG_IS_SERVER_SIDE, AreaTrigger, AreaTriggerId, AreaTriggerShapeType,
    MapObjectRecord, VisualAnimValues,
};
use wow_map::{Map, SpawnData, SpawnObjectType, map::LoadedGridRespawnRecordsLikeCpp};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAreaTriggerCreatePropertiesLikeCpp {
    pub id: AreaTriggerId,
    pub template_id: Option<AreaTriggerId>,
    pub template_flags: u32,
    pub flags: u32,
    pub anim_id: i32,
    pub anim_kit_id: i32,
    pub decal_properties_id: u32,
    pub shape: AreaTriggerShapeInfoLikeCpp,
}

impl ResolvedAreaTriggerCreatePropertiesLikeCpp {
    pub fn shape_type_like_cpp(&self) -> AreaTriggerShapeType {
        area_trigger_shape_type_like_cpp(self.shape.shape_type)
    }

    pub fn bounds_radius_2d_like_cpp(&self) -> f32 {
        area_trigger_shape_max_search_radius_like_cpp(&self.shape)
    }

    pub fn visual_anim_field_c_like_cpp(&self) -> bool {
        (self.flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK3_LIKE_CPP) != 0
    }

    pub fn decal_properties_id_like_cpp(&self) -> u32 {
        if (self.template_flags & AREA_TRIGGER_FLAG_IS_SERVER_SIDE) != 0 {
            24
        } else {
            self.decal_properties_id
        }
    }

    pub fn guid_entry_like_cpp(&self) -> u32 {
        self.template_id.map(|id| id.id).unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAreaTriggerSpawnLikeCpp {
    pub spawn_id: u64,
    pub create_properties_id: AreaTriggerId,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub spell_for_visuals: Option<i32>,
    pub spell_visual_id: i32,
    pub add_to_map: bool,
    pub phase_use_flags: u8,
    pub phase_id: u32,
    pub phase_group: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerLoadedGridResolvedLikeCpp {
    pub area_trigger: AreaTrigger,
    pub map_object_record: Option<MapObjectRecord>,
    pub map_insertion_requested: bool,
    pub db_phase_shift_requested: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AreaTriggerLoadedGridResolveErrorLikeCpp {
    MissingSpawnData {
        spawn_id: u64,
    },
    MissingCreateProperties {
        create_properties_id: AreaTriggerId,
    },
    WrongSpawnObjectType {
        spawn_id: u64,
        object_type: SpawnObjectType,
    },
    MismatchedRuntimeSpawn {
        expected_spawn_id: u64,
        runtime_spawn_id: u64,
    },
    MismatchedCreatePropertiesId {
        spawn_id: u64,
        spawn_create_properties_id: u32,
        runtime_create_properties_id: AreaTriggerIdLikeCpp,
        create_properties_id: AreaTriggerIdLikeCpp,
    },
    MismatchedTemplateId {
        create_properties_id: AreaTriggerIdLikeCpp,
        expected_template_id: Option<AreaTriggerIdLikeCpp>,
        template_id: Option<AreaTriggerIdLikeCpp>,
    },
    MapMismatch {
        spawn_id: u64,
        spawn_map_id: u32,
        spawn_instance_id: u32,
        map_id: u32,
        instance_id: u32,
    },
    MapIdOutOfRange {
        spawn_id: u64,
        map_id: u32,
    },
    MapOwnedLowGuidGeneration {
        spawn_id: u64,
        error: String,
    },
    MissingMapObjectRecord {
        spawn_id: u64,
    },
    InvalidMapObjectGuid {
        guid: ObjectGuid,
        expected_high: HighGuid,
        expected_map_id: u32,
        expected_entry: u32,
    },
    MapBinding(String),
    MapObjectRecord(String),
}

pub fn resolve_area_trigger_loaded_grid_inputs_from_spawn_data_like_cpp(
    spawn: &SpawnData,
    runtime_row: &AreaTriggerSpawnRuntimeRowLikeCpp,
    create_properties: &AreaTriggerCreatePropertiesLikeCpp,
    template: Option<&AreaTriggerTemplateLikeCpp>,
    instance_id: u32,
    add_to_map: bool,
    spell_visual_id: i32,
) -> Result<
    (
        ResolvedAreaTriggerCreatePropertiesLikeCpp,
        ResolvedAreaTriggerSpawnLikeCpp,
    ),
    AreaTriggerLoadedGridResolveErrorLikeCpp,
> {
    if spawn.object_type != SpawnObjectType::AreaTrigger {
        return Err(
            AreaTriggerLoadedGridResolveErrorLikeCpp::WrongSpawnObjectType {
                spawn_id: spawn.spawn_id,
                object_type: spawn.object_type,
            },
        );
    }
    if runtime_row.spawn_id != spawn.spawn_id {
        return Err(
            AreaTriggerLoadedGridResolveErrorLikeCpp::MismatchedRuntimeSpawn {
                expected_spawn_id: spawn.spawn_id,
                runtime_spawn_id: runtime_row.spawn_id,
            },
        );
    }
    if spawn.id != runtime_row.create_properties_id.id
        || create_properties.id != runtime_row.create_properties_id
    {
        return Err(
            AreaTriggerLoadedGridResolveErrorLikeCpp::MismatchedCreatePropertiesId {
                spawn_id: spawn.spawn_id,
                spawn_create_properties_id: spawn.id,
                runtime_create_properties_id: runtime_row.create_properties_id,
                create_properties_id: create_properties.id,
            },
        );
    }

    let template_id = template.map(|template| template.id);
    if create_properties.template_id != template_id {
        return Err(
            AreaTriggerLoadedGridResolveErrorLikeCpp::MismatchedTemplateId {
                create_properties_id: create_properties.id,
                expected_template_id: create_properties.template_id,
                template_id,
            },
        );
    }

    Ok((
        ResolvedAreaTriggerCreatePropertiesLikeCpp {
            id: area_trigger_id_from_data_like_cpp(create_properties.id),
            template_id: create_properties
                .template_id
                .map(area_trigger_id_from_data_like_cpp),
            template_flags: template.map(|template| template.flags).unwrap_or(0),
            flags: create_properties.flags,
            anim_id: create_properties.anim_id,
            anim_kit_id: create_properties.anim_kit_id,
            decal_properties_id: create_properties.decal_properties_id,
            shape: create_properties.shape.clone(),
        },
        ResolvedAreaTriggerSpawnLikeCpp {
            spawn_id: spawn.spawn_id,
            create_properties_id: area_trigger_id_from_data_like_cpp(
                runtime_row.create_properties_id,
            ),
            map_id: spawn.map_id,
            instance_id,
            position: Position::new(
                spawn.spawn_point.x,
                spawn.spawn_point.y,
                spawn.spawn_point.z,
                spawn.spawn_point.orientation,
            ),
            spell_for_visuals: runtime_row.spell_for_visuals,
            spell_visual_id,
            add_to_map,
            phase_use_flags: spawn.phase_use_flags,
            phase_id: spawn.phase_id,
            phase_group: spawn.phase_group,
        },
    ))
}

pub fn build_loaded_grid_area_trigger_record_from_spawn_data_like_cpp(
    map: &mut Map,
    spawn: &SpawnData,
    runtime_row: &AreaTriggerSpawnRuntimeRowLikeCpp,
    create_properties: &AreaTriggerCreatePropertiesLikeCpp,
    template: Option<&AreaTriggerTemplateLikeCpp>,
    spell_visual_id: i32,
) -> Result<LoadedGridRespawnRecordsLikeCpp, AreaTriggerLoadedGridResolveErrorLikeCpp> {
    let (create_properties, spawn) =
        resolve_area_trigger_loaded_grid_inputs_from_spawn_data_like_cpp(
            spawn,
            runtime_row,
            create_properties,
            template,
            map.instance_id(),
            true,
            spell_visual_id,
        )?;
    build_loaded_grid_area_trigger_record_like_cpp(map, &create_properties, &spawn)
}

pub fn build_loaded_grid_area_trigger_record_like_cpp(
    map: &mut Map,
    create_properties: &ResolvedAreaTriggerCreatePropertiesLikeCpp,
    spawn: &ResolvedAreaTriggerSpawnLikeCpp,
) -> Result<LoadedGridRespawnRecordsLikeCpp, AreaTriggerLoadedGridResolveErrorLikeCpp> {
    if map.map_id() != spawn.map_id || map.instance_id() != spawn.instance_id {
        return Err(AreaTriggerLoadedGridResolveErrorLikeCpp::MapMismatch {
            spawn_id: spawn.spawn_id,
            spawn_map_id: spawn.map_id,
            spawn_instance_id: spawn.instance_id,
            map_id: map.map_id(),
            instance_id: map.instance_id(),
        });
    }
    let Ok(map_id) = u16::try_from(map.map_id()) else {
        return Err(AreaTriggerLoadedGridResolveErrorLikeCpp::MapIdOutOfRange {
            spawn_id: spawn.spawn_id,
            map_id: map.map_id(),
        });
    };
    let low = map
        .generate_low_guid_like_cpp(HighGuid::AreaTrigger)
        .map_err(
            |error| AreaTriggerLoadedGridResolveErrorLikeCpp::MapOwnedLowGuidGeneration {
                spawn_id: spawn.spawn_id,
                error: format!("{error:?}"),
            },
        )?;
    let map_object_guid = ObjectGuid::create_world_object(
        HighGuid::AreaTrigger,
        0,
        1,
        map_id,
        1,
        create_properties.guid_entry_like_cpp(),
        low,
    );
    let resolver = AreaTriggerLoadedGridLifecycleResolverLikeCpp::new(
        [create_properties.clone()],
        [spawn.clone()],
    );
    let resolved =
        resolver.resolve_loaded_grid_area_trigger_like_cpp(spawn.spawn_id, map_object_guid)?;
    let primary_record = resolved.map_object_record.ok_or(
        AreaTriggerLoadedGridResolveErrorLikeCpp::MissingMapObjectRecord {
            spawn_id: spawn.spawn_id,
        },
    )?;
    Ok(LoadedGridRespawnRecordsLikeCpp::primary_only(
        primary_record,
    ))
}

#[derive(Debug, Clone, Default)]
pub struct AreaTriggerLoadedGridLifecycleResolverLikeCpp {
    create_properties:
        BTreeMap<AreaTriggerIdKeyLikeCpp, ResolvedAreaTriggerCreatePropertiesLikeCpp>,
    spawns: BTreeMap<u64, ResolvedAreaTriggerSpawnLikeCpp>,
}

impl AreaTriggerLoadedGridLifecycleResolverLikeCpp {
    pub fn new(
        create_properties: impl IntoIterator<Item = ResolvedAreaTriggerCreatePropertiesLikeCpp>,
        spawns: impl IntoIterator<Item = ResolvedAreaTriggerSpawnLikeCpp>,
    ) -> Self {
        Self {
            create_properties: create_properties
                .into_iter()
                .map(|properties| (AreaTriggerIdKeyLikeCpp::from(properties.id), properties))
                .collect(),
            spawns: spawns
                .into_iter()
                .map(|spawn| (spawn.spawn_id, spawn))
                .collect(),
        }
    }

    pub fn resolve_loaded_grid_area_trigger_like_cpp(
        &self,
        spawn_id: u64,
        map_object_guid: ObjectGuid,
    ) -> Result<AreaTriggerLoadedGridResolvedLikeCpp, AreaTriggerLoadedGridResolveErrorLikeCpp>
    {
        let spawn = self
            .spawns
            .get(&spawn_id)
            .ok_or(AreaTriggerLoadedGridResolveErrorLikeCpp::MissingSpawnData { spawn_id })?;
        let create_properties = self
            .create_properties
            .get(&AreaTriggerIdKeyLikeCpp::from(spawn.create_properties_id))
            .ok_or(
                AreaTriggerLoadedGridResolveErrorLikeCpp::MissingCreateProperties {
                    create_properties_id: spawn.create_properties_id,
                },
            )?;

        validate_map_object_guid_like_cpp(spawn, create_properties, map_object_guid)?;

        let mut area_trigger = AreaTrigger::new();
        area_trigger
            .world_mut()
            .set_map(spawn.map_id, spawn.instance_id)
            .map_err(|source| {
                AreaTriggerLoadedGridResolveErrorLikeCpp::MapBinding(format!("{source:?}"))
            })?;
        area_trigger.world_mut().relocate(spawn.position);
        area_trigger.relocate_stationary_position(spawn.position);
        area_trigger
            .world_mut()
            .object_mut()
            .create(map_object_guid);
        if let Some(template_id) = create_properties.template_id {
            area_trigger
                .world_mut()
                .object_mut()
                .set_entry(template_id.id);
            area_trigger.set_template(template_id, create_properties.template_flags);
        }
        area_trigger.world_mut().object_mut().set_scale(1.0);
        area_trigger.set_spawn_id(spawn.spawn_id);
        area_trigger.set_create_properties_id(create_properties.id);
        area_trigger.set_duration(-1);
        area_trigger.set_shape_type(create_properties.shape_type_like_cpp());
        if let Some(spell_for_visuals) = spawn.spell_for_visuals {
            area_trigger.set_spell_for_visuals(spell_for_visuals);
        }
        area_trigger.set_spell_visual_id(spawn.spell_visual_id);
        area_trigger.set_bounds_radius_2d(create_properties.bounds_radius_2d_like_cpp());
        area_trigger.set_decal_properties_id(create_properties.decal_properties_id_like_cpp());
        area_trigger.set_visual_anim(VisualAnimValues {
            field_c: create_properties.visual_anim_field_c_like_cpp(),
            animation_data_id: create_properties.anim_id as u32,
            anim_kit_id: create_properties.anim_kit_id as u32,
            anim_progress: 0,
        });
        area_trigger.ai_initialize();

        let map_insertion_requested = spawn.add_to_map;
        let map_object_record = if map_insertion_requested {
            Some(
                MapObjectRecord::new_area_trigger(area_trigger.clone()).map_err(|error| {
                    AreaTriggerLoadedGridResolveErrorLikeCpp::MapObjectRecord(format!("{error:?}"))
                })?,
            )
        } else {
            None
        };

        Ok(AreaTriggerLoadedGridResolvedLikeCpp {
            area_trigger,
            map_object_record,
            map_insertion_requested,
            db_phase_shift_requested: spawn.phase_use_flags != 0
                || spawn.phase_id != 0
                || spawn.phase_group != 0,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AreaTriggerIdKeyLikeCpp {
    id: u32,
    is_custom: bool,
}

impl From<AreaTriggerId> for AreaTriggerIdKeyLikeCpp {
    fn from(id: AreaTriggerId) -> Self {
        Self {
            id: id.id,
            is_custom: id.is_custom,
        }
    }
}

fn area_trigger_id_from_data_like_cpp(id: AreaTriggerIdLikeCpp) -> AreaTriggerId {
    AreaTriggerId {
        id: id.id,
        is_custom: id.is_custom,
    }
}

pub fn area_trigger_shape_max_search_radius_like_cpp(shape: &AreaTriggerShapeInfoLikeCpp) -> f32 {
    match area_trigger_shape_type_like_cpp(shape.shape_type) {
        AreaTriggerShapeType::Sphere => shape.data[0].max(shape.data[1]),
        AreaTriggerShapeType::Box => {
            let current = shape.data[0] * shape.data[0] + shape.data[1] * shape.data[1];
            let target = shape.data[3] * shape.data[3] + shape.data[4] * shape.data[4];
            current.max(target).sqrt()
        }
        AreaTriggerShapeType::Polygon => shape
            .polygon_vertices
            .iter()
            .chain(shape.polygon_vertices_target.iter())
            .map(|vertex| (vertex.x * vertex.x + vertex.y * vertex.y).sqrt())
            .fold(0.0, f32::max),
        AreaTriggerShapeType::Cylinder => shape.data[0].max(shape.data[3]),
        AreaTriggerShapeType::Disk => shape.data[1].max(shape.data[3]),
        AreaTriggerShapeType::BoundedPlane => {
            let current = shape.data[0] * shape.data[0] / 4.0 + shape.data[1] * shape.data[1] / 4.0;
            let target = shape.data[3] * shape.data[3] / 4.0 + shape.data[4] * shape.data[4] / 4.0;
            current.max(target).sqrt()
        }
        AreaTriggerShapeType::Unknown => 0.0,
    }
}

fn area_trigger_shape_type_like_cpp(shape_type: u8) -> AreaTriggerShapeType {
    match shape_type {
        0 => AreaTriggerShapeType::Sphere,
        1 => AreaTriggerShapeType::Box,
        3 => AreaTriggerShapeType::Polygon,
        4 => AreaTriggerShapeType::Cylinder,
        5 => AreaTriggerShapeType::Disk,
        6 => AreaTriggerShapeType::BoundedPlane,
        _ => AreaTriggerShapeType::Unknown,
    }
}

fn validate_map_object_guid_like_cpp(
    spawn: &ResolvedAreaTriggerSpawnLikeCpp,
    create_properties: &ResolvedAreaTriggerCreatePropertiesLikeCpp,
    guid: ObjectGuid,
) -> Result<(), AreaTriggerLoadedGridResolveErrorLikeCpp> {
    let expected_entry = create_properties.guid_entry_like_cpp();
    if guid.high_type() != HighGuid::AreaTrigger
        || u32::from(guid.map_id()) != spawn.map_id
        || guid.entry() != expected_entry
    {
        return Err(
            AreaTriggerLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                guid,
                expected_high: HighGuid::AreaTrigger,
                expected_map_id: spawn.map_id,
                expected_entry,
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_data::{
        AreaTriggerCreatePropertiesLikeCpp, AreaTriggerIdLikeCpp, AreaTriggerPosition2LikeCpp,
        AreaTriggerShapeInfoLikeCpp, AreaTriggerTemplateLikeCpp, ScriptIdLikeCpp,
    };
    use wow_map::{SpawnGroupTemplateData, SpawnPosition};

    fn area_trigger_id(id: u32) -> AreaTriggerId {
        AreaTriggerId {
            id,
            is_custom: false,
        }
    }

    fn sphere_shape(radius: f32, radius_target: f32) -> AreaTriggerShapeInfoLikeCpp {
        let mut data = [0.0; wow_data::area_trigger_template::MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP];
        data[0] = radius;
        data[1] = radius_target;
        AreaTriggerShapeInfoLikeCpp {
            shape_type: 0,
            data,
            polygon_vertices: Vec::new(),
            polygon_vertices_target: Vec::new(),
        }
    }

    fn create_properties() -> ResolvedAreaTriggerCreatePropertiesLikeCpp {
        ResolvedAreaTriggerCreatePropertiesLikeCpp {
            id: area_trigger_id(2001),
            template_id: Some(area_trigger_id(9001)),
            template_flags: AREA_TRIGGER_FLAG_IS_SERVER_SIDE,
            flags: AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK3_LIKE_CPP,
            anim_id: 11,
            anim_kit_id: 22,
            decal_properties_id: 77,
            shape: sphere_shape(6.0, 9.0),
        }
    }

    fn spawn(add_to_map: bool) -> ResolvedAreaTriggerSpawnLikeCpp {
        ResolvedAreaTriggerSpawnLikeCpp {
            spawn_id: 12345,
            create_properties_id: area_trigger_id(2001),
            map_id: 571,
            instance_id: 7,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            spell_for_visuals: Some(6789),
            spell_visual_id: 4321,
            add_to_map,
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
        }
    }

    fn area_trigger_guid(entry: u32) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::AreaTrigger, 0, 1, 571, 1, entry, 99)
    }

    fn data_area_trigger_id(id: u32) -> AreaTriggerIdLikeCpp {
        AreaTriggerIdLikeCpp {
            id,
            is_custom: false,
        }
    }

    fn spawn_data(object_type: SpawnObjectType) -> SpawnData {
        SpawnData {
            object_type,
            spawn_id: 12345,
            map_id: 571,
            db_data: true,
            spawn_group: SpawnGroupTemplateData::legacy_group(),
            id: 2001,
            spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 4.0),
            phase_use_flags: 2,
            phase_id: 33,
            phase_group: 44,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 0,
            spawn_difficulties: Vec::new(),
            script_id: 0,
            string_id: String::new(),
        }
    }

    fn runtime_row() -> AreaTriggerSpawnRuntimeRowLikeCpp {
        AreaTriggerSpawnRuntimeRowLikeCpp {
            spawn_id: 12345,
            create_properties_id: data_area_trigger_id(2001),
            spell_for_visuals: Some(6789),
        }
    }

    fn data_create_properties(
        template_id: Option<AreaTriggerIdLikeCpp>,
    ) -> AreaTriggerCreatePropertiesLikeCpp {
        AreaTriggerCreatePropertiesLikeCpp {
            id: data_area_trigger_id(2001),
            template_id,
            flags: AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK3_LIKE_CPP,
            move_curve_id: 0,
            scale_curve_id: 0,
            morph_curve_id: 0,
            facing_curve_id: 0,
            anim_id: 11,
            anim_kit_id: 22,
            decal_properties_id: 77,
            time_to_target: 0,
            time_to_target_scale: 0,
            shape: sphere_shape(6.0, 9.0),
            spline_points: Vec::new(),
            orbit_info: None,
            script_id: ScriptIdLikeCpp(0),
            script_name: String::new(),
        }
    }

    fn data_template() -> AreaTriggerTemplateLikeCpp {
        AreaTriggerTemplateLikeCpp {
            id: data_area_trigger_id(9001),
            flags: AREA_TRIGGER_FLAG_IS_SERVER_SIDE,
            actions: Vec::new(),
        }
    }

    #[test]
    fn loaded_grid_area_trigger_create_matches_static_load_from_db_like_cpp() {
        let resolver = AreaTriggerLoadedGridLifecycleResolverLikeCpp::new(
            [create_properties()],
            [spawn(true)],
        );
        let resolved = resolver
            .resolve_loaded_grid_area_trigger_like_cpp(12345, area_trigger_guid(9001))
            .unwrap();
        let area_trigger = resolved.area_trigger;

        assert!(resolved.map_insertion_requested);
        assert!(resolved.map_object_record.is_some());
        assert!(!resolved.db_phase_shift_requested);
        assert_eq!(area_trigger.world().guid(), area_trigger_guid(9001));
        assert_eq!(area_trigger.world().map_id(), 571);
        assert_eq!(area_trigger.world().instance_id(), 7);
        assert_eq!(
            area_trigger.world().position(),
            Position::new(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(
            area_trigger.stationary_position(),
            Position::new(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(area_trigger.spawn_id(), 12345);
        assert!(area_trigger.is_static_spawn());
        assert_eq!(
            area_trigger.create_properties_id(),
            Some(area_trigger_id(2001))
        );
        assert_eq!(area_trigger.template_id(), Some(area_trigger_id(9001)));
        assert_eq!(
            area_trigger.template_flags(),
            AREA_TRIGGER_FLAG_IS_SERVER_SIDE
        );
        assert_eq!(area_trigger.duration_ms(), -1);
        assert_eq!(area_trigger.total_duration_ms(), -1);
        assert_eq!(area_trigger.data().duration, 0);
        assert_eq!(area_trigger.shape_type(), AreaTriggerShapeType::Sphere);
        assert_eq!(area_trigger.spell_id(), 0);
        assert_eq!(area_trigger.data().spell_for_visuals, 6789);
        assert_eq!(area_trigger.data().spell_visual_id, 4321);
        assert_eq!(area_trigger.data().bounds_radius_2d, 9.0);
        assert_eq!(area_trigger.data().decal_properties_id, 24);
        assert_eq!(
            area_trigger.data().visual_anim,
            VisualAnimValues {
                field_c: true,
                animation_data_id: 11,
                anim_kit_id: 22,
                anim_progress: 0,
            }
        );
        assert!(area_trigger.is_ai_initialized());
    }

    #[test]
    fn loaded_grid_area_trigger_inputs_from_spawn_data_match_cpp_load_from_db_metadata() {
        let (properties, spawn) = resolve_area_trigger_loaded_grid_inputs_from_spawn_data_like_cpp(
            &spawn_data(SpawnObjectType::AreaTrigger),
            &runtime_row(),
            &data_create_properties(Some(data_area_trigger_id(9001))),
            Some(&data_template()),
            7,
            true,
            4321,
        )
        .unwrap();

        assert_eq!(properties.id, area_trigger_id(2001));
        assert_eq!(properties.template_id, Some(area_trigger_id(9001)));
        assert_eq!(properties.template_flags, AREA_TRIGGER_FLAG_IS_SERVER_SIDE);
        assert_eq!(properties.guid_entry_like_cpp(), 9001);
        assert_eq!(spawn.spawn_id, 12345);
        assert_eq!(spawn.create_properties_id, area_trigger_id(2001));
        assert_eq!(spawn.map_id, 571);
        assert_eq!(spawn.instance_id, 7);
        assert_eq!(spawn.position, Position::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(spawn.spell_for_visuals, Some(6789));
        assert_eq!(spawn.spell_visual_id, 4321);
        assert!(spawn.add_to_map);
        assert_eq!(spawn.phase_use_flags, 2);
        assert_eq!(spawn.phase_id, 33);
        assert_eq!(spawn.phase_group, 44);

        let resolver = AreaTriggerLoadedGridLifecycleResolverLikeCpp::new([properties], [spawn]);
        let resolved = resolver
            .resolve_loaded_grid_area_trigger_like_cpp(12345, area_trigger_guid(9001))
            .unwrap();
        assert_eq!(
            resolved.area_trigger.template_id(),
            Some(area_trigger_id(9001))
        );
        assert_eq!(
            resolved.area_trigger.create_properties_id(),
            Some(area_trigger_id(2001))
        );
        assert!(resolved.db_phase_shift_requested);
    }

    #[test]
    fn loaded_grid_area_trigger_inputs_without_template_use_zero_guid_entry_like_cpp() {
        let (properties, spawn) = resolve_area_trigger_loaded_grid_inputs_from_spawn_data_like_cpp(
            &spawn_data(SpawnObjectType::AreaTrigger),
            &runtime_row(),
            &data_create_properties(None),
            None,
            0,
            false,
            0,
        )
        .unwrap();

        assert_eq!(properties.template_id, None);
        assert_eq!(properties.template_flags, 0);
        assert_eq!(properties.guid_entry_like_cpp(), 0);
        assert!(!spawn.add_to_map);
    }

    #[test]
    fn loaded_grid_area_trigger_inputs_reject_non_area_trigger_spawn_like_cpp() {
        assert_eq!(
            resolve_area_trigger_loaded_grid_inputs_from_spawn_data_like_cpp(
                &spawn_data(SpawnObjectType::Creature),
                &runtime_row(),
                &data_create_properties(Some(data_area_trigger_id(9001))),
                Some(&data_template()),
                0,
                true,
                0,
            )
            .unwrap_err(),
            AreaTriggerLoadedGridResolveErrorLikeCpp::WrongSpawnObjectType {
                spawn_id: 12345,
                object_type: SpawnObjectType::Creature,
            }
        );
    }

    #[test]
    fn loaded_grid_area_trigger_inputs_reject_mismatched_template_like_cpp() {
        assert_eq!(
            resolve_area_trigger_loaded_grid_inputs_from_spawn_data_like_cpp(
                &spawn_data(SpawnObjectType::AreaTrigger),
                &runtime_row(),
                &data_create_properties(Some(data_area_trigger_id(9001))),
                None,
                0,
                true,
                0,
            )
            .unwrap_err(),
            AreaTriggerLoadedGridResolveErrorLikeCpp::MismatchedTemplateId {
                create_properties_id: data_area_trigger_id(2001),
                expected_template_id: Some(data_area_trigger_id(9001)),
                template_id: None,
            }
        );
    }

    #[test]
    fn loaded_grid_area_trigger_inputs_reject_mismatched_create_properties_like_cpp() {
        let mut row = runtime_row();
        row.create_properties_id = data_area_trigger_id(2222);

        assert_eq!(
            resolve_area_trigger_loaded_grid_inputs_from_spawn_data_like_cpp(
                &spawn_data(SpawnObjectType::AreaTrigger),
                &row,
                &data_create_properties(Some(data_area_trigger_id(9001))),
                Some(&data_template()),
                0,
                true,
                0,
            )
            .unwrap_err(),
            AreaTriggerLoadedGridResolveErrorLikeCpp::MismatchedCreatePropertiesId {
                spawn_id: 12345,
                spawn_create_properties_id: 2001,
                runtime_create_properties_id: data_area_trigger_id(2222),
                create_properties_id: data_area_trigger_id(2001),
            }
        );
    }

    #[test]
    fn loaded_grid_area_trigger_record_builder_uses_map_owned_low_guid_like_cpp() {
        let mut map = Map::new(571, 7, 0, 60_000);
        assert_eq!(
            map.generate_low_guid_like_cpp(HighGuid::AreaTrigger)
                .unwrap(),
            1
        );
        let records = build_loaded_grid_area_trigger_record_from_spawn_data_like_cpp(
            &mut map,
            &spawn_data(SpawnObjectType::AreaTrigger),
            &runtime_row(),
            &data_create_properties(Some(data_area_trigger_id(9001))),
            Some(&data_template()),
            4321,
        )
        .unwrap();

        assert!(records.pre_add_records.is_empty());
        let area_trigger = records.primary_record.area_trigger().unwrap();
        assert_eq!(area_trigger.world().guid().counter(), 2);
        assert_eq!(area_trigger.world().guid().entry(), 9001);
        assert_eq!(
            area_trigger.world().guid().high_type(),
            HighGuid::AreaTrigger
        );
        assert_eq!(u32::from(area_trigger.world().guid().map_id()), 571);
        assert_eq!(area_trigger.spawn_id(), 12345);
        assert_eq!(area_trigger.world().map_id(), 571);
        assert_eq!(area_trigger.world().instance_id(), 7);
    }

    #[test]
    fn loaded_grid_area_trigger_record_builder_rejects_wrong_map_like_cpp() {
        let mut map = Map::new(1, 7, 0, 60_000);
        let err = build_loaded_grid_area_trigger_record_from_spawn_data_like_cpp(
            &mut map,
            &spawn_data(SpawnObjectType::AreaTrigger),
            &runtime_row(),
            &data_create_properties(Some(data_area_trigger_id(9001))),
            Some(&data_template()),
            4321,
        )
        .unwrap_err();

        assert_eq!(
            err,
            AreaTriggerLoadedGridResolveErrorLikeCpp::MapMismatch {
                spawn_id: 12345,
                spawn_map_id: 571,
                spawn_instance_id: 7,
                map_id: 1,
                instance_id: 7,
            }
        );
    }

    #[test]
    fn loaded_grid_area_trigger_guid_entry_uses_template_id_not_create_properties_id_like_cpp() {
        let resolver = AreaTriggerLoadedGridLifecycleResolverLikeCpp::new(
            [create_properties()],
            [spawn(true)],
        );
        let err = resolver
            .resolve_loaded_grid_area_trigger_like_cpp(12345, area_trigger_guid(2001))
            .unwrap_err();
        assert_eq!(
            err,
            AreaTriggerLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                guid: area_trigger_guid(2001),
                expected_high: HighGuid::AreaTrigger,
                expected_map_id: 571,
                expected_entry: 9001,
            }
        );
    }

    #[test]
    fn loaded_grid_area_trigger_without_template_uses_zero_guid_entry_like_cpp() {
        let mut properties = create_properties();
        properties.template_id = None;
        properties.template_flags = 0;
        let resolver =
            AreaTriggerLoadedGridLifecycleResolverLikeCpp::new([properties], [spawn(false)]);
        let resolved = resolver
            .resolve_loaded_grid_area_trigger_like_cpp(12345, area_trigger_guid(0))
            .unwrap();

        assert!(!resolved.map_insertion_requested);
        assert!(resolved.map_object_record.is_none());
        assert_eq!(resolved.area_trigger.template_id(), None);
        assert_eq!(resolved.area_trigger.template_flags(), 0);
        assert_eq!(resolved.area_trigger.world().object().entry(), 0);
        assert_eq!(resolved.area_trigger.data().decal_properties_id, 77);
    }

    #[test]
    fn loaded_grid_area_trigger_reports_missing_spawn_and_create_properties_like_cpp() {
        let resolver =
            AreaTriggerLoadedGridLifecycleResolverLikeCpp::new([create_properties()], []);
        assert_eq!(
            resolver
                .resolve_loaded_grid_area_trigger_like_cpp(12345, area_trigger_guid(9001))
                .unwrap_err(),
            AreaTriggerLoadedGridResolveErrorLikeCpp::MissingSpawnData { spawn_id: 12345 }
        );

        let resolver = AreaTriggerLoadedGridLifecycleResolverLikeCpp::new([], [spawn(true)]);
        assert_eq!(
            resolver
                .resolve_loaded_grid_area_trigger_like_cpp(12345, area_trigger_guid(9001))
                .unwrap_err(),
            AreaTriggerLoadedGridResolveErrorLikeCpp::MissingCreateProperties {
                create_properties_id: area_trigger_id(2001),
            }
        );
    }

    #[test]
    fn area_trigger_shape_max_search_radius_matches_cpp_shapes() {
        assert_eq!(
            area_trigger_shape_max_search_radius_like_cpp(&sphere_shape(3.0, 5.0)),
            5.0
        );

        let mut box_data =
            [0.0; wow_data::area_trigger_template::MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP];
        box_data[0] = 3.0;
        box_data[1] = 4.0;
        box_data[3] = 6.0;
        box_data[4] = 8.0;
        assert_eq!(
            area_trigger_shape_max_search_radius_like_cpp(&AreaTriggerShapeInfoLikeCpp {
                shape_type: 1,
                data: box_data,
                polygon_vertices: Vec::new(),
                polygon_vertices_target: Vec::new(),
            }),
            10.0
        );

        let mut disk_data =
            [0.0; wow_data::area_trigger_template::MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP];
        disk_data[1] = 12.0;
        disk_data[3] = 7.0;
        assert_eq!(
            area_trigger_shape_max_search_radius_like_cpp(&AreaTriggerShapeInfoLikeCpp {
                shape_type: 5,
                data: disk_data,
                polygon_vertices: Vec::new(),
                polygon_vertices_target: Vec::new(),
            }),
            12.0
        );

        assert_eq!(
            area_trigger_shape_max_search_radius_like_cpp(&AreaTriggerShapeInfoLikeCpp {
                shape_type: 3,
                data: [0.0; wow_data::area_trigger_template::MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP],
                polygon_vertices: vec![AreaTriggerPosition2LikeCpp { x: 3.0, y: 4.0 }],
                polygon_vertices_target: vec![AreaTriggerPosition2LikeCpp { x: 5.0, y: 12.0 }],
            }),
            13.0
        );
    }
}
