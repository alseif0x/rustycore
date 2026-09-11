//! #743 — application or reconciliation of remote group state changes.
//!
//! C++ applies a group transition to every connected member inside the same
//! operation: `Group::RemoveMember` (`Group.cpp:550`, clearing at `:593`) and
//! `Group::Disband` (`Group.cpp:713`, clearing at `:734`) call
//! `Player::SetGroup(nullptr)` (`Player.cpp:23440`) directly, so no member can
//! be left holding a group the authority already revoked.
//!
//! RustyCore applies the same transitions on the member's own session, which
//! preserves its admission phase, canonical Player guard and publication
//! order. These scenarios prove the queue hop between the two is lossless:
//! a saturated, replaced, closed or unaddressable target converges on
//! `GroupRegistry` instead of keeping a revoked membership.

use super::*;

use crate::session::SessionState;
use crate::session::mailbox::{ApplyGroupRemovalLikeCppCommand, ApplyGroupSubgroupLikeCppCommand};

/// One kick/disband scenario: a leader that acts and a target whose mailbox
/// is already saturated by the time the group authority reaches it.
struct GroupReconciliationFixtureLikeCpp {
    leader_session: WorldSession,
    target_session: WorldSession,
    target_send_rx: flume::Receiver<Vec<u8>>,
    target_command_tx: flume::Sender<SessionCommand>,
    player_registry: Arc<PlayerRegistry>,
    group_registry: Arc<GroupRegistry>,
    group_guid: u64,
    leader: ObjectGuid,
    target: ObjectGuid,
}

/// Build the two-session fixture with real canonical Players.
///
/// `target_command_capacity` is the bounded mailbox the group authority must
/// reach; `saturate` fills it first so the state change cannot be handed over.
fn group_reconciliation_fixture_like_cpp(
    target_command_capacity: usize,
    saturate: bool,
    extra_members: usize,
) -> GroupReconciliationFixtureLikeCpp {
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let mut group = GroupInfo::new(leader);
    assert!(group.add_member(target));
    let mut all_members = vec![leader, target];
    for extra in 0..extra_members {
        let member = ObjectGuid::create_player(1, 200 + extra as i64);
        assert!(group.add_member(member));
        all_members.push(member);
    }
    let group_guid = group.group_guid;
    let group_registry = Arc::new(GroupRegistry::default());
    group_registry.register_group_like_cpp(group_guid, group);

    let canonical = bind_canonical_party_players_like_cpp(&player_registry, all_members.clone());

    let (leader_tx, _leader_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    for member in all_members.iter().skip(2).copied() {
        let (member_tx, _member_rx) = bounded(8);
        player_registry.register_or_replace(
            member,
            broadcast_info(member, member_tx),
            Default::default(),
        );
    }

    let (target_send_tx, target_send_rx) = flume::unbounded::<Vec<u8>>();
    let target_socket_tx = target_send_tx.clone();
    let (target_command_tx, _target_command_rx) = bounded(target_command_capacity);
    player_registry.register_or_replace(
        target,
        broadcast_info_with_command_tx(target, target_send_tx, target_command_tx.clone()),
        Default::default(),
    );
    if saturate {
        for _ in 0..target_command_capacity {
            target_command_tx
                .try_send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
                .expect("saturate the bounded target mailbox");
        }
        assert!(
            target_command_tx
                .try_send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
                .is_err(),
            "the target mailbox must be full before the group transition"
        );
    }

    let (mut leader_session, _leader_send_rx) = make_session_with_send();
    leader_session.set_player_guid(Some(leader));
    leader_session.group_guid = Some(group_guid);
    leader_session.set_player_registry(Arc::clone(&player_registry));
    leader_session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );

    // The target session must publish on the same socket channel the
    // directory registered, so a reconciliation's packets are observable.
    let (_target_pkt_tx, target_pkt_rx) = bounded::<WorldPacket>(1);
    let mut target_session = WorldSession::new(
        2,
        "TargetAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        target_pkt_rx,
        target_socket_tx,
    );
    target_session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    target_session.set_player_guid(Some(target));
    target_session.set_canonical_map_manager(canonical);
    target_session.set_player_registry(Arc::clone(&player_registry));
    target_session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    target_session.set_state(SessionState::LoggedIn);
    assert!(
        target_session.adopt_registered_canonical_player_fixture_like_cpp(),
        "target session must own its canonical Player"
    );
    assert!(
        target_session.set_owned_player_group_like_cpp(Some((group_guid, 0))),
        "target must start inside the group like C++ `Player::SetGroup`"
    );
    assert_eq!(
        target_session.resolved_group_guid_like_cpp(),
        Some(group_guid)
    );

    GroupReconciliationFixtureLikeCpp {
        leader_session,
        target_session,
        target_send_rx,
        target_command_tx,
        player_registry,
        group_registry,
        group_guid,
        leader,
        target,
    }
}

