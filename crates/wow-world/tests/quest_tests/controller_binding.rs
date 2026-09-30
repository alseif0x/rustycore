//! Explicit controller-fixture attachment versus normal feature-enabled production.

use super::*;
use super::reputation::install_world_map_catalogs;

#[test]
fn quest_controller_fixture_delays_owner_installation_until_map_resolution() {
    let (mut session, send_rx) = make_session();
    install_world_map_catalogs(&mut session);
    let guid = ObjectGuid::create_player(1, 7106);
    let position = Position::new(3700.0, 1500.0, 120.0, 0.0);

    attach_player_controller_for_test(
        &mut session, guid, "FixtureAttachment".into(), position, 571, 1, 1, 80, 0,
    );

    assert!(mutate_canonical_player_for_test(&session, |_| ()).is_none());
    assert_eq!(loaded_player_identity_for_test(&session), (571, 1, 1, 80, 0));
    assert!(send_rx.try_recv().is_err());
    ensure_world_map_for_current_player_for_test(&mut session).expect("world map");
    let observed = mutate_canonical_player_for_test(&session, |player| {
        let position = player.unit().world().position();
        (position.x, position.y, position.z)
    });
    assert_eq!(observed, Some((3700.0, 1500.0, 120.0)));
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_controller_normal_feature_attachment_keeps_immediate_detached_owner() {
    let (mut session, send_rx) = make_session();
    install_world_map_catalogs(&mut session);
    let guid = ObjectGuid::create_player(1, 7107);
    let position = Position::new(3710.0, 1510.0, 125.0, 0.0);

    // This existing facade forwards normal production attachment, not the opt-in route.
    assert!(ensure_login_player_controller_for_test(
        &mut session, guid, "ProductionAttachment".into(), position, 571, 1, 1, 80, 0,
    ));
    let observed = mutate_canonical_player_for_test(&session, |player| {
        let position = player.unit().world().position();
        (position.x, position.y, position.z)
    });
    assert_eq!(observed, Some((3710.0, 1510.0, 125.0)));
    assert_eq!(loaded_player_identity_for_test(&session), (571, 1, 1, 80, 0));
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_controller_fixture_missing_catalog_fails_before_owner_installation() {
    let (mut session, send_rx) = make_session();
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    attach_player_controller_for_test(
        &mut session, ObjectGuid::create_player(1, 7108), "MissingMap".into(),
        Position::new(3700.0, 1500.0, 120.0, 0.0), 571, 1, 1, 80, 0,
    );

    assert!(mutate_canonical_player_for_test(&session, |_| ()).is_none());
    assert!(ensure_world_map_for_current_player_for_test(&mut session).is_none());
    assert!(mutate_canonical_player_for_test(&session, |_| ()).is_none());
    assert!(send_rx.try_recv().is_err());
}
