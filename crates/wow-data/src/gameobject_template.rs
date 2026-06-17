use std::collections::{HashMap, HashSet};

use anyhow::Result;
use wow_database::WorldDatabase;
use wow_entities::{
    GameObjectTemplateData, GameObjectTemplateLifecycleRecord, MAX_GAMEOBJECT_DATA,
};

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

    pub fn entries_like_cpp(
        &self,
    ) -> impl Iterator<Item = &GameObjectTemplateLifecycleRecordLikeCpp> {
        self.templates.values()
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameObjectForQuestStoreLikeCpp {
    entries: HashSet<u32>,
}

impl GameObjectForQuestStoreLikeCpp {
    /// C++ `ObjectMgr::LoadGameObjectForQuests` builds a derived entry set from
    /// loaded `gameobject_template` rows and gameobject loot quest markers.
    pub fn from_templates_like_cpp(
        templates: &GameObjectTemplateLifecycleStoreLikeCpp,
        mut have_quest_loot_for: impl FnMut(u32) -> bool,
    ) -> Self {
        let mut entries = HashSet::new();

        for template in templates.entries_like_cpp() {
            let template_data = GameObjectTemplateData::new(template.go_type, template.data);
            let is_for_quest = match template.go_type {
                wow_entities::GAMEOBJECT_TYPE_QUESTGIVER => true,
                wow_entities::GAMEOBJECT_TYPE_CHEST => template_data
                    .chest_loot_source_like_cpp()
                    .is_some_and(|source| {
                        source.chest_quest_id != 0
                            || [source.loot_id, source.personal_loot_id, source.push_loot_id]
                                .into_iter()
                                .filter(|loot_id| *loot_id != 0)
                                .any(&mut have_quest_loot_for)
                    }),
                wow_entities::GAMEOBJECT_TYPE_GENERIC => {
                    template.data.get(5).copied().unwrap_or(0) > 0
                }
                wow_entities::GAMEOBJECT_TYPE_GOOBER => template_data
                    .goober_use_source_like_cpp()
                    .is_some_and(|source| source.quest_id > 0),
                wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE => template_data
                    .gathering_node_use_source_like_cpp()
                    .is_some_and(|source| {
                        source.loot_id != 0 && have_quest_loot_for(source.loot_id)
                    }),
                _ => false,
            };

            if is_for_quest {
                entries.insert(template.entry);
            }
        }

        Self { entries }
    }

    pub fn is_game_object_for_quests_like_cpp(&self, entry: u32) -> bool {
        self.entries.contains(&entry)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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

    fn template_with_data(
        entry: u32,
        go_type: u32,
        data: [u32; MAX_GAMEOBJECT_DATA],
    ) -> GameObjectTemplateLifecycleRecordLikeCpp {
        GameObjectTemplateLifecycleRecordLikeCpp {
            entry,
            go_type,
            display_id: 0,
            name: format!("go_{entry}"),
            size: 1.0,
            data,
            content_tuning_id: 0,
            ai_name: String::new(),
            script_name: String::new(),
            string_id: String::new(),
            addon: None,
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

    #[test]
    fn gameobject_for_quest_store_matches_cpp_template_type_filters() {
        let mut chest_quest_data = [0_u32; MAX_GAMEOBJECT_DATA];
        // C++ `GameObjectTemplate::chest.questID` is `Data8`.
        chest_quest_data[8] = 7000;
        let mut chest_loot_data = [0_u32; MAX_GAMEOBJECT_DATA];
        chest_loot_data[wow_entities::GAMEOBJECT_DATA_CHEST_LOOT] = 9000;
        let mut generic_data = [0_u32; MAX_GAMEOBJECT_DATA];
        generic_data[5] = 8000;
        let mut goober_data = [0_u32; MAX_GAMEOBJECT_DATA];
        goober_data[1] = 8100;
        let mut gathering_data = [0_u32; MAX_GAMEOBJECT_DATA];
        gathering_data[wow_entities::GAMEOBJECT_DATA_CHEST_LOOT] = 9100;

        let templates = GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            template_with_data(
                100,
                wow_entities::GAMEOBJECT_TYPE_QUESTGIVER,
                [0; MAX_GAMEOBJECT_DATA],
            ),
            template_with_data(101, wow_entities::GAMEOBJECT_TYPE_CHEST, chest_quest_data),
            template_with_data(102, wow_entities::GAMEOBJECT_TYPE_CHEST, chest_loot_data),
            template_with_data(103, wow_entities::GAMEOBJECT_TYPE_GENERIC, generic_data),
            template_with_data(104, wow_entities::GAMEOBJECT_TYPE_GOOBER, goober_data),
            template_with_data(
                105,
                wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE,
                gathering_data,
            ),
            template_with_data(
                106,
                wow_entities::GAMEOBJECT_TYPE_CHEST,
                [0; MAX_GAMEOBJECT_DATA],
            ),
            template_with_data(
                107,
                wow_entities::GAMEOBJECT_TYPE_DOOR,
                [0; MAX_GAMEOBJECT_DATA],
            ),
        ]);

        let store =
            GameObjectForQuestStoreLikeCpp::from_templates_like_cpp(&templates, |loot_id| {
                matches!(loot_id, 9000 | 9100)
            });

        for entry in [100, 101, 102, 103, 104, 105] {
            assert!(
                store.is_game_object_for_quests_like_cpp(entry),
                "entry {entry} should match C++ LoadGameObjectForQuests"
            );
        }
        assert!(!store.is_game_object_for_quests_like_cpp(106));
        assert!(!store.is_game_object_for_quests_like_cpp(107));
        assert_eq!(store.len(), 6);
    }
}
