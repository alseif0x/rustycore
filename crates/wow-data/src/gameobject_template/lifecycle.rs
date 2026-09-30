//! GameObjectTemplate lifecycle records, stores, and projections.

use std::collections::{HashMap, HashSet};

use super::catalog::{
    GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GENERIC,
    GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_QUESTGIVER, GameObjectTemplateData,
    MAX_GAMEOBJECT_DATA,
};
pub use wow_data_model::game_object::GameObjectTemplateLifecycleRecord;

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

    pub fn from_templates_and_addons_like_cpp(
        templates: impl IntoIterator<Item = GameObjectTemplateLifecycleRecordLikeCpp>,
        addons: impl IntoIterator<Item = GameObjectTemplateAddonLifecycleRecordLikeCpp>,
    ) -> Self {
        let mut store = Self::from_templates(templates);
        for addon in addons {
            if let Some(template) = store.templates.get_mut(&addon.entry) {
                template.addon = Some(addon);
            }
        }
        store
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
            let is_for_quest =
                match template.go_type {
                    GAMEOBJECT_TYPE_QUESTGIVER => true,
                    GAMEOBJECT_TYPE_CHEST => template_data
                        .chest_loot_source_like_cpp()
                        .is_some_and(|source| {
                            source.chest_quest_id != 0
                                || [source.loot_id, source.personal_loot_id, source.push_loot_id]
                                    .into_iter()
                                    .filter(|loot_id| *loot_id != 0)
                                    .any(&mut have_quest_loot_for)
                        }),
                    GAMEOBJECT_TYPE_GENERIC => template.data.get(5).copied().unwrap_or(0) > 0,
                    GAMEOBJECT_TYPE_GOOBER => template_data
                        .goober_use_source_like_cpp()
                        .is_some_and(|source| source.quest_id > 0),
                    GAMEOBJECT_TYPE_GATHERING_NODE => template_data
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
mod tests;
