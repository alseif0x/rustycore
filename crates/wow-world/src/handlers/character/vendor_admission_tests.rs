use crate::handlers::test_support::world::make_session;
use wow_core::{ObjectGuid, guid::HighGuid};

#[test]
fn vendor_item_current_count_updates_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let vendor_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 7, 1);

    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        5
    );
    assert_eq!(
        session.update_vendor_item_current_count(vendor_guid, 700, 5, 60, 1, 2),
        3
    );
    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        3
    );

    if let Some(count) = session.vendor_item_counts.get_mut(&(vendor_guid, 700)) {
        count.last_increment_time =
            (wow_entities::game_time_secs_like_cpp().max(0) as u64).saturating_sub(120);
    }

    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        5
    );
    assert!(!session.vendor_item_counts.contains_key(&(vendor_guid, 700)));
}
