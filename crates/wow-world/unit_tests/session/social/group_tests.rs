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
use std::sync::Arc;
use wow_data::character_progression::{ChrRacesEntry, ChrRacesStore};
use wow_data::progression_rewards::{FactionTemplateEntry, FactionTemplateStore};
use wow_packet::ServerPacket;
use wow_packet::packets::party::{
    PartyDifficultySettings, PartyLootSettings, PartyPlayerInfo, PartyUpdate,
};
use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};
use wow_world_core::session::SessionState;

fn login_player_registration_like_cpp(
    name: &str,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<crate::session::mailbox::SessionCommand>,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp::new_with_bnet(
            name.to_owned(),
            1,
            0,
            0,
            1,
            1,
            0,
            2,
            "enUS",
            None,
        ),
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: wow_core::Position::ZERO,
            is_in_world: true,
            level: 80,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        client_visible_transports_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

fn human_race_entry_like_cpp(id: u32, faction_id: i16) -> ChrRacesEntry {
    ChrRacesEntry {
        id,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: "Human".into(),
        flags: 0,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id: 0,
        splash_sound_id: 0,
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        altered_form_start_visual_kit_id: [0; 3],
        altered_form_finish_visual_kit_id: [0; 3],
        heritage_armor_achievement_id: 0,
        starting_level: 1,
        ui_display_order: 0,
        playable_race_bit: 0,
        female_skeleton_file_data_id: 0,
        male_skeleton_file_data_id: 0,
        helmet_anim_scaling_race_id: 0,
        transmogrify_disabled_slot_mask: 0,
        faction_id,
        cinematic_sequence_id: 0,
        base_language: 0,
        creature_type: 0,
        alliance: 0,
        race_related: 0,
        unaltered_visual_race_id: 0,
        default_class_id: 0,
        neutral_race_id: 0,
    }
}

fn set_group_slot_like_cpp(
    group: &mut GroupInfo,
    guid: ObjectGuid,
    name: &str,
    class: u8,
    subgroup: u8,
    flags: u8,
    roles: u8,
) {
    let slot = group
        .member_slots
        .iter_mut()
        .find(|slot| slot.guid == guid)
        .expect("group member slot");
    slot.name = name.to_owned();
    slot.race = 1;
    slot.class = class;
    slot.subgroup = subgroup;
    slot.flags = flags;
    slot.roles = roles;
}

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
        crate::session::registry::build_dispatch_table(),
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
            identity: PlayerDirectoryIdentityLikeCpp::new_with_bnet(
                "TapOwner", 1, 0, 0, 1, 1, 0, 2, "enUS", None,
            ),
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
            session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            client_visible_transports_like_cpp: Default::default(),
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

fn login_group_session_like_cpp() -> (
    WorldSession,
    u64,
    ObjectGuid,
    flume::Receiver<crate::session::mailbox::SessionCommand>,
    flume::Receiver<Vec<u8>>,
) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<wow_packet::WorldPacket>(1);
    let (session_send_tx, _session_send_rx) = flume::unbounded::<Vec<u8>>();
    let mut session = WorldSession::new(
        1,
        "LoginMember".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "enUS".into(),
        pkt_rx,
        session_send_tx.clone(),
        crate::session::registry::build_dispatch_table(),
    );

    let leader = ObjectGuid::create_player(1, 4_224);
    let current = ObjectGuid::create_player(1, 4_220);
    let remote = ObjectGuid::create_player(1, 4_225);
    let offline = ObjectGuid::create_player(1, 4_227);
    let mut group = GroupInfo::new(leader);
    assert!(group.add_member(current));
    assert!(group.add_member(remote));
    assert!(group.add_member(offline));
    set_group_slot_like_cpp(&mut group, leader, "Leader", 8, 0, 1, 2);
    set_group_slot_like_cpp(&mut group, current, "LoginMember", 2, 0, 2, 4);
    set_group_slot_like_cpp(&mut group, remote, "OnlineMate", 3, 0, 0, 1);
    set_group_slot_like_cpp(&mut group, offline, "OfflineMate", 1, 0, 0, 0);
    group.loot_method = 4;
    group.loot_threshold = 3;
    group.dungeon_difficulty_id = 2;
    group.raid_difficulty_id = 5;
    group.legacy_raid_difficulty_id = 7;
    let group_guid = group.group_guid;
    let group_registry = Arc::new(GroupRegistry::default());
    group_registry.register_group_like_cpp(group_guid, group);

    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.create_world_map(0, 0).map_mut();
        for guid in [current, remote] {
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

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (remote_command_tx, remote_command_rx) = flume::unbounded();
    for (guid, name, command_tx) in [
        (current, "LoginMember", flume::unbounded().0),
        (remote, "OnlineMate", remote_command_tx),
    ] {
        player_registry.register_or_replace(
            guid,
            login_player_registration_like_cpp(name, session_send_tx.clone(), command_tx),
            Default::default(),
        );
    }

    session.set_player_guid(Some(current));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_state(SessionState::Authed);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));

    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([
        human_race_entry_like_cpp(1, 11),
    ])));
    session.set_faction_template_store(Arc::new(FactionTemplateStore::from_entries([
        FactionTemplateEntry {
            id: 11,
            faction: 11,
            flags: 0,
            faction_group: 3,
            friend_group: 0,
            enemy_group: 0,
            enemies: [0; 8],
            friend: [0; 8],
        },
    ])));

    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    (session, group_guid, remote, remote_command_rx, realm_rx)
}

