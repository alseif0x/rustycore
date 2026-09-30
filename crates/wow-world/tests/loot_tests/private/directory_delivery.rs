//! Original application cases using the feature fixtures and production operations.
use super::support::*;

#[test]
fn loot_directory_delivery_rejects_replaced_session_generation_like_cpp() {
    let guid = ObjectGuid::create_player(1, 91);
    let registry = PlayerRegistry::new();
    let (first_tx, first_rx) = flume::bounded(2);
    registry.register_or_replace(guid, broadcast_info(guid, first_tx), Default::default());
    let stale = registry
        .loot_presence(guid)
        .expect("first connected loot recipient");

    let (replacement_tx, replacement_rx) = flume::bounded(2);
    registry.register_or_replace(
        guid,
        broadcast_info(guid, replacement_tx),
        Default::default(),
    );

    assert_eq!(
        registry.send_current_packet(stale.registration, vec![0xAA]),
        Err(wow_world::session::directory::PlayerDirectorySendError::StaleRegistration)
    );
    assert!(first_rx.try_recv().is_err());
    assert!(replacement_rx.try_recv().is_err());

    let current = registry
        .loot_presence(guid)
        .expect("replacement loot recipient");
    registry
        .send_current_packet(current.registration, vec![0xBB])
        .expect("current generation receives its packet");
    assert_eq!(replacement_rx.try_recv().unwrap(), vec![0xBB]);
}
