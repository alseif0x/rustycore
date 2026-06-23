// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure loaded-grid GameObject lifecycle resolver for DB-backed `LoadFromDB` input.
//!
//! C++ anchors:
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:2492-2736`
//!   `ObjectMgr::LoadGameObjects`: DB spawn fields, event-managed rows and grid insertion source.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:7552-7610`
//!   `ObjectMgr::LoadGameObjectTemplate`: template fields.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:7770-7854`
//!   `LoadGameObjectTemplateAddons`: addon fields.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp:951-1185`
//!   `GameObject::Create`: caller-owned GUID, map binding, template/addon/override intrinsic state,
//!   and linked-trap create/AddToMap side effect for DB-backed spawns.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp:1187-1200`
//!   `GameObject::CreateGameObject`: linked trap template lookup plus dynamic spawn_id=0 create.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp:1911-1978`
//!   `GameObject::LoadFromDB`: spawn id, compatibility, spawntimesecs and caller-owned AddToMap.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObjectData.h:843-851,998-1010`
//!   `IsDespawnAtAction` and `GetDespawnPossibility` represented helpers.

use std::collections::BTreeMap;

use crate::spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::{
    GameObjectOverrideLifecycleRecordLikeCpp, GameObjectOverrideLifecycleStoreLikeCpp,
    GameObjectTemplateLifecycleRecordLikeCpp, GameObjectTemplateLifecycleStoreLikeCpp,
};
use wow_entities::{
    GAMEOBJECT_TYPE_BUTTON, GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_FLAGDROP,
    GAMEOBJECT_TYPE_FLAGSTAND, GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT,
    GAMEOBJECT_TYPE_QUESTGIVER, GAMEOBJECT_TYPE_TRANSPORT, GameObject,
    GameObjectCreateLifecycleRecord, GameObjectLifecycleError, GameObjectLoadFromDbLifecycleRecord,
    GameObjectTemplateData, GameObjectTemplateLifecycleRecord, GoState, MAX_GAMEOBJECT_DATA,
    MapObjectRecord,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGameObjectTemplateLikeCpp {
    pub entry: u32,
    pub go_type: u32,
    pub display_id: u32,
    pub name: String,
    pub scale: f32,
    pub faction: u32,
    pub flags: u32,
    pub data: [u32; MAX_GAMEOBJECT_DATA],
    pub world_effect_id: u32,
    pub anim_kit_id: u16,
    pub level: u32,
    pub percent_health: u8,
    pub custom_param: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGameObjectSpawnLikeCpp {
    pub spawn_id: u64,
    pub entry: u32,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub rotation: [f32; 4],
    pub anim_progress: u8,
    pub go_state: GoState,
    pub spawntimesecs: i32,
    pub effective_map_respawn_time: i64,
    pub add_to_map: bool,
    pub respawn_compatibility_mode: bool,
    pub string_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectLoadedGridResolvedLikeCpp {
    pub lifecycle_record: GameObjectLoadFromDbLifecycleRecord,
    pub game_object: GameObject,
    pub pre_add_records: Vec<MapObjectRecord>,
    pub map_object_record: Option<MapObjectRecord>,
    pub map_insertion_requested: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameObjectLoadedGridResolveErrorLikeCpp {
    MissingSpawnData {
        spawn_id: u64,
    },
    MissingTemplate {
        entry: u32,
    },
    UnsupportedMapObjectTransport {
        entry: u32,
    },
    UnsupportedTransportStopFrame {
        spawn_id: u64,
        state: u8,
    },
    InvalidGoState {
        spawn_id: u64,
        state: u8,
    },
    RotationComponentOutOfRange {
        spawn_id: u64,
        component: usize,
        value: f32,
    },
    InvalidMapObjectGuid {
        guid: ObjectGuid,
        expected_high: HighGuid,
        expected_map_id: u32,
        expected_entry: u32,
    },
    Lifecycle(GameObjectLifecycleError),
    MapObjectRecord(String),
}

#[derive(Debug, Clone, Default)]
pub struct GameObjectLoadedGridLifecycleResolverLikeCpp {
    templates: BTreeMap<u32, ResolvedGameObjectTemplateLikeCpp>,
    spawns: BTreeMap<u64, ResolvedGameObjectSpawnLikeCpp>,
}

impl GameObjectLoadedGridLifecycleResolverLikeCpp {
    pub fn new(
        templates: impl IntoIterator<Item = ResolvedGameObjectTemplateLikeCpp>,
        spawns: impl IntoIterator<Item = ResolvedGameObjectSpawnLikeCpp>,
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
        }
    }

    pub fn resolve_loaded_grid_gameobject_like_cpp(
        &self,
        spawn_id: u64,
        map_object_guid: ObjectGuid,
    ) -> Result<GameObjectLoadedGridResolvedLikeCpp, GameObjectLoadedGridResolveErrorLikeCpp> {
        self.resolve_loaded_grid_gameobject_with_linked_trap_like_cpp(
            spawn_id,
            map_object_guid,
            None,
        )
    }

    pub fn resolve_loaded_grid_gameobject_with_linked_trap_like_cpp(
        &self,
        spawn_id: u64,
        map_object_guid: ObjectGuid,
        linked_trap_guid: Option<ObjectGuid>,
    ) -> Result<GameObjectLoadedGridResolvedLikeCpp, GameObjectLoadedGridResolveErrorLikeCpp> {
        let spawn = self
            .spawns
            .get(&spawn_id)
            .ok_or(GameObjectLoadedGridResolveErrorLikeCpp::MissingSpawnData { spawn_id })?;
        let template = self.templates.get(&spawn.entry).ok_or(
            GameObjectLoadedGridResolveErrorLikeCpp::MissingTemplate { entry: spawn.entry },
        )?;
        validate_map_object_guid_like_cpp(spawn, template, map_object_guid)?;
        let template_data = GameObjectTemplateData::new(template.go_type, template.data);
        let lifecycle_record = GameObjectLoadFromDbLifecycleRecord {
            create: GameObjectCreateLifecycleRecord {
                guid: map_object_guid,
                map_id: spawn.map_id,
                instance_id: spawn.instance_id,
                position: spawn.position,
                rotation: spawn.rotation,
                anim_progress: spawn.anim_progress,
                go_state: spawn.go_state,
                art_kit: 0,
                dynamic: !spawn.respawn_compatibility_mode,
                spawn_id: spawn.spawn_id,
                template: template_lifecycle_record(template),
            },
            spawntimesecs: spawn.spawntimesecs,
            effective_map_respawn_time: spawn.effective_map_respawn_time,
            despawn_possible: get_despawn_possibility_like_cpp(&template_data),
            despawn_at_action: template_data.is_despawn_at_action_like_cpp(),
            respawn_compatibility_mode: spawn.respawn_compatibility_mode,
            string_id: spawn.string_id.clone(),
        };
        let mut game_object = GameObject::try_load_from_db_lifecycle(lifecycle_record.clone())
            .map_err(GameObjectLoadedGridResolveErrorLikeCpp::Lifecycle)?;
        let mut pre_add_records = Vec::new();
        if let Some(trap_guid) = linked_trap_guid {
            if let Some(linked_trap_record) =
                self.build_linked_trap_record_like_cpp(spawn, &template_data, trap_guid)?
            {
                game_object.set_linked_trap_like_cpp(trap_guid);
                pre_add_records.push(linked_trap_record);
            }
        }
        let map_insertion_requested = spawn.add_to_map;
        let map_object_record = if map_insertion_requested {
            Some(
                MapObjectRecord::new_game_object(game_object.clone()).map_err(|error| {
                    GameObjectLoadedGridResolveErrorLikeCpp::MapObjectRecord(format!("{error:?}"))
                })?,
            )
        } else {
            None
        };
        Ok(GameObjectLoadedGridResolvedLikeCpp {
            lifecycle_record,
            game_object,
            pre_add_records,
            map_object_record,
            map_insertion_requested,
        })
    }

    fn build_linked_trap_record_like_cpp(
        &self,
        owner_spawn: &ResolvedGameObjectSpawnLikeCpp,
        owner_template_data: &GameObjectTemplateData,
        trap_guid: ObjectGuid,
    ) -> Result<Option<MapObjectRecord>, GameObjectLoadedGridResolveErrorLikeCpp> {
        let linked_entry = owner_template_data.get_linked_gameobject_entry_like_cpp();
        if linked_entry == 0 {
            return Ok(None);
        }
        let Some(trap_template) = self.templates.get(&linked_entry) else {
            return Ok(None);
        };
        let trap_spawn = ResolvedGameObjectSpawnLikeCpp {
            spawn_id: 0,
            entry: linked_entry,
            map_id: owner_spawn.map_id,
            instance_id: owner_spawn.instance_id,
            position: owner_spawn.position,
            rotation: owner_spawn.rotation,
            anim_progress: 255,
            go_state: GoState::Ready,
            spawntimesecs: 0,
            effective_map_respawn_time: 0,
            add_to_map: true,
            respawn_compatibility_mode: true,
            string_id: String::new(),
        };
        if validate_map_object_guid_like_cpp(&trap_spawn, trap_template, trap_guid).is_err() {
            return Ok(None);
        }
        let Ok(trap) = GameObject::try_create_from_lifecycle(GameObjectCreateLifecycleRecord {
            guid: trap_guid,
            map_id: owner_spawn.map_id,
            instance_id: owner_spawn.instance_id,
            position: owner_spawn.position,
            rotation: owner_spawn.rotation,
            anim_progress: 255,
            go_state: GoState::Ready,
            art_kit: 0,
            dynamic: false,
            spawn_id: 0,
            template: template_lifecycle_record(trap_template),
        }) else {
            return Ok(None);
        };
        Ok(MapObjectRecord::new_game_object(trap).ok())
    }
}

pub fn resolved_template_from_lifecycle_record_like_cpp(
    template: &GameObjectTemplateLifecycleRecordLikeCpp,
    override_record: Option<&GameObjectOverrideLifecycleRecordLikeCpp>,
) -> Result<ResolvedGameObjectTemplateLikeCpp, GameObjectLoadedGridResolveErrorLikeCpp> {
    if template.go_type == u32::from(GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT) {
        return Err(
            GameObjectLoadedGridResolveErrorLikeCpp::UnsupportedMapObjectTransport {
                entry: template.entry,
            },
        );
    }
    let addon = template.addon;
    let faction = override_record
        .map(|record| record.faction)
        .or_else(|| addon.map(|record| record.faction))
        .unwrap_or(0);
    let flags = override_record
        .map(|record| record.flags)
        .or_else(|| addon.map(|record| record.flags))
        .unwrap_or(0);
    let world_effect_id = addon.map(|record| record.world_effect_id).unwrap_or(0);
    let anim_kit_id = addon.map(|record| record.anim_kit_id).unwrap_or(0);

    Ok(ResolvedGameObjectTemplateLikeCpp {
        entry: template.entry,
        go_type: template.go_type,
        display_id: template.display_id,
        name: template.name.clone(),
        scale: template.size,
        faction,
        flags,
        data: template.data,
        world_effect_id,
        anim_kit_id,
        level: template.content_tuning_id,
        percent_health: 100,
        custom_param: 0,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn build_loaded_grid_gameobject_inputs_from_db_like_cpp(
    spawn: &wow_map::SpawnData,
    runtime_row: &GameObjectSpawnRuntimeRowLikeCpp,
    template_store: &GameObjectTemplateLifecycleStoreLikeCpp,
    override_store: &GameObjectOverrideLifecycleStoreLikeCpp,
    instance_id: u32,
    effective_map_respawn_time: i64,
    add_to_map: bool,
) -> Result<
    (
        ResolvedGameObjectTemplateLikeCpp,
        ResolvedGameObjectSpawnLikeCpp,
    ),
    GameObjectLoadedGridResolveErrorLikeCpp,
> {
    let template = template_store
        .get(spawn.id)
        .ok_or(GameObjectLoadedGridResolveErrorLikeCpp::MissingTemplate { entry: spawn.id })?;
    if template.go_type == u32::from(GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT) {
        return Err(
            GameObjectLoadedGridResolveErrorLikeCpp::UnsupportedMapObjectTransport {
                entry: template.entry,
            },
        );
    }
    let rotation = normalize_rotation_like_cpp(
        spawn.spawn_id,
        runtime_row.rotation,
        spawn.spawn_point.orientation,
    )?;
    let go_state = go_state_from_db_like_cpp(spawn.spawn_id, template.go_type, runtime_row.state)?;
    let addon = template.addon;
    let override_record = override_store.get(spawn.spawn_id);
    let faction = override_record
        .map(|record| record.faction)
        .or_else(|| addon.map(|record| record.faction))
        .unwrap_or(0);
    let flags = override_record
        .map(|record| record.flags)
        .or_else(|| addon.map(|record| record.flags))
        .unwrap_or(0);
    let world_effect_id = addon.map(|record| record.world_effect_id).unwrap_or(0);
    let anim_kit_id = addon.map(|record| record.anim_kit_id).unwrap_or(0);
    let string_id = if !runtime_row.string_id.is_empty() {
        runtime_row.string_id.clone()
    } else if !spawn.string_id.is_empty() {
        spawn.string_id.clone()
    } else {
        template.string_id.clone()
    };

    Ok((
        ResolvedGameObjectTemplateLikeCpp {
            entry: template.entry,
            go_type: template.go_type,
            display_id: template.display_id,
            name: template.name.clone(),
            scale: template.size,
            faction,
            flags,
            data: template.data,
            world_effect_id,
            anim_kit_id,
            level: template.content_tuning_id,
            percent_health: 100,
            custom_param: 0,
        },
        ResolvedGameObjectSpawnLikeCpp {
            spawn_id: spawn.spawn_id,
            entry: spawn.id,
            map_id: spawn.map_id,
            instance_id,
            position: Position {
                x: spawn.spawn_point.x,
                y: spawn.spawn_point.y,
                z: spawn.spawn_point.z,
                orientation: spawn.spawn_point.orientation,
            },
            rotation,
            anim_progress: runtime_row.anim_progress,
            go_state,
            spawntimesecs: runtime_row.spawn_time_secs,
            effective_map_respawn_time,
            add_to_map,
            respawn_compatibility_mode: spawn
                .spawn_group
                .flags
                .contains(wow_map::SpawnGroupFlags::COMPATIBILITY_MODE),
            string_id,
        },
    ))
}

pub fn go_state_from_db_like_cpp(
    spawn_id: u64,
    template_go_type: u32,
    state: u8,
) -> Result<GoState, GameObjectLoadedGridResolveErrorLikeCpp> {
    let is_transport = template_go_type == u32::from(GAMEOBJECT_TYPE_TRANSPORT);
    match state {
        0 => Ok(GoState::Active),
        1 => Ok(GoState::Ready),
        2 => Ok(GoState::Destroyed),
        24 if is_transport => Ok(GoState::TransportActive),
        25 if is_transport => Ok(GoState::TransportStopped),
        24 | 25 => Err(GameObjectLoadedGridResolveErrorLikeCpp::InvalidGoState { spawn_id, state }),
        26..=33 if is_transport => Err(
            GameObjectLoadedGridResolveErrorLikeCpp::UnsupportedTransportStopFrame {
                spawn_id,
                state,
            },
        ),
        _ => Err(GameObjectLoadedGridResolveErrorLikeCpp::InvalidGoState { spawn_id, state }),
    }
}

pub fn normalize_rotation_like_cpp(
    spawn_id: u64,
    rotation: [f32; 4],
    orientation: f32,
) -> Result<[f32; 4], GameObjectLoadedGridResolveErrorLikeCpp> {
    for (component, value) in rotation.iter().copied().enumerate() {
        if !(-1.0..=1.0).contains(&value) {
            return Err(
                GameObjectLoadedGridResolveErrorLikeCpp::RotationComponentOutOfRange {
                    spawn_id,
                    component,
                    value,
                },
            );
        }
    }
    let norm_sq = rotation.iter().map(|value| value * value).sum::<f32>();
    if (norm_sq - 1.0).abs() <= 0.01 {
        return Ok(rotation);
    }
    let half = orientation * 0.5;
    Ok([0.0, 0.0, half.sin(), half.cos()])
}

pub fn get_despawn_possibility_like_cpp(template: &GameObjectTemplateData) -> bool {
    match template.go_type {
        GAMEOBJECT_TYPE_DOOR
        | GAMEOBJECT_TYPE_BUTTON
        | GAMEOBJECT_TYPE_QUESTGIVER
        | GAMEOBJECT_TYPE_GOOBER
        | GAMEOBJECT_TYPE_FLAGSTAND
        | GAMEOBJECT_TYPE_FLAGDROP => template.get_no_damage_immune_like_cpp() != 0,
        _ => true,
    }
}

fn validate_map_object_guid_like_cpp(
    spawn: &ResolvedGameObjectSpawnLikeCpp,
    template: &ResolvedGameObjectTemplateLikeCpp,
    map_object_guid: ObjectGuid,
) -> Result<(), GameObjectLoadedGridResolveErrorLikeCpp> {
    let expected_high = if template.go_type == GAMEOBJECT_TYPE_TRANSPORT {
        HighGuid::Transport
    } else {
        HighGuid::GameObject
    };
    if map_object_guid.high_type() != expected_high
        || (expected_high != HighGuid::Transport
            && u32::from(map_object_guid.map_id()) != spawn.map_id)
        || (expected_high != HighGuid::Transport && map_object_guid.entry() != template.entry)
    {
        return Err(
            GameObjectLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
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
    template: &ResolvedGameObjectTemplateLikeCpp,
) -> GameObjectTemplateLifecycleRecord {
    GameObjectTemplateLifecycleRecord {
        entry: template.entry,
        name: template.name.clone(),
        go_type: template.go_type,
        display_id: template.display_id,
        scale: template.scale,
        faction: template.faction,
        flags: template.flags,
        data: template.data,
        world_effect_id: template.world_effect_id,
        anim_kit_id: template.anim_kit_id,
        level: template.level,
        percent_health: template.percent_health,
        custom_param: template.custom_param,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_data::{
        GameObjectOverrideLifecycleRecordLikeCpp, GameObjectOverrideLifecycleStoreLikeCpp,
        GameObjectTemplateAddonLifecycleRecordLikeCpp, GameObjectTemplateLifecycleRecordLikeCpp,
        GameObjectTemplateLifecycleStoreLikeCpp,
    };
    use wow_entities::{
        GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_TRANSPORT,
        GAMEOBJECT_TYPE_TRAP,
    };
    use wow_map::{SpawnData, SpawnObjectType, SpawnPosition};

    fn template_store(
        addon: Option<GameObjectTemplateAddonLifecycleRecordLikeCpp>,
    ) -> GameObjectTemplateLifecycleStoreLikeCpp {
        template_store_with_type(GAMEOBJECT_TYPE_GOOBER, addon)
    }

    fn template_store_with_type(
        go_type: u32,
        addon: Option<GameObjectTemplateAddonLifecycleRecordLikeCpp>,
    ) -> GameObjectTemplateLifecycleStoreLikeCpp {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[11] = 1;
        GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            GameObjectTemplateLifecycleRecordLikeCpp {
                entry: 9001,
                go_type,
                display_id: 44,
                name: "goober".to_string(),
                size: 1.25,
                data,
                content_tuning_id: 0,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: "template-string".to_string(),
                addon,
            },
        ])
    }

    fn template_record(
        entry: u32,
        go_type: u32,
        data: [u32; MAX_GAMEOBJECT_DATA],
    ) -> GameObjectTemplateLifecycleRecordLikeCpp {
        GameObjectTemplateLifecycleRecordLikeCpp {
            entry,
            go_type,
            display_id: 44,
            name: format!("template-{entry}"),
            size: 1.25,
            data,
            content_tuning_id: 0,
            ai_name: String::new(),
            script_name: String::new(),
            string_id: String::new(),
            addon: None,
        }
    }

    fn spawn(spawn_id: u64) -> SpawnData {
        SpawnData {
            object_type: SpawnObjectType::GameObject,
            spawn_id,
            map_id: 571,
            db_data: true,
            spawn_group: wow_map::SpawnGroupTemplateData::default_group(),
            id: 9001,
            spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 1.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: -1,
            pool_id: 0,
            spawn_time_secs: 30,
            spawn_difficulties: vec![0],
            script_id: 0,
            string_id: String::new(),
        }
    }

    fn runtime(spawn_id: u64) -> GameObjectSpawnRuntimeRowLikeCpp {
        GameObjectSpawnRuntimeRowLikeCpp {
            spawn_id,
            rotation: [0.0, 0.0, 0.0, 1.0],
            anim_progress: 55,
            state: 1,
            string_id: "runtime-string".to_string(),
            spawn_time_secs: 30,
        }
    }

    #[test]
    fn gameobject_builder_maps_db_spawn_template_addon_to_map_record_like_cpp() {
        let addon = GameObjectTemplateAddonLifecycleRecordLikeCpp {
            entry: 9001,
            faction: 35,
            flags: 7,
            world_effect_id: 9,
            anim_kit_id: 11,
        };
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::default();
        let (template, resolved_spawn) = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
            &spawn(88),
            &runtime(88),
            &template_store(Some(addon)),
            &overrides,
            1,
            0,
            true,
        )
        .unwrap();
        let guid = ObjectGuid::create_gameobject_like_cpp(571, 9001, 22);
        let resolved =
            GameObjectLoadedGridLifecycleResolverLikeCpp::new([template], [resolved_spawn])
                .resolve_loaded_grid_gameobject_like_cpp(88, guid)
                .unwrap();
        assert!(resolved.map_object_record.is_some());
        assert_eq!(resolved.lifecycle_record.create.template.faction, 35);
        assert_eq!(resolved.lifecycle_record.create.template.flags, 7);
        assert_eq!(resolved.lifecycle_record.create.template.world_effect_id, 9);
        assert_eq!(resolved.lifecycle_record.string_id, "runtime-string");
    }

    #[test]
    fn gameobject_linked_trap_pre_add_record_matches_cpp_dynamic_spawn() {
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::default();
        let mut owner_data = [0; MAX_GAMEOBJECT_DATA];
        owner_data[12] = 9002;
        let trap_data = [0; MAX_GAMEOBJECT_DATA];
        let templates = GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            template_record(9001, GAMEOBJECT_TYPE_GOOBER, owner_data),
            template_record(9002, GAMEOBJECT_TYPE_TRAP, trap_data),
        ]);
        let (template, resolved_spawn) = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
            &spawn(88),
            &runtime(88),
            &templates,
            &overrides,
            1,
            0,
            true,
        )
        .unwrap();
        let owner_guid = ObjectGuid::create_gameobject_like_cpp(571, 9001, 22);
        let trap_guid = ObjectGuid::create_gameobject_like_cpp(571, 9002, 23);

        let resolved = GameObjectLoadedGridLifecycleResolverLikeCpp::new(
            [
                template,
                resolved_template_from_lifecycle_record_like_cpp(
                    templates.get(9002).unwrap(),
                    None,
                )
                .unwrap(),
            ],
            [resolved_spawn],
        )
        .resolve_loaded_grid_gameobject_with_linked_trap_like_cpp(88, owner_guid, Some(trap_guid))
        .unwrap();

        assert_eq!(resolved.pre_add_records.len(), 1);
        assert_eq!(resolved.game_object.linked_trap_guid_like_cpp(), trap_guid);
        let trap = resolved.pre_add_records[0]
            .game_object()
            .expect("linked trap pre-add record is a GameObject");
        assert_eq!(trap.spawn_id(), 0);
        assert_eq!(trap.world().guid(), trap_guid);
        assert_eq!(trap.world().map_id(), 571);
        assert_eq!(
            trap.world().instance_id(),
            resolved.game_object.world().instance_id()
        );
        assert_eq!(
            trap.world().position(),
            resolved.game_object.world().position()
        );
        assert_eq!(trap.prev_go_state(), GoState::Ready);
        assert_eq!(trap.respawn_compatibility_mode(), true);
    }

    #[test]
    fn gameobject_linked_trap_missing_template_does_not_block_owner_like_cpp() {
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::default();
        let mut owner_data = [0; MAX_GAMEOBJECT_DATA];
        owner_data[12] = 9002;
        let templates = GameObjectTemplateLifecycleStoreLikeCpp::from_templates([template_record(
            9001,
            GAMEOBJECT_TYPE_GOOBER,
            owner_data,
        )]);
        let (template, resolved_spawn) = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
            &spawn(88),
            &runtime(88),
            &templates,
            &overrides,
            1,
            0,
            true,
        )
        .unwrap();
        let owner_guid = ObjectGuid::create_gameobject_like_cpp(571, 9001, 22);
        let trap_guid = ObjectGuid::create_gameobject_like_cpp(571, 9002, 23);

        let resolved =
            GameObjectLoadedGridLifecycleResolverLikeCpp::new([template], [resolved_spawn])
                .resolve_loaded_grid_gameobject_with_linked_trap_like_cpp(
                    88,
                    owner_guid,
                    Some(trap_guid),
                )
                .unwrap();

        assert!(resolved.map_object_record.is_some());
        assert!(resolved.pre_add_records.is_empty());
        assert_eq!(
            resolved.game_object.linked_trap_guid_like_cpp(),
            ObjectGuid::EMPTY
        );
    }

    #[test]
    fn gameobject_linked_trap_rejected_guid_does_not_block_owner_like_cpp() {
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::default();
        let mut owner_data = [0; MAX_GAMEOBJECT_DATA];
        owner_data[12] = 9002;
        let trap_data = [0; MAX_GAMEOBJECT_DATA];
        let templates = GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            template_record(9001, GAMEOBJECT_TYPE_GOOBER, owner_data),
            template_record(9002, GAMEOBJECT_TYPE_TRANSPORT, trap_data),
        ]);
        let (template, resolved_spawn) = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
            &spawn(88),
            &runtime(88),
            &templates,
            &overrides,
            1,
            0,
            true,
        )
        .unwrap();
        let owner_guid = ObjectGuid::create_gameobject_like_cpp(571, 9001, 22);
        let mismatched_trap_guid = ObjectGuid::create_gameobject_like_cpp(571, 9002, 23);

        let resolved = GameObjectLoadedGridLifecycleResolverLikeCpp::new(
            [
                template,
                resolved_template_from_lifecycle_record_like_cpp(
                    templates.get(9002).unwrap(),
                    None,
                )
                .unwrap(),
            ],
            [resolved_spawn],
        )
        .resolve_loaded_grid_gameobject_with_linked_trap_like_cpp(
            88,
            owner_guid,
            Some(mismatched_trap_guid),
        )
        .unwrap();

        assert!(resolved.map_object_record.is_some());
        assert!(resolved.pre_add_records.is_empty());
        assert_eq!(
            resolved.game_object.linked_trap_guid_like_cpp(),
            ObjectGuid::EMPTY
        );
    }

    #[test]
    fn gameobject_override_faction_flags_win_over_template_addon_like_cpp() {
        let addon = GameObjectTemplateAddonLifecycleRecordLikeCpp {
            entry: 9001,
            faction: 35,
            flags: 7,
            world_effect_id: 9,
            anim_kit_id: 11,
        };
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::from_overrides([
            GameObjectOverrideLifecycleRecordLikeCpp {
                spawn_id: 88,
                faction: 99,
                flags: 123,
            },
        ]);
        let (template, _) = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
            &spawn(88),
            &runtime(88),
            &template_store(Some(addon)),
            &overrides,
            1,
            0,
            false,
        )
        .unwrap();
        assert_eq!(template.faction, 99);
        assert_eq!(template.flags, 123);
        assert_eq!(template.world_effect_id, 9);
    }

    #[test]
    fn negative_spawntimesecs_forces_compatibility_and_not_spawned_by_default_like_cpp() {
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::default();
        let mut row = runtime(88);
        row.spawn_time_secs = -45;
        let (template, resolved_spawn) = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
            &spawn(88),
            &row,
            &template_store(None),
            &overrides,
            1,
            0,
            true,
        )
        .unwrap();
        let guid = ObjectGuid::create_gameobject_like_cpp(571, 9001, 22);
        let resolved =
            GameObjectLoadedGridLifecycleResolverLikeCpp::new([template], [resolved_spawn])
                .resolve_loaded_grid_gameobject_like_cpp(88, guid)
                .unwrap();
        assert!(!resolved.game_object.spawned_by_default());
        assert!(resolved.game_object.respawn_compatibility_mode());
        assert_eq!(resolved.game_object.respawn_delay_time(), 45);
    }

    #[test]
    fn non_unit_quaternion_normalizes_and_out_of_range_errors_like_cpp() {
        let normalized = normalize_rotation_like_cpp(1, [0.0, 0.0, 0.0, 0.0], 1.0).unwrap();
        assert!((normalized[2] - 0.5_f32.sin()).abs() < 0.0001);
        assert!((normalized[3] - 0.5_f32.cos()).abs() < 0.0001);
        assert!(matches!(
            normalize_rotation_like_cpp(1, [2.0, 0.0, 0.0, 0.0], 1.0),
            Err(
                GameObjectLoadedGridResolveErrorLikeCpp::RotationComponentOutOfRange {
                    component: 0,
                    ..
                }
            )
        ));
    }

    #[test]
    fn get_despawn_possibility_chest_defaults_true_like_cpp() {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[22] = 1;
        assert!(get_despawn_possibility_like_cpp(
            &GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        ));
    }

    #[test]
    fn non_transport_rejects_transport_states_like_cpp() {
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::default();
        let mut row = runtime(88);
        row.state = 24;

        let result = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
            &spawn(88),
            &row,
            &template_store(None),
            &overrides,
            1,
            0,
            true,
        );

        assert!(matches!(
            result,
            Err(GameObjectLoadedGridResolveErrorLikeCpp::InvalidGoState {
                spawn_id: 88,
                state: 24,
            })
        ));
    }

    #[test]
    fn transport_accepts_represented_transport_states_like_cpp() {
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::default();
        for (state, expected) in [
            (24, GoState::TransportActive),
            (25, GoState::TransportStopped),
        ] {
            let mut row = runtime(88);
            row.state = state;
            let (_, resolved_spawn) = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
                &spawn(88),
                &row,
                &template_store_with_type(GAMEOBJECT_TYPE_TRANSPORT, None),
                &overrides,
                1,
                0,
                true,
            )
            .unwrap();

            assert_eq!(resolved_spawn.go_state, expected);
        }
    }

    #[test]
    fn transport_stop_frame_states_are_explicitly_unsupported_like_cpp() {
        let overrides = GameObjectOverrideLifecycleStoreLikeCpp::default();
        let mut row = runtime(88);
        row.state = 26;

        let result = build_loaded_grid_gameobject_inputs_from_db_like_cpp(
            &spawn(88),
            &row,
            &template_store_with_type(GAMEOBJECT_TYPE_TRANSPORT, None),
            &overrides,
            1,
            0,
            true,
        );

        assert!(matches!(
            result,
            Err(
                GameObjectLoadedGridResolveErrorLikeCpp::UnsupportedTransportStopFrame {
                    spawn_id: 88,
                    state: 26,
                }
            )
        ));
    }
}