#[test]
fn tap_members_follow_the_group_authority_not_the_owned_snapshot_like_cpp() {
    let (session, group_registry, group_guid, owner, mate) = reader_session_like_cpp();

    assert_eq!(
        wow_world_instances::current_group_member_guids_for_tap_like_cpp(&session, owner),
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
        wow_world_instances::current_group_member_guids_for_tap_like_cpp(&session, owner)
            .is_empty(),
        "a removed member must not keep sharing creature tap rights"
    );
}

#[test]
fn instance_owner_follows_the_group_authority_like_cpp() {
    let (session, group_registry, group_guid, owner, mate) = reader_session_like_cpp();

    assert_eq!(
        wow_world_instances::create_map_instance_owner_guid_like_cpp(&session, 571),
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
        wow_world_instances::create_map_instance_owner_guid_like_cpp(&session, 571),
        Some(owner),
        "a removed member owns its own instance, not the group's"
    );
}

#[test]
fn login_group_publishes_exact_party_roster_to_current_and_online_members_like_cpp() {
    let (mut session, group_guid, remote, remote_command_rx, realm_rx) =
        login_group_session_like_cpp();
    assert_eq!(
        session.next_group_update_sequence_number_like_cpp(0),
        Some(0)
    );

    // The initial map-add hook runs before login publishes LoggedIn.
    assert_eq!(session.state(), SessionState::Authed);
    wow_world_character::CharacterLoginHostLikeCpp::send_initial_group_update_after_login_like_cpp(
        &mut session,
    );

    let direct_packet = realm_rx.try_recv().expect("current login member update");
    let expected_players = vec![
        PartyPlayerInfo {
            guid: ObjectGuid::create_player(1, 4_224),
            name: "Leader".into(),
            class: 8,
            subgroup: 0,
            flags: 1,
            roles_assigned: 2,
            faction_group: 3,
            connected: false,
        },
        PartyPlayerInfo {
            guid: ObjectGuid::create_player(1, 4_220),
            name: "LoginMember".into(),
            class: 2,
            subgroup: 0,
            flags: 2,
            roles_assigned: 4,
            faction_group: 3,
            connected: true,
        },
        PartyPlayerInfo {
            guid: remote,
            name: "OnlineMate".into(),
            class: 3,
            subgroup: 0,
            flags: 0,
            roles_assigned: 1,
            faction_group: 3,
            connected: true,
        },
        PartyPlayerInfo {
            guid: ObjectGuid::create_player(1, 4_227),
            name: "OfflineMate".into(),
            class: 1,
            subgroup: 0,
            flags: 0,
            roles_assigned: 0,
            faction_group: 3,
            connected: false,
        },
    ];
    let expected_direct = PartyUpdate {
        party_flags: 0,
        party_index: 0,
        party_type: 1,
        my_index: 1,
        party_guid: group_guid,
        sequence_num: 1,
        leader_guid: ObjectGuid::create_player(1, 4_224),
        leader_faction_group: 3,
        player_list: expected_players.clone(),
        loot_settings: Some(PartyLootSettings {
            method: 4,
            loot_master: ObjectGuid::EMPTY,
            threshold: 3,
        }),
        difficulty_settings: Some(PartyDifficultySettings {
            dungeon_difficulty_id: 2,
            raid_difficulty_id: 5,
            legacy_raid_difficulty_id: 7,
        }),
    };
    assert_eq!(direct_packet, expected_direct.to_bytes());
    assert!(realm_rx.try_recv().is_err(), "one direct login packet only");

    let remote_command = match remote_command_rx
        .try_recv()
        .expect("online group member queue")
    {
        crate::session::mailbox::SessionCommand::SendPartyUpdateLikeCpp(command) => command,
        other => panic!("unexpected remote command: {other:?}"),
    };
    assert_eq!(remote_command.recipient, remote);
    assert!(remote_command.member_full_state_packets.is_empty());
    let expected_remote = PartyUpdate {
        my_index: 2,
        sequence_num: 0,
        ..expected_direct
    };
    assert_eq!(
        remote_command.party_update.to_bytes(),
        expected_remote.to_bytes()
    );
    assert!(remote_command_rx.try_recv().is_err());
}

