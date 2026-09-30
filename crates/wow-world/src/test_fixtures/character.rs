use crate::session::WorldSession;
pub use crate::handlers::character::inventory_fixture_access::*;

mod inventory;
pub use inventory::*;
mod inventory_handlers;
pub use inventory_handlers::*;
mod inventory_children;
pub use inventory_children::*;
mod inventory_effects;
pub use inventory_effects::*;
mod inventory_bank;
pub use inventory_bank::*;

pub fn loaded_player_identity_for_test(session: &WorldSession) -> (u16, u8, u8, u8, u8) {
    (
        session.player_map_id_like_cpp(),
        session.player_race_like_cpp(),
        session.player_class_like_cpp(),
        session.player_level_like_cpp(),
        session.player_gender_like_cpp(),
    )
}

pub fn install_object_mgr_catalogs_for_test(
    session: &mut WorldSession,
    catalogs: std::sync::Arc<crate::session::ObjectMgrCatalogsLikeCpp>,
) {
    session.set_object_mgr_catalogs_like_cpp(catalogs);
}

pub fn install_tact_key_store_for_test(
    session: &mut WorldSession,
    store: std::sync::Arc<wow_data::TactKeyStore>,
) {
    session.set_tact_key_store(store);
}

pub async fn handle_db_query_bulk_for_test(
    session: &mut WorldSession,
    query: wow_packet::packets::misc::DbQueryBulk,
) {
    session.handle_db_query_bulk(query).await;
}

pub async fn handle_creature_query_for_test(
    session: &mut WorldSession,
    query: wow_packet::packets::query::QueryCreature,
) {
    session.handle_query_creature(query).await;
}

pub async fn handle_gameobject_query_for_test(
    session: &mut WorldSession,
    query: wow_packet::packets::query::QueryGameObject,
) {
    session.handle_query_game_object(query).await;
}

pub async fn handle_page_text_query_for_test(
    session: &mut WorldSession,
    query: wow_packet::packets::query::QueryPageText,
) {
    session.handle_query_page_text(query).await;
}

/// Exercise the registered DB2 bulk route with the caller's original catalogs.
pub async fn dispatch_db_query_bulk_for_test(
    session: &mut WorldSession,
    catalogs: &crate::session::SessionHandlerCatalogsLikeCpp,
    packet: wow_packet::WorldPacket,
) {
    session.dispatch_packet(catalogs, packet).await;
}

mod appearance;
pub use appearance::*;

mod gossip;
pub use gossip::*;
