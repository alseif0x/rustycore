//! SQLx-free source for immutable world identifiers and safe locations.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldObjectIdCatalogKindLikeCpp {
    CreatureTemplate,
    GameObjectTemplate,
    GameEvent,
    WorldState,
    Trainer,
    ConversationLineTemplate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSpawnCatalogKindLikeCpp {
    Creature,
    GameObject,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldSafeLocPersistenceRowLikeCpp {
    pub id: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub facing_degrees: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldReferenceRowsLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

pub trait WorldReferenceCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_world_object_ids_like_cpp(
        &self,
        kind: WorldObjectIdCatalogKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, WorldReferenceRowsLoadOutcomeLikeCpp<Vec<u32>>>;

    fn load_world_spawn_ids_like_cpp(
        &self,
        kind: WorldSpawnCatalogKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, WorldReferenceRowsLoadOutcomeLikeCpp<Vec<(u32, u32)>>>;

    fn load_world_safe_locs_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldReferenceRowsLoadOutcomeLikeCpp<Vec<WorldSafeLocPersistenceRowLikeCpp>>,
    >;
}
