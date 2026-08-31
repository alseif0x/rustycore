//! SQLx-free World source contract for the GameTele catalog.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub struct GameTelePersistenceRowLikeCpp {
    pub id: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub map_id: u16,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameTeleCatalogLoadOutcomeLikeCpp {
    Loaded(Vec<GameTelePersistenceRowLikeCpp>),
    Failed { reason: String },
}

pub trait GameTeleCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_rows_like_cpp(&self)
    -> PersistenceFutureLikeCpp<'_, GameTeleCatalogLoadOutcomeLikeCpp>;
}
