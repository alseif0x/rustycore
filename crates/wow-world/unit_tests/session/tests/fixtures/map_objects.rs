//! Canonical map and world-object fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn configure_player_shape_mount_collision_stores_like_cpp(
    session: &mut WorldSession,
) {
    let native_display_id = crate::handlers::character::default_display_id(
        crate::session::hub_ref(&session).player_race_like_cpp(),
        crate::session::hub_ref(&session).player_gender_like_cpp(),
    );
    session.set_creature_template_mount_store(Arc::new(
        wow_data::CreatureTemplateMountStoreLikeCpp::from_entries([
            wow_data::CreatureTemplateMountEntryLikeCpp {
                entry: 1234,
                vehicle_id: 55,
                models: vec![wow_data::CreatureTemplateMountModelLikeCpp {
                    display_id: 4321,
                    display_scale: 1.0,
                    probability: 0.0,
                }],
            },
        ]),
    ));
    session.set_creature_display_info_store(Arc::new(
        wow_data::CreatureDisplayInfoStore::from_entries([
            wow_data::CreatureDisplayInfoEntry {
                id: native_display_id,
                model_id: 100,
                extended_display_info_id: 0,
                creature_model_scale: 1.2,
            },
            wow_data::CreatureDisplayInfoEntry {
                id: 4321,
                model_id: 200,
                extended_display_info_id: 0,
                creature_model_scale: 1.5,
            },
        ]),
    ));
    session.set_creature_model_data_store(Arc::new(
        wow_data::CreatureModelDataStore::from_entries([
            wow_data::CreatureModelDataEntry {
                id: 100,
                flags: 0,
                file_data_id: 0,
                collision_height: 2.0,
                hover_height: 0.75,
                model_scale: 1.1,
                mount_height: 0.0,
            },
            wow_data::CreatureModelDataEntry {
                id: 200,
                flags: 0,
                file_data_id: 0,
                collision_height: 0.0,
                hover_height: 1.25,
                model_scale: 1.0,
                mount_height: 4.0,
            },
        ]),
    ));
}

/// C++ `Player::GetNPCIfCanInteractWith` requires the canonical Player to be
/// in-world, alive and to carry a resolvable faction before a positive NPC
/// interaction fixture is meaningful.
pub(in crate::session::tests) fn adopt_live_canonical_test_player_for_interaction_like_cpp(
    session: &mut WorldSession,
) {
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            player.unit_mut().set_faction(1);
        })
        .expect("live canonical Player interaction fixture");
}

pub(in crate::session::tests) fn bind_canonical_test_player_to_registry_like_cpp(
    session: &mut WorldSession,
    registry: &Arc<PlayerRegistry>,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
) -> SharedCanonicalMapManager {
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, guid, position, map_id, 0);
    canonical
}
