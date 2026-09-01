//! SQLx-free startup source for bounded ObjectMgr auxiliary catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequirementPersistenceRowLikeCpp {
    pub map_id: u32,
    pub difficulty: u8,
    pub level_min: u8,
    pub level_max: u8,
    pub item: u32,
    pub item2: u32,
    pub quest_done_a: u32,
    pub quest_done_h: u32,
    pub completed_achievement: u32,
    pub quest_failed_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraveyardZonePersistenceRowLikeCpp {
    pub safe_loc_id: u32,
    pub ghost_zone_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneTemplatePersistenceRowLikeCpp {
    pub scene_id: u32,
    pub flags: u32,
    pub script_package_id: u32,
    pub encrypted: u8,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnGroupTemplatePersistenceRowLikeCpp {
    pub group_id: u32,
    pub name: String,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrinityStringPersistenceRowLikeCpp {
    pub entry: u32,
    pub content: [String; 9],
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldAuxiliaryRowsLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

pub trait WorldAuxiliaryCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_access_requirement_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<AccessRequirementPersistenceRowLikeCpp>>,
    >;

    fn load_graveyard_zone_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<GraveyardZonePersistenceRowLikeCpp>>,
    >;

    fn load_scene_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<SceneTemplatePersistenceRowLikeCpp>>,
    >;

    fn load_spawn_group_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<SpawnGroupTemplatePersistenceRowLikeCpp>>,
    >;

    fn load_trinity_string_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<TrinityStringPersistenceRowLikeCpp>>,
    >;
}
