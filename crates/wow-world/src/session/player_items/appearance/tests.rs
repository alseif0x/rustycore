//! The historical cfg(test) ownerless partial write remains a unit-only branch.
use super::*;

#[test]
fn appearance_unit_ownerless_temporary_write_remains_partial_without_player_fields() {
    let (_, packet_rx) = flume::bounded(1);
    let (send_tx, _) = flume::unbounded();
    let mut session = WorldSession::new(
        1, "TestAccount".into(), 0, 2, 9, 54261, vec![0; 40],
        "esES".into(), packet_rx, send_tx,
    );
    session.set_player_guid(Some(ObjectGuid::create_player(1, 94)));
    let item_guid = ObjectGuid::create_item(1, 900);
    assert!(session.add_temporary_item_appearance_like_cpp(65, item_guid).is_none());
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, true));
    assert_eq!(session.items_providing_temporary_appearance_like_cpp(65),
        HashSet::from([item_guid]));
    assert!(session.add_item_appearance_like_cpp(65).is_none());
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, true));
}
