//! SQLx-free startup source contract for the C++ mount catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MountHotfixRowLikeCpp {
    pub id: u32,
    pub mount_type_id: u16,
    pub flags: u16,
    pub source_type_enum: i8,
    pub source_spell_id: i32,
    pub player_condition_id: u32,
    pub mount_fly_ride_height: f32,
    pub ui_model_scene_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountDefinitionRowLikeCpp {
    pub spell_id: u32,
    pub other_faction_spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountCapabilityHotfixRowLikeCpp {
    pub id: u32,
    pub flags: u8,
    pub req_riding_skill: u16,
    pub req_area_id: u16,
    pub req_spell_aura_id: u32,
    pub req_spell_known_id: i32,
    pub mod_spell_aura_id: i32,
    pub req_map_id: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountTypeXCapabilityHotfixRowLikeCpp {
    pub id: u32,
    pub mount_type_id: u16,
    pub mount_capability_id: u16,
    pub order_index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountXDisplayHotfixRowLikeCpp {
    pub id: u32,
    pub creature_display_info_id: i32,
    pub player_condition_id: u32,
    pub mount_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MountCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// C++ DB2 hotfix overlays plus `CollectionMgr::LoadMountDefinitions` source.
/// The adapter owns statement identity and row decoding; `wow-data` retains
/// DB2 parsing, replacement semantics, derived indices and validation.
pub trait MountCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_mount_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountHotfixRowLikeCpp>>;

    fn load_mount_definition_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountDefinitionRowLikeCpp>>;

    fn load_mount_capability_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountCapabilityHotfixRowLikeCpp>>;

    fn load_mount_type_x_capability_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        MountCatalogLoadOutcomeLikeCpp<MountTypeXCapabilityHotfixRowLikeCpp>,
    >;

    fn load_mount_x_display_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountXDisplayHotfixRowLikeCpp>>;
}
