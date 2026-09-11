//! #743 — membership-sensitive readers resolve through the group authority.
//!
//! These live inside `crate::session` because the readers under test are
//! `pub(in crate::session)`: proving them must not widen their visibility.
//!
//! C++ readers dereference `Player::m_group`, which `Group::RemoveMember`
//! (`Group.cpp:550`) and `Group::Disband` (`Group.cpp:713`) clear inside the
//! same operation. A Rust reader that answers from the owned snapshot alone
//! would keep granting rights while the notification is still in flight.

use super::*;

use crate::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};

fn reader_session_like_cpp() -> (
    WorldSession,
    Arc<GroupRegistry>,
    u64,
    ObjectGuid,
    ObjectGuid,
) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<wow_packet::WorldPacket>(1);
    let (send_tx, _send_rx) = flume::unbounded::<Vec<u8>>();
    let mut session = WorldSession::new(
        1,
        "TapReader".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx.clone(),
    );

    let owner = ObjectGuid::create_player(1, 4_201);
    let mate = ObjectGuid::create_player(1, 4_202);
    let mut group = GroupInfo::new(mate);
    assert!(group.add_member(owner));
    let group_guid = group.group_guid;
    let group_registry = Arc::new(GroupRegistry::default());
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let canonical = player_registry
        .fixture_canonical_map_manager_like_cpp()
        .expect("canonical player fixture manager");
    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.create_world_map(0, 0).map_mut();
        for guid in [owner, mate] {
            let mut player = wow_entities::Player::new(Some(1), false);
            player.unit_mut().world_mut().object_mut().create(guid);
            player.unit_mut().world_mut().set_map(0, 0).unwrap();
            player
                .unit_mut()
                .world_mut()
                .relocate(wow_core::Position::ZERO);
            player.unit_mut().world_mut().object_mut().add_to_world();
            map.insert_map_object_record(
                wow_entities::MapObjectRecord::new_player(player).unwrap(),
            )
            .unwrap();
        }
    }
    player_registry.register_or_replace(
        owner,
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp {
                player_name: "TapOwner".into(),
                account_id: 1,
                recruiter_id: 0,
                race: 1,
                class: 1,
                sex: 0,
                active_expansion: 2,
            },
            placement: PlayerDirectoryPlacementLikeCpp {
                map_id: 0,
                instance_id: 0,
                position: wow_core::Position::ZERO,
                is_in_world: true,
                level: 1,
                is_alive: true,
            },
            active_loot_rolls: Vec::new(),
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx: flume::bounded(4).0,
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        },
        Default::default(),
    );

    session.set_player_guid(Some(owner));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_state(SessionState::LoggedIn);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));

    (session, group_registry, group_guid, owner, mate)
}

#[test]
fn tap_members_follow_the_group_authority_not_the_owned_snapshot_like_cpp() {
    let (session, group_registry, group_guid, owner, mate) = reader_session_like_cpp();

    assert_eq!(
        session.current_group_member_guids_for_tap_like_cpp(owner),
        vec![mate]
    );

    group_registry
        .remove_member_like_cpp(
            group_guid,
            owner,
            wow_social::group::GroupMemberRemovalKindLikeCpp::Leave,
            &[],
        )
        .expect("authority removal");

    // The owned snapshot is deliberately still stale here: this is the window
    // a lost `ApplyGroupRemovalLikeCpp` leaves open.
    assert_eq!(session.resolved_group_guid_like_cpp(), Some(group_guid));
    assert!(
        session
            .current_group_member_guids_for_tap_like_cpp(owner)
            .is_empty(),
        "a removed member must not keep sharing creature tap rights"
    );
}

#[test]
fn instance_owner_follows_the_group_authority_like_cpp() {
    let (session, group_registry, group_guid, owner, mate) = reader_session_like_cpp();

    assert_eq!(
        session.create_map_instance_owner_guid_like_cpp(571),
        Some(mate),
        "C++ `Group::GetRecentInstanceOwner` falls back to the group leader"
    );

    group_registry
        .remove_member_like_cpp(
            group_guid,
            owner,
            wow_social::group::GroupMemberRemovalKindLikeCpp::Leave,
            &[],
        )
        .expect("authority removal");

    assert_eq!(
        session.create_map_instance_owner_guid_like_cpp(571),
        Some(owner),
        "a removed member owns its own instance, not the group's"
    );
}
