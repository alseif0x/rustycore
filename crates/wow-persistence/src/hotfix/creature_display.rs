//! SQLx-free Hotfix contract for creature display/model DB2 overlays.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureDisplayInfoHotfixRowLikeCpp {
    pub id: u32,
    pub model_id: u16,
    pub extended_display_info_id: i32,
    pub creature_model_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureModelDataHotfixRowLikeCpp {
    pub id: u32,
    pub flags: u32,
    pub file_data_id: u32,
    pub collision_height: f32,
    pub hover_height: f32,
    pub model_scale: f32,
    pub mount_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CreatureDisplayHotfixLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

pub trait CreatureDisplayHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_creature_display_info_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CreatureDisplayHotfixLoadOutcomeLikeCpp<CreatureDisplayInfoHotfixRowLikeCpp>,
    >;

    fn load_creature_model_data_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CreatureDisplayHotfixLoadOutcomeLikeCpp<CreatureModelDataHotfixRowLikeCpp>,
    >;
}
