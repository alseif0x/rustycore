//! Compatibility exports for canonical player access retained in wow-world.

pub use wow_world_core::canonical_player_access::{
    HonorStatsLikeCpp, PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
    canonical_player_forced_reputation_faction_ids_like_cpp,
    canonical_player_is_contested_pvp_like_cpp, canonical_player_reputation_standings_like_cpp,
    canonical_player_reputation_state_flags_like_cpp, canonical_player_unit_flags2_like_cpp,
};

pub(crate) use wow_world_core::canonical_player_access::set_player_visible_item_values_like_cpp;

#[cfg(test)]
pub(crate) use wow_world_core::canonical_player_access::{
    canonical_player_presentation_like_cpp, configure_canonical_player_party_flags_for_test,
    configure_canonical_player_vitals_for_test, with_canonical_player_at_like_cpp,
    with_canonical_player_at_mut_like_cpp,
};

#[cfg(test)]
use crate::session::SharedCanonicalMapManager;
#[cfg(test)]
use wow_core::ObjectGuid;
#[cfg(test)]
use wow_entities::Player;

#[cfg(test)]
pub(crate) fn install_canonical_player_owner_for_test(
    session: &mut crate::session::WorldSession,
    map_id: u32,
    instance_id: u32,
) -> ObjectGuid {
    let guid = session
        .player_guid()
        .unwrap_or_else(|| ObjectGuid::create_player(1, 42));
    session.set_player_guid(Some(guid));
    let canonical: SharedCanonicalMapManager =
        std::sync::Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    session.set_canonical_map_manager(canonical);
    assert!(
        session.adopt_registered_canonical_player_fixture_like_cpp(),
        "canonical Player fixture must register its production ownership handle"
    );
    guid
}