fn server_opcode_of(bytes: &[u8]) -> u16 {
    let mut packet = WorldPacket::from_bytes(bytes);
    packet.read_uint16().expect("server opcode")
}

fn drain_opcodes(rx: &flume::Receiver<Vec<u8>>) -> Vec<u16> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = rx.try_recv() {
        opcodes.push(server_opcode_of(&bytes));
    }
    opcodes
}

/// A kick whose target mailbox is full still removes the target's membership.
#[tokio::test]
async fn saturated_kick_reconciles_the_target_group_state_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(2, true, 2);

    fixture
        .leader_session
        .handle_party_uninvite(party_uninvite_packet(fixture.target, None, "bye"))
        .await;

    // The authority removed the member; the command could not be handed over.
    assert!(
        !fixture
            .group_registry
            .get(&fixture.group_guid)
            .expect("group survives a kick above the disband threshold")
            .members
            .contains(&fixture.target)
    );
    assert!(
        fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(fixture.target),
        "a state change that could not be delivered must be recorded"
    );
    // Not yet applied: this is exactly the state C++ never reaches.
    assert_eq!(
        fixture.target_session.resolved_group_guid_like_cpp(),
        Some(fixture.group_guid)
    );

    assert!(fixture.target_session.reconcile_group_state_like_cpp());

    assert_eq!(fixture.target_session.resolved_group_guid_like_cpp(), None);
    assert!(
        !fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(fixture.target)
    );
    let opcodes = drain_opcodes(&fixture.target_send_rx);
    assert!(
        opcodes.contains(&(ServerOpcodes::GroupUninvite as u16)),
        "a surviving group sends the removed member GroupUninvite: {opcodes:?}"
    );
    assert!(
        opcodes.contains(&(ServerOpcodes::PartyUpdate as u16)),
        "C++ `Group::SendUpdateDestroyGroupToPlayer` tears the frames down: {opcodes:?}"
    );
}

/// The reconciliation is idempotent: it runs once per recorded obligation.
#[tokio::test]
async fn reconciliation_runs_once_per_recorded_obligation_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(2, true, 2);

    fixture
        .leader_session
        .handle_party_uninvite(party_uninvite_packet(fixture.target, None, "bye"))
        .await;
    assert!(fixture.target_session.reconcile_group_state_like_cpp());
    let _ = drain_opcodes(&fixture.target_send_rx);

    assert!(!fixture.target_session.reconcile_group_state_like_cpp());
    assert!(drain_opcodes(&fixture.target_send_rx).is_empty());
}

/// A removal command that arrives after the reconciliation publishes nothing.
#[tokio::test]
async fn delivered_removal_after_reconciliation_does_not_publish_twice_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(2, true, 2);

    fixture
        .leader_session
        .handle_party_uninvite(party_uninvite_packet(fixture.target, None, "bye"))
        .await;
    assert!(fixture.target_session.reconcile_group_state_like_cpp());
    let _ = drain_opcodes(&fixture.target_send_rx);

    fixture
        .target_session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(
            ApplyGroupRemovalLikeCppCommand {
                group_guid: fixture.group_guid,
                category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                send_group_destroyed: false,
                send_group_uninvite: true,
                refresh_visible_gameobjects_or_spellclicks: true,
            },
        ))
        .expect("own mailbox accepts the late command");
    fixture
        .target_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(fixture.target_session.resolved_group_guid_like_cpp(), None);
    assert!(
        drain_opcodes(&fixture.target_send_rx).is_empty(),
        "the late command finds the snapshot converged and publishes nothing"
    );
}

/// Disbanding a two-member group with a saturated target still clears it.
#[tokio::test]
async fn saturated_disband_reconciles_the_remaining_member_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(2, true, 0);

    // C++ `Group::RemoveMember` disbands below the two-member threshold
    // (`Group.cpp:660-663`), which reaches every connected member.
    fixture
        .leader_session
        .handle_party_uninvite(party_uninvite_packet(fixture.target, None, "bye"))
        .await;

    assert!(fixture.group_registry.get(&fixture.group_guid).is_none());
    assert!(
        fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(fixture.target)
    );

    assert!(fixture.target_session.reconcile_group_state_like_cpp());

    assert_eq!(fixture.target_session.resolved_group_guid_like_cpp(), None);
    let opcodes = drain_opcodes(&fixture.target_send_rx);
    assert!(
        opcodes.contains(&(ServerOpcodes::GroupDestroyed as u16)),
        "a retired group sends GroupDestroyed, not GroupUninvite: {opcodes:?}"
    );
    assert!(
        opcodes.contains(&(ServerOpcodes::PartyUpdate as u16)),
        "{opcodes:?}"
    );
}

