//! SQLx-free startup source for canonical creature and gameobject catalogs.

use crate::PersistenceFutureLikeCpp;

pub const MAX_GAMEOBJECT_DATA_PERSISTENCE_LIKE_CPP: usize = 35;

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureTemplatePersistenceRowLikeCpp {
    pub entry: u32,
    pub name: String,
    pub ai_name: String,
    pub script_name: String,
    pub required_expansion: u8,
    pub faction: u32,
    pub npc_flags: u64,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub scale: f32,
    pub classification: u32,
    pub damage_school: u8,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
    pub creature_type: u32,
    pub family: u32,
    pub trainer_class: u8,
    pub unit_class: u8,
    pub vehicle_id: u32,
    pub movement_type: u8,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub flags_extra: u32,
    pub string_id: String,
    pub regen_health: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureTemplateSpellPersistenceRowLikeCpp {
    pub creature_id: u32,
    pub index: u8,
    pub spell_id: u32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureTemplateModelPersistenceRowLikeCpp {
    pub creature_id: u32,
    pub display_id: u32,
    pub display_scale: f32,
    pub probability: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureAddonPersistenceRowLikeCpp {
    pub owner_id: u64,
    pub path_id: u32,
    pub mount: u32,
    pub stand_state: u8,
    pub anim_tier: u8,
    pub vis_flags: u8,
    pub sheath_state: u8,
    pub pvp_flags: u8,
    pub emote: u32,
    pub ai_anim_kit: u16,
    pub movement_anim_kit: u16,
    pub melee_anim_kit: u16,
    pub visibility_distance_type: u8,
    pub auras: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureMountPersistenceRowLikeCpp {
    pub entry: u32,
    pub vehicle_id: u32,
    pub display_id: u32,
    pub display_scale: f32,
    pub probability: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureBaseStatsPersistenceRowLikeCpp {
    pub level: u8,
    pub unit_class: u8,
    pub base_health: [u32; 3],
    pub base_mana: u32,
    pub base_armor: u32,
    pub attack_power: u32,
    pub ranged_attack_power: u32,
    pub base_damage: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureDifficultyPersistenceRowLikeCpp {
    pub entry: u32,
    pub difficulty_id: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub health_scaling_expansion: i32,
    pub health_modifier: f32,
    pub mana_modifier: f32,
    pub armor_modifier: f32,
    pub damage_modifier: f32,
    pub creature_difficulty_id: i32,
    pub type_flags: u32,
    pub type_flags2: u32,
    pub loot_id: u32,
    pub pickpocket_loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub static_flags: [u32; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreatureEquipmentItemPersistenceLikeCpp {
    pub item_id: u32,
    pub appearance_mod_id: u16,
    pub item_visual: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureEquipmentPersistenceRowLikeCpp {
    pub creature_id: u32,
    pub id: u8,
    pub items: [CreatureEquipmentItemPersistenceLikeCpp; 3],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureModelInfoPersistenceRowLikeCpp {
    pub display_id: u32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub display_id_other_gender: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectTemplatePersistenceRowLikeCpp {
    pub entry: u32,
    pub go_type: u32,
    pub display_id: u32,
    pub name: String,
    pub size: f32,
    pub data: [u32; MAX_GAMEOBJECT_DATA_PERSISTENCE_LIKE_CPP],
    pub content_tuning_id: u32,
    pub ai_name: String,
    pub script_name: String,
    pub string_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectTemplateAddonPersistenceRowLikeCpp {
    pub entry: u32,
    pub faction: u32,
    pub flags: u32,
    pub world_effect_id: u32,
    pub anim_kit_id: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectOverridePersistenceRowLikeCpp {
    pub spawn_id: u64,
    pub faction: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureTemplateCatalogPersistenceRowsLikeCpp {
    pub templates: Vec<CreatureTemplatePersistenceRowLikeCpp>,
    pub spells: Vec<CreatureTemplateSpellPersistenceRowLikeCpp>,
    pub models: Vec<CreatureTemplateModelPersistenceRowLikeCpp>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureAddonCatalogPersistenceRowsLikeCpp {
    pub spawn_addons: Vec<CreatureAddonPersistenceRowLikeCpp>,
    pub template_addons: Vec<CreatureAddonPersistenceRowLikeCpp>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectTemplateCatalogPersistenceRowsLikeCpp {
    pub templates: Vec<GameObjectTemplatePersistenceRowLikeCpp>,
    pub addons: Vec<GameObjectTemplateAddonPersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldObjectRowsLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

pub trait WorldObjectCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_creature_classification_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, WorldObjectRowsLoadOutcomeLikeCpp<Vec<(u32, u32)>>>;
    fn load_creature_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<CreatureTemplateCatalogPersistenceRowsLikeCpp>,
    >;
    fn load_creature_sparring_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, WorldObjectRowsLoadOutcomeLikeCpp<Vec<(u32, f32)>>>;
    fn load_gameobject_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<GameObjectTemplateCatalogPersistenceRowsLikeCpp>,
    >;
    fn load_gameobject_override_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<GameObjectOverridePersistenceRowLikeCpp>>,
    >;
    fn load_creature_difficulty_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureDifficultyPersistenceRowLikeCpp>>,
    >;
    fn load_creature_base_stats_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureBaseStatsPersistenceRowLikeCpp>>,
    >;
    fn load_creature_mount_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureMountPersistenceRowLikeCpp>>,
    >;
    fn load_creature_model_info_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureModelInfoPersistenceRowLikeCpp>>,
    >;
    fn load_creature_addon_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<CreatureAddonCatalogPersistenceRowsLikeCpp>,
    >;
    fn load_creature_equipment_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldObjectRowsLoadOutcomeLikeCpp<Vec<CreatureEquipmentPersistenceRowLikeCpp>>,
    >;
}