#[test]
fn login_group_publication_requires_canonical_player_membership_like_cpp() {
    let (mut session, group_guid, _remote, _remote_command_rx, realm_rx) =
        login_group_session_like_cpp();
    assert!(session.set_owned_player_group_like_cpp(None));

    // The stale registry still contains the GUID, but Player::GetGroup is
    // empty, so the native initial SendUpdateToPlayer path is not entered.
    wow_world_character::CharacterLoginHostLikeCpp::send_initial_group_update_after_login_like_cpp(
        &mut session,
    );
    assert!(realm_rx.try_recv().is_err());
    assert_eq!(
        session
            .core
            .player_group_owner_access_like_cpp()
            .canonical_group_guid_like_cpp(),
        Some(None)
    );

    // A Player snapshot that still points at the group is also insufficient
    // after the group authority has detached it.
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    let registry = session.core.directory.group_registry.as_ref().unwrap();
    registry
        .remove_member_like_cpp(
            group_guid,
            session.player_guid().unwrap(),
            wow_social::group::GroupMemberRemovalKindLikeCpp::Leave,
            &[],
        )
        .expect("remove canonical member from current group authority");
    wow_world_character::CharacterLoginHostLikeCpp::send_initial_group_update_after_login_like_cpp(
        &mut session,
    );
    assert!(realm_rx.try_recv().is_err());

    // The registry still lists the player, but a detached/missing canonical
    // owner cannot supply Player::GetGroup and must also stay silent.
    let (mut ownerless_session, _group_guid, _remote, _commands, ownerless_realm_rx) =
        login_group_session_like_cpp();
    ownerless_session.core.player_handle_like_cpp = None;
    assert_eq!(
        ownerless_session
            .core
            .player_group_owner_access_like_cpp()
            .canonical_group_guid_like_cpp(),
        None
    );
    wow_world_character::CharacterLoginHostLikeCpp::send_initial_group_update_after_login_like_cpp(
        &mut ownerless_session,
    );
    assert!(ownerless_realm_rx.try_recv().is_err());
}

#[test]
fn login_group_queued_update_is_silent_after_receiver_leaves_group_like_cpp() {
    let (mut session, group_registry, group_guid, owner, _mate) = reader_session_like_cpp();
    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    let command = || crate::session::mailbox::SendPartyUpdateLikeCppCommand {
        recipient: owner,
        party_update: PartyUpdate {
            party_flags: 0,
            party_index: 0,
            party_type: 1,
            my_index: 0,
            party_guid: group_guid,
            sequence_num: 0,
            leader_guid: owner,
            leader_faction_group: 3,
            player_list: Vec::new(),
            loot_settings: None,
            difficulty_settings: None,
        },
        member_full_state_packets: Vec::new(),
    };

    wow_world_instances::handle_send_party_update_command_like_cpp(&mut session, command());
    assert!(
        realm_rx.try_recv().is_ok(),
        "current group member receives update"
    );

    group_registry
        .remove_member_like_cpp(
            group_guid,
            owner,
            wow_social::group::GroupMemberRemovalKindLikeCpp::Leave,
            &[],
        )
        .expect("receiver leaves group");
    wow_world_instances::handle_send_party_update_command_like_cpp(&mut session, command());
    assert!(
        realm_rx.try_recv().is_err(),
        "a stale queued update must not reach a member detached from the current group"
    );
}
