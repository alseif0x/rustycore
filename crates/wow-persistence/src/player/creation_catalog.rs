//! SQLx-free World source contract for represented Player-creation catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCreateInfoPersistenceRowLikeCpp {
    pub race: u8,
    pub class: u8,
    pub map_id: u16,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub npe_map_id: Option<u32>,
    pub npe_position_x: Option<f32>,
    pub npe_position_y: Option<f32>,
    pub npe_position_z: Option<f32>,
    pub npe_orientation: Option<f32>,
    pub npe_transport_guid: Option<u64>,
    pub npe_transport_entry: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCreateCastSpellPersistenceRowLikeCpp {
    pub race_mask: u64,
    pub class_mask: u32,
    pub spell_id: u32,
    pub create_mode: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCreateCustomSpellPersistenceRowLikeCpp {
    pub race_mask: u64,
    pub class_mask: u32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerCreationCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// Represented World sources used by C++ `ObjectMgr::LoadPlayerInfo`.
///
/// These operations are deliberately staged. Base rows must be validated and
/// published before startup may reach the two spell sources, and the existing
/// Rust publication order remains cast spells followed by custom spells.
pub trait PlayerCreationCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_player_create_info_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateInfoPersistenceRowLikeCpp>,
    >;

    fn load_player_create_cast_spell_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateCastSpellPersistenceRowLikeCpp>,
    >;

    fn load_player_create_custom_spell_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateCustomSpellPersistenceRowLikeCpp>,
    >;
}
