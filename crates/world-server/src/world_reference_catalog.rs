//! Composition boundary for immutable world reference catalogs.

use anyhow::{Result, bail};
use wow_persistence::{
    WorldObjectIdCatalogKindLikeCpp, WorldReferenceCatalogPersistencePortLikeCpp,
    WorldReferenceRowsLoadOutcomeLikeCpp, WorldSafeLocPersistenceRowLikeCpp,
    WorldSpawnCatalogKindLikeCpp,
};

fn loaded<T>(outcome: WorldReferenceRowsLoadOutcomeLikeCpp<T>) -> Result<T> {
    match outcome {
        WorldReferenceRowsLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        WorldReferenceRowsLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

const fn object_name(kind: WorldObjectIdCatalogKindLikeCpp) -> &'static str {
    match kind {
        WorldObjectIdCatalogKindLikeCpp::CreatureTemplate => "creature_template",
        WorldObjectIdCatalogKindLikeCpp::GameObjectTemplate => "gameobject_template",
        WorldObjectIdCatalogKindLikeCpp::GameEvent => "game_event",
        WorldObjectIdCatalogKindLikeCpp::WorldState => "world_state",
        WorldObjectIdCatalogKindLikeCpp::Trainer => "trainer",
        WorldObjectIdCatalogKindLikeCpp::ConversationLineTemplate => "conversation_line_template",
    }
}

pub(super) async fn load_world_id_store_like_cpp(
    persistence: &dyn WorldReferenceCatalogPersistencePortLikeCpp,
    kind: WorldObjectIdCatalogKindLikeCpp,
) -> Result<wow_data::WorldIdStore> {
    let ids = loaded(persistence.load_world_object_ids_like_cpp(kind).await)?;
    Ok(wow_data::WorldIdStore::from_ids(object_name(kind), ids))
}

pub(super) async fn load_filtering_world_id_store_like_cpp(
    persistence: &dyn WorldReferenceCatalogPersistencePortLikeCpp,
    kind: WorldObjectIdCatalogKindLikeCpp,
    mut keep_id: impl FnMut(u32) -> bool,
) -> Result<wow_data::WorldIdStore> {
    let ids = loaded(persistence.load_world_object_ids_like_cpp(kind).await)?;
    Ok(wow_data::WorldIdStore::from_ids(
        object_name(kind),
        ids.into_iter().filter(|id| keep_id(*id)),
    ))
}

pub(super) async fn load_world_spawn_id_store_like_cpp(
    persistence: &dyn WorldReferenceCatalogPersistencePortLikeCpp,
    kind: WorldSpawnCatalogKindLikeCpp,
) -> Result<wow_data::WorldSpawnIdStore> {
    let rows = loaded(persistence.load_world_spawn_ids_like_cpp(kind).await)?;
    let name = match kind {
        WorldSpawnCatalogKindLikeCpp::Creature => "creature",
        WorldSpawnCatalogKindLikeCpp::GameObject => "gameobject",
    };
    Ok(wow_data::WorldSpawnIdStore::from_entries(name, rows))
}

pub(super) async fn load_world_safe_locs_like_cpp(
    persistence: &dyn WorldReferenceCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
) -> Result<(
    wow_data::WorldSafeLocStore,
    wow_data::WorldSafeLocLoadReport,
)> {
    let rows = loaded(persistence.load_world_safe_locs_like_cpp().await)?;
    Ok(wow_data::WorldSafeLocStore::from_rows_like_cpp(
        rows.into_iter().map(
            |row: WorldSafeLocPersistenceRowLikeCpp| wow_data::WorldSafeLocRow {
                id: row.id,
                map_id: row.map_id,
                x: row.x,
                y: row.y,
                z: row.z,
                facing_degrees: row.facing_degrees,
            },
        ),
        map_store,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_names_are_bounded_and_not_supplied_by_callers() {
        assert_eq!(
            object_name(WorldObjectIdCatalogKindLikeCpp::CreatureTemplate),
            "creature_template"
        );
        assert_eq!(
            object_name(WorldObjectIdCatalogKindLikeCpp::ConversationLineTemplate),
            "conversation_line_template"
        );
    }
}
