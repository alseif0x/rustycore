//! SQLx-free startup source contracts for C++ vehicle catalogs.

use crate::PersistenceFutureLikeCpp;

pub const VEHICLE_SEAT_COUNT_LIKE_CPP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleHotfixPersistenceRowLikeCpp {
    pub id: u32,
    pub flags: i32,
    pub flags_b: i32,
    pub seat_ids: [u16; VEHICLE_SEAT_COUNT_LIKE_CPP],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleSeatHotfixPersistenceRowLikeCpp {
    pub id: u32,
    pub attachment_offset_x: f32,
    pub attachment_offset_y: f32,
    pub attachment_offset_z: f32,
    pub flags: i32,
    pub flags_b: i32,
    pub flags_c: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VehicleHotfixLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// Independent Hotfix DB capability for the two DB2 overlays. The concrete
/// adapter owns statement identity and decoding; the data owner retains DB2
/// parsing and replacement semantics.
pub trait VehicleHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_vehicle_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleHotfixLoadOutcomeLikeCpp<VehicleHotfixPersistenceRowLikeCpp>,
    >;

    fn load_vehicle_seat_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleHotfixLoadOutcomeLikeCpp<VehicleSeatHotfixPersistenceRowLikeCpp>,
    >;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleTemplatePersistenceRowLikeCpp {
    pub creature_entry: u32,
    pub despawn_delay_ms: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleTemplateAccessoryPersistenceRowLikeCpp {
    pub creature_entry: u32,
    pub accessory_entry: u32,
    pub seat_id: i8,
    pub is_minion: bool,
    pub summoned_type: u8,
    pub summon_time_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleSpawnAccessoryPersistenceRowLikeCpp {
    pub spawn_guid: u64,
    pub accessory_entry: u32,
    pub seat_id: i8,
    pub is_minion: bool,
    pub summoned_type: u8,
    pub summon_time_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VehicleWorldCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// Independent World DB capability for vehicle templates and accessories.
/// The adapter owns SQL and row decoding; the data owner retains grouping,
/// duplicate order and spawn-specific precedence.
pub trait VehicleWorldCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_vehicle_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleTemplatePersistenceRowLikeCpp>,
    >;

    fn load_vehicle_template_accessory_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleTemplateAccessoryPersistenceRowLikeCpp>,
    >;

    fn load_vehicle_spawn_accessory_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleSpawnAccessoryPersistenceRowLikeCpp>,
    >;
}
