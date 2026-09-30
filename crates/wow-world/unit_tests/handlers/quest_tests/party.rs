//! Canonical party and group fixtures for quest-sharing scenarios.

use super::{add_active_quest_in_slot_with_status, make_session};
use crate::session::WorldSession;
use crate::session::directory::PlayerRegistry;
use std::sync::Arc;
use wow_core::{ObjectGuid, Position};
use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};

pub(crate) fn install_confirm_accept_sender_snapshot(
    session: &mut WorldSession,
    sender_guid: ObjectGuid,
    quest_id: u32,
    same_group: bool,
    sender_active_status: Option<u8>,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_loaded_player_name_like_cpp("Receiver".to_string());
    session.register_in_player_registry();

    let (mut sender_session, sender_rx) = make_session();
    sender_session.set_player_guid(Some(sender_guid));
    sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
    sender_session.set_player_registry(player_registry);
    sender_session.register_in_player_registry();
    assert!(sender_session.adopt_registered_canonical_player_fixture_like_cpp());
    if let Some(status) = sender_active_status {
        add_active_quest_in_slot_with_status(&mut sender_session, quest_id, 0, status);
    }
    sender_session.sync_player_registry_state_like_cpp();

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender_guid);
    if same_group {
        if let Some(receiver_guid) = session.player_guid() {
            group.add_member(receiver_guid);
        }
    }
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    (sender_session, sender_rx)
}

/// Put one party member's canonical `Player` on the shared map.
///
/// Mirrors what a live session does at world entry; the quest-share gates read
/// reputation off this owner since #252.
///
/// Takes the three values it needs rather than `&WorldSession`: the session type
/// carries database handles, so accepting it here would register this fixture as
/// a direct persistence accessor in the ownership inventory for no reason.
fn insert_canonical_party_player_like_cpp(
    account_id: u32,
    player_guid: ObjectGuid,
    position: Position,
    canonical: &crate::session::SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
) {
    let mut player = wow_entities::Player::new(Some(u64::from(account_id)), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player.unit_mut().set_faction(1);
    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

/// Set one faction standing on a party member's canonical `Player`.
pub(crate) fn set_canonical_party_reputation_like_cpp(
    canonical: &crate::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    faction_id: u32,
    standing: i32,
) {
    let mut guard = canonical.lock().unwrap();
    let player = guard
        .find_map_mut(571, 0)
        .expect("resident party map")
        .map_mut()
        .get_typed_player_mut(guid)
        .expect("canonical party member");
    player.reputation_mut_like_cpp().insert_faction_like_cpp(
        wow_entities::PlayerFactionStateLikeCpp {
            faction_id,
            standing,
            ..Default::default()
        },
    );
}

pub(crate) fn install_represented_party(
    session: &mut WorldSession,
    sender_guid: ObjectGuid,
    receiver_guid: ObjectGuid,
) -> (Arc<PlayerRegistry>, WorldSession, flume::Receiver<Vec<u8>>) {
    let player_registry = Arc::new(PlayerRegistry::default());
    let (mut receiver_session, receiver_rx) = make_session();
    receiver_session.set_player_guid(Some(receiver_guid));
    receiver_session.set_loaded_player_name_like_cpp("Receiver".to_string());
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.set_player_position_like_cpp(Position::new(11.0, 0.0, 0.0, 0.0));
    receiver_session.set_player_registry(Arc::clone(&player_registry));

    // Production keeps every in-world player on the shared canonical map, and
    // #252 reads the receiver's reputation off that owner instead of a mirrored
    // copy. Install it here so the harness exercises the same path.
    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    insert_canonical_party_player_like_cpp(
        receiver_session.account_id,
        receiver_guid,
        receiver_session
            .player_position_like_cpp()
            .expect("party member position"),
        &canonical,
        571,
        0,
    );
    receiver_session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_canonical_map_manager(Arc::clone(&canonical));

    receiver_session.register_in_player_registry();
    assert!(receiver_session.adopt_registered_canonical_player_fixture_like_cpp());
    // Production `Player::LoadFromDB` builds the canonical Player with the
    // character's identity, and later registry movement publications read
    // `player_level_like_cpp`. Apply the session's loaded identity after
    // adoption so the fixture party member matches that owner instead of the
    // identity-less synthetic Player.
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender_guid);
    group.add_member(receiver_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry.clone());
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    (player_registry, receiver_session, receiver_rx)
}
