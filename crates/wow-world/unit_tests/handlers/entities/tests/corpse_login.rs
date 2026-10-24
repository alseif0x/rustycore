//! Player corpse restoration during login.

use super::*;
use wow_persistence::PlayerCorpseLocationLoadRow;

const AT_LOGIN_RESURRECT: u16 = 0x0100;
const RELEASE_TIMER: u32 = 0x0000_0008;

fn corpse_login_session(
    current_map: u32,
    maps: Arc<MapStore>,
    dead: bool,
) -> (
    crate::session::WorldSession,
    ObjectGuid,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, send_rx) = make_session();
    session.set_map_store(maps);
    let guid = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(
        &mut session,
        current_map,
        0,
    );
    session.set_loaded_player_identity_like_cpp(current_map as u16, 1, 1, 80, 0);
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(if dead { 0 } else { 100 });
        if dead {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Dead);
        }
    });
    (session, guid, send_rx)
}

fn corpse_location_row(map_id: u16) -> PlayerCorpseLocationLoadRow {
    PlayerCorpseLocationLoadRow {
        map_id: Some(map_id),
        x: Some(11.0),
        y: Some(22.0),
        z: Some(33.0),
        orientation: Some(1.5),
    }
}

async fn restore_corpse(
    session: &mut crate::session::WorldSession,
    row: Option<PlayerCorpseLocationLoadRow>,
) {
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = GraveyardStore::default();
    wow_world_entities::restore_player_corpse_after_login(
        session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: &graveyards,
        },
        row,
    )
    .await;
}

fn maps_with_world_and_instance() -> Arc<MapStore> {
    Arc::new(MapStore::from_entries([
        MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 33,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]))
}

#[tokio::test]
async fn alive_player_clears_stale_canonical_corpse_location_and_resurrect_flag() {
    let (mut session, _, _) = corpse_login_session(571, maps_with_world_and_instance(), false);
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.resurrection_state_mut_like_cpp().corpse_location =
            Some((571, Position::new(1.0, 2.0, 3.0, 0.0)));
        player.set_at_login_flags_like_cpp(AT_LOGIN_RESURRECT);
    });

    restore_corpse(&mut session, Some(corpse_location_row(33))).await;

    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        None
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .persistent_capability_state_like_cpp()
            .at_login_flags),
        Some(0)
    );
    assert!(session.player_is_alive_like_cpp());
}

#[tokio::test]
async fn dead_player_restores_other_instance_corpse_and_clears_release_timer() {
    let (mut session, _, send_rx) = corpse_login_session(571, maps_with_world_and_instance(), true);
    crate::session::hub_mut(&mut session).mutate_active_player_update_state_like_cpp(|state| {
        state.active_local_flags |= RELEASE_TIMER;
    });

    restore_corpse(&mut session, Some(corpse_location_row(33))).await;

    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        Some((33, Position::new(11.0, 22.0, 33.0, 1.5)))
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            player.gameplay_state().active_local_flags & RELEASE_TIMER
        }),
        Some(0)
    );
    assert!(crate::session::hub_ref(&session)
        .player_movement_flags_like_cpp()
        .contains(wow_constants::MovementFlag::WATER_WALK));
    let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes.last().copied(),
        Some(wow_constants::ServerOpcodes::MoveSetWaterWalk as u16),
        "ghost login enables water walking after restoring the ghost auras"
    );
}

#[tokio::test]
async fn dead_player_current_world_corpse_sets_release_timer() {
    let (mut session, _, _) = corpse_login_session(571, maps_with_world_and_instance(), true);
    crate::session::hub_mut(&mut session).mutate_active_player_update_state_like_cpp(|state| {
        state.active_local_flags &= !RELEASE_TIMER;
    });

    restore_corpse(&mut session, Some(corpse_location_row(571))).await;

    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        Some((571, Position::new(11.0, 22.0, 33.0, 1.5)))
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            player.gameplay_state().active_local_flags & RELEASE_TIMER
        }),
        Some(RELEASE_TIMER)
    );
}

#[tokio::test]
async fn dead_player_without_corpse_keeps_location_empty() {
    let (mut session, _, _) = corpse_login_session(571, maps_with_world_and_instance(), true);

    restore_corpse(&mut session, None).await;

    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        None
    );
    assert!(!session.player_is_alive_like_cpp());
}

#[tokio::test]
async fn invalid_corpse_coordinates_do_not_replace_canonical_location() {
    let (mut session, _, _) = corpse_login_session(571, maps_with_world_and_instance(), true);
    let original = Position::new(2.0, 3.0, 4.0, 0.5);
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.resurrection_state_mut_like_cpp().corpse_location = Some((571, original));
    });
    let mut invalid = corpse_location_row(33);
    invalid.x = Some(f32::NAN);

    restore_corpse(&mut session, Some(invalid)).await;

    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        Some((571, original))
    );
}

#[tokio::test]
async fn corpse_row_for_unknown_map_does_not_create_a_location() {
    let (mut session, _, _) = corpse_login_session(571, maps_with_world_and_instance(), true);

    restore_corpse(&mut session, Some(corpse_location_row(60_000))).await;

    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        None
    );
}

#[tokio::test]
async fn at_login_resurrect_ignores_saved_location_and_resurrects_player() {
    let (mut session, _, _) = corpse_login_session(571, maps_with_world_and_instance(), true);
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.set_at_login_flags_like_cpp(AT_LOGIN_RESURRECT);
    });

    restore_corpse(&mut session, Some(corpse_location_row(33))).await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        None
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .persistent_capability_state_like_cpp()
            .at_login_flags),
        Some(0)
    );
}