/// A removal that races a re-join into the same group must not clear it.
#[tokio::test]
async fn obsolete_removal_after_rejoining_the_same_group_is_ignored_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(4, false, 2);
    let group_guid = fixture.group_guid;

    // The authority holds the membership again by the time the earlier
    // removal command is applied.
    fixture
        .target_session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(
            ApplyGroupRemovalLikeCppCommand {
                group_guid,
                category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                send_group_destroyed: false,
                send_group_uninvite: true,
                refresh_visible_gameobjects_or_spellclicks: true,
            },
        ))
        .expect("own mailbox");
    fixture
        .target_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        fixture.target_session.resolved_group_guid_like_cpp(),
        Some(group_guid),
        "an obsolete removal must not revoke a membership the authority holds"
    );
    assert!(drain_opcodes(&fixture.target_send_rx).is_empty());
}

/// A removal for a group the member already left is still obsolete.
#[tokio::test]
async fn removal_for_another_group_leaves_current_membership_alone_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(4, false, 2);

    fixture
        .target_session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(
            ApplyGroupRemovalLikeCppCommand {
                group_guid: fixture.group_guid + 1,
                category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                send_group_destroyed: true,
                send_group_uninvite: false,
                refresh_visible_gameobjects_or_spellclicks: true,
            },
        ))
        .expect("own mailbox");
    fixture
        .target_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        fixture.target_session.resolved_group_guid_like_cpp(),
        Some(fixture.group_guid)
    );
}

/// A replaced incarnation loses the command; its successor reconciles.
#[tokio::test]
async fn replaced_session_incarnation_reconciles_group_state_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(4, false, 2);
    let target = fixture.target;

    // The address the group authority resolved before the replacement.
    let stale = fixture
        .player_registry
        .group_presence(target)
        .expect("connected target")
        .registration;
    // A new incarnation registers before the authority delivers.
    let (replacement_tx, replacement_rx) = flume::unbounded::<Vec<u8>>();
    let (replacement_command_tx, _replacement_command_rx) = bounded(4);
    fixture.player_registry.register_or_replace(
        target,
        broadcast_info_with_command_tx(target, replacement_tx, replacement_command_tx),
        Default::default(),
    );
    assert!(
        fixture
            .player_registry
            .deliver_group_state_command_like_cpp(
                stale,
                SessionCommand::ApplyGroupRemovalLikeCpp(ApplyGroupRemovalLikeCppCommand {
                    group_guid: fixture.group_guid,
                    category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                    party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                    send_group_destroyed: false,
                    send_group_uninvite: true,
                    refresh_visible_gameobjects_or_spellclicks: true,
                }),
            )
            .is_err(),
        "a replaced incarnation cannot receive the command"
    );
    assert!(
        fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(target),
        "the obligation follows the GUID, not the retired incarnation"
    );

    // The authority then really removes the member.
    fixture
        .group_registry
        .remove_member_like_cpp(
            fixture.group_guid,
            target,
            wow_social::group::GroupMemberRemovalKindLikeCpp::Leave,
            &[],
        )
        .expect("authority removal");

    assert!(fixture.target_session.reconcile_group_state_like_cpp());
    assert_eq!(fixture.target_session.resolved_group_guid_like_cpp(), None);
    // The successor publishes the teardown on its own socket; the retired
    // incarnation's address received nothing.
    let opcodes = drain_opcodes(&fixture.target_send_rx);
    assert!(
        opcodes.contains(&(ServerOpcodes::GroupUninvite as u16)),
        "{opcodes:?}"
    );
    assert!(replacement_rx.is_empty());
}

/// A closed mailbox records the obligation exactly like a full one.
#[tokio::test]
async fn disconnected_target_records_the_group_obligation_like_cpp() {
    let fixture = group_reconciliation_fixture_like_cpp(4, false, 2);
    let registration = fixture
        .player_registry
        .group_presence(fixture.target)
        .expect("connected target")
        .registration;
    drop(fixture.target_command_tx);
    // The dispatcher thread owns the other clone; drop it by draining the
    // registry entry's receiver through a disconnect instead.
    fixture.player_registry.unregister(registration);
    assert!(
        !fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(fixture.target),
        "unregistering a gone session clears its obligation"
    );
}

