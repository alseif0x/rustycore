use std::collections::HashMap;

use anyhow::Result;
use wow_database::WorldDatabase;
use wow_entities::{GameObjectTemplateLifecycleRecord, MAX_GAMEOBJECT_DATA};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameObjectTemplateAddonLifecycleRecordLikeCpp {
    pub entry: u32,
    pub faction: u32,
    pub flags: u32,
    pub world_effect_id: u32,
    pub anim_kit_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameObjectOverrideLifecycleRecordLikeCpp {
    pub spawn_id: u64,
    pub faction: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectTemplateLifecycleRecordLikeCpp {
    pub entry: u32,
    pub go_type: u32,
    pub display_id: u32,
    pub name: String,
    pub size: f32,
    pub data: [u32; MAX_GAMEOBJECT_DATA],
    pub content_tuning_id: u32,
    pub ai_name: String,
    pub script_name: String,
    pub string_id: String,
    pub addon: Option<GameObjectTemplateAddonLifecycleRecordLikeCpp>,
}

#[derive(Debug, Clone, Default)]
pub struct GameObjectTemplateLifecycleStoreLikeCpp {
    templates: HashMap<u32, GameObjectTemplateLifecycleRecordLikeCpp>,
}

impl GameObjectTemplateLifecycleStoreLikeCpp {
    pub fn from_templates(
        templates: impl IntoIterator<Item = GameObjectTemplateLifecycleRecordLikeCpp>,
    ) -> Self {
        Self {
            templates: templates
                .into_iter()
                .map(|template| (template.entry, template))
                .collect(),
        }
    }

    /// Loads the DB-backed template/addon dependency used by represented
    /// `GameObject::LoadFromDB` lifecycle construction.
    ///
    /// C++ anchors:
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:7552-7610`
    ///   `ObjectMgr::LoadGameObjectTemplate` field order.
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:7770-7854`
    ///   `LoadGameObjectTemplateAddons` fields consumed by `GameObject::Create`.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let mut templates = HashMap::new();
        let mut result = db
            .direct_query(
                "SELECT entry, type, displayId, name, size, \
                 Data0, Data1, Data2, Data3, Data4, Data5, Data6, Data7, \
                 Data8, Data9, Data10, Data11, Data12, Data13, Data14, Data15, \
                 Data16, Data17, Data18, Data19, Data20, Data21, Data22, Data23, \
                 Data24, Data25, Data26, Data27, Data28, Data29, Data30, Data31, \
                 Data32, Data33, Data34, ContentTuningId, AIName, ScriptName, StringId \
                 FROM gameobject_template",
            )
            .await?;
        if !result.is_empty() {
            loop {
                let mut data = [0_u32; MAX_GAMEOBJECT_DATA];
                for (index, slot) in data.iter_mut().enumerate() {
                    *slot = result.try_read::<u32>(5 + index).unwrap_or(0);
                }
                let record = GameObjectTemplateLifecycleRecordLikeCpp {
                    entry: result.try_read::<u32>(0).unwrap_or(0),
                    go_type: result.try_read::<u32>(1).unwrap_or(0),
                    display_id: result.try_read::<u32>(2).unwrap_or(0),
                    name: result.try_read::<String>(3).unwrap_or_default(),
                    size: result.try_read::<f32>(4).unwrap_or(1.0),
                    data,
                    content_tuning_id: result.try_read::<u32>(5 + MAX_GAMEOBJECT_DATA).unwrap_or(0),
                    ai_name: result
                        .try_read::<String>(6 + MAX_GAMEOBJECT_DATA)
                        .unwrap_or_default(),
                    script_name: result
                        .try_read::<String>(7 + MAX_GAMEOBJECT_DATA)
                        .unwrap_or_default(),
                    string_id: result
                        .try_read::<String>(8 + MAX_GAMEOBJECT_DATA)
                        .unwrap_or_default(),
                    addon: None,
                };
                templates.insert(record.entry, record);
                if !result.next_row() {
                    break;
                }
            }
        }

        let mut addon_result = db
            .direct_query(
                "SELECT entry, faction, flags, WorldEffectID, AIAnimKitID FROM gameobject_template_addon",
            )
            .await?;
        if !addon_result.is_empty() {
            loop {
                let entry = addon_result.try_read::<u32>(0).unwrap_or(0);
                let addon = GameObjectTemplateAddonLifecycleRecordLikeCpp {
                    entry,
                    faction: addon_result.try_read::<u32>(1).unwrap_or(0),
                    flags: addon_result.try_read::<u32>(2).unwrap_or(0),
                    world_effect_id: addon_result.try_read::<u32>(3).unwrap_or(0),
                    anim_kit_id: addon_result.try_read::<u16>(4).unwrap_or(0),
                };
                if let Some(template) = templates.get_mut(&entry) {
                    template.addon = Some(addon);
                }
                if !addon_result.next_row() {
                    break;
                }
            }
        }

        Ok(Self { templates })
    }

    pub fn get(&self, entry: u32) -> Option<&GameObjectTemplateLifecycleRecordLikeCpp> {
        self.templates.get(&entry)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

/// Converts a DB-backed `gameobject_template` / `gameobject_template_addon`
/// record into the entity lifecycle template consumed by represented
/// GameObject creation paths that do not have a DB spawn row.
///
/// C++ anchors:
/// - `ObjectMgr.cpp:7552-7610` loads `gameobject_template`.
/// - `ObjectMgr.cpp:7770-7854` loads `gameobject_template_addon`.
/// - `GameObject.cpp:1187-1200` `GameObject::CreateGameObject` consults only
///   the template/addon sources for spell-created dynamic GameObjects; spawn
///   overrides are a `LoadFromDB` concern and are intentionally not applied.
pub fn gameobject_template_lifecycle_record_like_cpp(
    template: &GameObjectTemplateLifecycleRecordLikeCpp,
) -> GameObjectTemplateLifecycleRecord {
    let addon = template.addon;
    GameObjectTemplateLifecycleRecord {
        entry: template.entry,
        name: template.name.clone(),
        go_type: template.go_type,
        display_id: template.display_id,
        scale: template.size,
        faction: addon.map(|record| record.faction).unwrap_or(0),
        flags: addon.map(|record| record.flags).unwrap_or(0),
        data: template.data,
        world_effect_id: addon.map(|record| record.world_effect_id).unwrap_or(0),
        anim_kit_id: addon.map(|record| record.anim_kit_id).unwrap_or(0),
        level: template.content_tuning_id,
        percent_health: 100,
        custom_param: 0,
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameObjectOverrideLifecycleStoreLikeCpp {
    overrides: HashMap<u64, GameObjectOverrideLifecycleRecordLikeCpp>,
}

impl GameObjectOverrideLifecycleStoreLikeCpp {
    pub fn from_overrides(
        overrides: impl IntoIterator<Item = GameObjectOverrideLifecycleRecordLikeCpp>,
    ) -> Self {
        Self {
            overrides: overrides
                .into_iter()
                .map(|record| (record.spawn_id, record))
                .collect(),
        }
    }

    /// C++ `GameObject::Create` consults `ObjectMgr::GetGameObjectOverride(spawnId)`
    /// and lets spawn-specific faction/flags win over template-addon values.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let mut overrides = HashMap::new();
        let mut result = db
            .direct_query("SELECT spawnId, faction, flags FROM gameobject_overrides")
            .await?;
        if !result.is_empty() {
            loop {
                let record = GameObjectOverrideLifecycleRecordLikeCpp {
                    spawn_id: result.try_read::<u64>(0).unwrap_or(0),
                    faction: result.try_read::<u32>(1).unwrap_or(0),
                    flags: result.try_read::<u32>(2).unwrap_or(0),
                };
                overrides.insert(record.spawn_id, record);
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(Self { overrides })
    }

    pub fn get(&self, spawn_id: u64) -> Option<&GameObjectOverrideLifecycleRecordLikeCpp> {
        self.overrides.get(&spawn_id)
    }

    pub fn len(&self) -> usize {
        self.overrides.len()
    }

    pub fn is_empty(&self) -> bool {
        self.overrides.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(
        addon: Option<GameObjectTemplateAddonLifecycleRecordLikeCpp>,
    ) -> GameObjectTemplateLifecycleRecordLikeCpp {
        let mut data = [0_u32; MAX_GAMEOBJECT_DATA];
        data[3] = 333;
        GameObjectTemplateLifecycleRecordLikeCpp {
            entry: 42,
            go_type: 5,
            display_id: 700,
            name: "summoned object".to_string(),
            size: 1.25,
            data,
            content_tuning_id: 80,
            ai_name: "SmartGameObjectAI".to_string(),
            script_name: "scripted_go".to_string(),
            string_id: "string-id".to_string(),
            addon,
        }
    }

    #[test]
    fn gameobject_template_lifecycle_record_without_addon_uses_cpp_create_defaults() {
        let resolved = gameobject_template_lifecycle_record_like_cpp(&template(None));

        assert_eq!(resolved.entry, 42);
        assert_eq!(resolved.name, "summoned object");
        assert_eq!(resolved.go_type, 5);
        assert_eq!(resolved.display_id, 700);
        assert_eq!(resolved.scale, 1.25);
        assert_eq!(resolved.data[3], 333);
        assert_eq!(resolved.faction, 0);
        assert_eq!(resolved.flags, 0);
        assert_eq!(resolved.world_effect_id, 0);
        assert_eq!(resolved.anim_kit_id, 0);
        assert_eq!(resolved.level, 80);
        assert_eq!(resolved.percent_health, 100);
        assert_eq!(resolved.custom_param, 0);
    }

    #[test]
    fn gameobject_template_lifecycle_record_applies_template_addon_like_cpp() {
        let addon = GameObjectTemplateAddonLifecycleRecordLikeCpp {
            entry: 42,
            faction: 35,
            flags: 0x20,
            world_effect_id: 77,
            anim_kit_id: 9,
        };

        let resolved = gameobject_template_lifecycle_record_like_cpp(&template(Some(addon)));

        assert_eq!(resolved.faction, 35);
        assert_eq!(resolved.flags, 0x20);
        assert_eq!(resolved.world_effect_id, 77);
        assert_eq!(resolved.anim_kit_id, 9);
        assert_eq!(resolved.level, 80);
    }
}