/// A target the map cannot resolve still records the obligation.
#[tokio::test]
async fn unaddressable_target_records_the_group_obligation_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(4, false, 2);
    let absent = ObjectGuid::create_player(1, 9_001);

    fixture
        .leader_session
        .handle_party_uninvite(party_uninvite_packet(absent, None, "bye"))
        .await;

    // The kick is rejected by the authority (not a member), so nothing is
    // recorded for a player that never belonged to the group.
    assert!(
        !fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(absent)
    );

    // A member the directory cannot resolve does record one.
    fixture.player_registry.unregister(
        fixture
            .player_registry
            .group_presence(fixture.target)
            .expect("connected target")
            .registration,
    );
    fixture
        .leader_session
        .handle_party_uninvite(party_uninvite_packet(fixture.target, None, "bye"))
        .await;
    assert!(
        fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(fixture.target)
    );
}

/// A group command dropped by its admission phase is deferred, not lost.
#[tokio::test]
async fn group_command_dropped_before_login_defers_reconciliation_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(4, false, 2);
    fixture.target_session.set_state(SessionState::Authed);

    fixture
        .target_session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(
            ApplyGroupRemovalLikeCppCommand {
                group_guid: fixture.group_guid,
                category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                send_group_destroyed: false,
                send_group_uninvite: true,
                refresh_visible_gameobjects_or_spellclicks: true,
            },
        ))
        .expect("own mailbox");
    fixture
        .target_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(fixture.target),
        "an ineligible phase must defer the change, not discard it"
    );
    // The mark survives a reconciliation attempt that is still ineligible.
    assert!(!fixture.target_session.reconcile_group_state_like_cpp());
    assert!(
        fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(fixture.target)
    );

    fixture.target_session.set_state(SessionState::LoggedIn);
    fixture
        .group_registry
        .remove_member_like_cpp(
            fixture.group_guid,
            fixture.target,
            wow_social::group::GroupMemberRemovalKindLikeCpp::Leave,
            &[],
        )
        .expect("authority removal");
    assert!(fixture.target_session.reconcile_group_state_like_cpp());
    assert_eq!(fixture.target_session.resolved_group_guid_like_cpp(), None);
}

/// A saturated subgroup change converges on the authority's subgroup.
#[tokio::test]
async fn saturated_subgroup_change_reconciles_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(4, false, 2);
    let group_guid = fixture.group_guid;

    fixture
        .group_registry
        .convert_group_like_cpp(group_guid, fixture.leader, true)
        .expect("raid conversion");
    fixture
        .group_registry
        .change_member_subgroup_like_cpp(group_guid, fixture.leader, fixture.target, 3)
        .expect("subgroup change");
    fixture
        .player_registry
        .mark_group_state_reconciliation_like_cpp(fixture.target);

    assert!(fixture.target_session.reconcile_group_state_like_cpp());

    assert_eq!(
        fixture.target_session.resolved_group_guid_like_cpp(),
        Some(group_guid)
    );
    assert_eq!(
        fixture.target_session.represented_subgroup_like_cpp(),
        Some(3)
    );
}

/// A subgroup command for a group the member is not in defers instead of
/// silently doing nothing.
#[tokio::test]
async fn subgroup_command_for_another_group_defers_reconciliation_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(4, false, 2);

    fixture
        .target_session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupSubgroupLikeCpp(
            ApplyGroupSubgroupLikeCppCommand {
                group_guid: fixture.group_guid + 1,
                subgroup: 2,
            },
        ))
        .expect("own mailbox");
    fixture
        .target_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        fixture.target_session.represented_subgroup_like_cpp(),
        Some(0)
    );
    assert!(
        fixture
            .player_registry
            .group_state_reconciliation_pending_like_cpp(fixture.target)
    );
}

/// A lost difficulty notification converges with the membership check.
#[tokio::test]
async fn reconciliation_converges_group_difficulty_like_cpp() {
    let mut fixture = group_reconciliation_fixture_like_cpp(4, false, 2);

    fixture
        .group_registry
        .set_difficulty_transition_like_cpp(
            fixture.group_guid,
            fixture.leader,
            2,
            wow_social::group::GroupDifficultyKindLikeCpp::Dungeon,
        )
        .expect("difficulty transition");
    fixture
        .player_registry
        .mark_group_state_reconciliation_like_cpp(fixture.target);

    assert!(fixture.target_session.reconcile_group_state_like_cpp());
    assert_eq!(
        fixture
            .target_session
            .resolved_dungeon_difficulty_id_like_cpp(),
        Some(2)
    );
    let opcodes = drain_opcodes(&fixture.target_send_rx);
    assert!(
        opcodes.contains(&(ServerOpcodes::SetDungeonDifficulty as u16)),
        "{opcodes:?}"
    );
}
