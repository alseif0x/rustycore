use super::*;

const MAP_ID: u32 = 571;
const INSTANCE_ID: u32 = 0;

struct Fixture {
    session: crate::session::WorldSession,
    manager: SharedCanonicalMapManager,
    registry: Arc<PlayerRegistry>,
    viewer: ObjectGuid,
    target: ObjectGuid,
    target_handle: wow_map::PlayerHandle,
}

#[test]
fn nearby_player_create_uses_frozen_viewer_aura_state_instead_of_current_health() {
    let fixture = fixture();
    let registration = fixture
        .registry
        .control_address(fixture.target)
        .unwrap()
        .registration();
    let (map_key, revision) = fixture
        .manager
        .lock()
        .unwrap()
        .player_active_residence_revision_like_cpp(fixture.target_handle)
        .unwrap();
    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        player
            .unit_mut()
            .set_unit_flags2_like_cpp(wow_constants::UnitFlags2::from_bits_retain(0x8000_0800));
        let auras = &mut player.unit_mut().subsystems_mut().auras;
        auras.register_applied_aura(
            wow_entities::AppliedAuraRef::new(800, fixture.viewer, 0, 1),
            Some(14),
            0,
            0,
        );
        auras.register_applied_aura(
            wow_entities::AppliedAuraRef::new(801, fixture.target, 1, 1),
            Some(16),
            0,
            0,
        );
    });
    for (viewer, expected) in [
        (fixture.viewer, 1 << 13),
        (fixture.target, 1 << 15),
        (ObjectGuid::EMPTY, 0),
    ] {
        let snapshot = fixture
            .registry
            .player_visibility_create_snapshot_for_candidate(
                registration,
                fixture.target_handle,
                map_key,
                revision,
                viewer,
            )
            .unwrap();
        assert_eq!(snapshot.aura_state, expected);
        assert_eq!(snapshot.unit_flags2, 0x8000_0800);
        let update = wow_world_lifecycle::login_transport::player_visibility_create_update_from_snapshot_like_cpp(&snapshot, MAP_ID as u16);
        let wow_packet::packets::update::UpdateBlock::CreateObject { create_data, .. } =
            &update.blocks[0]
        else {
            panic!("native Player CREATE");
        };
        assert_eq!(create_data.aura_state, expected);
        assert_eq!(create_data.unit_flags2, 0x8000_0800);
    }
    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        player
            .unit_mut()
            .set_unit_flags2_like_cpp(wow_constants::UnitFlags2::from_bits_retain(0x8000_0000));
    });
    let snapshot = fixture
        .registry
        .player_visibility_create_snapshot_for_candidate(
            registration,
            fixture.target_handle,
            map_key,
            revision,
            fixture.viewer,
        )
        .unwrap();
    let update = wow_world_lifecycle::login_transport::player_visibility_create_update_from_snapshot_like_cpp(&snapshot, MAP_ID as u16);
    let wow_packet::packets::update::UpdateBlock::CreateObject { create_data, .. } =
        &update.blocks[0]
    else {
        panic!("native nearby CREATE after aura removal");
    };
    assert_eq!(create_data.unit_flags2, 0x8000_0000);
}

fn fixture() -> Fixture {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let registry = Arc::new(PlayerRegistry::default());
    let viewer = ObjectGuid::create_player(1, 93_100);
    let target = ObjectGuid::create_player(1, 93_101);

    assert!(registry.bind_canonical_map_manager(Arc::clone(&manager)));
    session.set_player_guid(Some(viewer));
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.set_loaded_player_identity_like_cpp(MAP_ID as u16, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::ZERO);
    add_canonical_test_player_on_map(&manager, viewer, Position::ZERO, MAP_ID, INSTANCE_ID);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    set_player_state(
        &manager,
        session.core.player_handle_like_cpp.unwrap(),
        |player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            player
                .unit_mut()
                .set_server_side_ghost_visibility_like_cpp(0x1);
            player
                .unit_mut()
                .set_server_side_ghost_visibility_detect_like_cpp(0x1);
        },
    );

    let target_handle = install_owned_visibility_player(
        &manager,
        target,
        Position::new(10.0, 0.0, 0.0, 0.0),
        MAP_ID,
        INSTANCE_ID,
    );
    set_player_state(&manager, target_handle, |player| {
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(1);
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Dead);
        player
            .unit_mut()
            .set_server_side_ghost_visibility_like_cpp(0x2);
        player
            .unit_mut()
            .set_server_side_ghost_visibility_detect_like_cpp(0x1);
    });
    let (target_tx, _target_rx) = flume::bounded(1);
    register_visibility_player(
        &registry,
        target,
        target_handle,
        Position::new(10.0, 0.0, 0.0, 0.0),
        MAP_ID,
        INSTANCE_ID,
        target_tx,
    );

    Fixture {
        session,
        manager,
        registry,
        viewer,
        target,
        target_handle,
    }
}

fn set_player_state(
    manager: &SharedCanonicalMapManager,
    handle: wow_map::PlayerHandle,
    update: impl FnOnce(&mut wow_entities::Player),
) {
    manager
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(handle, update)
        .expect("current canonical Player");
}

fn set_group(
    fixture: &mut Fixture,
    group_id: u64,
    viewer_subgroup: u8,
    target_subgroup: u8,
    include_target_in_registry: bool,
) {
    let mut group = GroupInfo::new(fixture.viewer);
    assert!(group.add_member(fixture.target));
    group.group_guid = group_id;
    group.member_slots.iter_mut().for_each(|slot| {
        if slot.guid == fixture.viewer {
            slot.subgroup = viewer_subgroup;
        } else if slot.guid == fixture.target {
            slot.subgroup = target_subgroup;
        }
    });
    if !include_target_in_registry {
        group.members.retain(|guid| *guid != fixture.target);
        group
            .member_slots
            .retain(|slot| slot.guid != fixture.target);
    }
    let group_guid = group.group_guid;
    let groups = Arc::new(GroupRegistry::default());
    groups.register_group_like_cpp(group_guid, group);
    fixture
        .session
        .set_group_registry(groups, Arc::new(PendingInvites::default()));

    let group_object_guid = ObjectGuid::create_group(group_guid);
    for (guid, handle, subgroup) in [
        (
            fixture.viewer,
            fixture.session.core.player_handle_like_cpp.unwrap(),
            viewer_subgroup,
        ),
        (fixture.target, fixture.target_handle, target_subgroup),
    ] {
        set_player_state(&fixture.manager, handle, |player| {
            player.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
                group_guid: group_object_guid,
                leader_guid: fixture.viewer,
                role_mask: 0,
                subgroup,
            }));
            if guid == fixture.viewer {
                player.unit_mut().set_race(1);
            }
        });
    }
}

fn visible(fixture: &Fixture) -> bool {
    fixture
        .session
        .visible_other_players_from_registry_like_cpp(
            MAP_ID as u16,
            &Position::ZERO,
            wow_entities::VISIBILITY_RADIUS_LIKE_CPP,
        )
        .iter()
        .any(|(guid, _)| *guid == fixture.target)
}

#[test]
fn group_visibility_default_and_modes_match_cpp() {
    assert_eq!(
        wow_world_core::session::state::SessionWorldConfig::default().group_visibility_mode,
        1
    );

    let mut fixture = fixture();
    set_group(&mut fixture, 50_001, 0, 1, true);
    assert!(
        visible(&fixture),
        "default mode 1 admits other raid subgroups"
    );

    fixture.session.set_group_visibility_mode(0);
    assert!(!visible(&fixture), "mode 0 requires the same subgroup");
    set_group(&mut fixture, 50_002, 0, 0, true);
    assert!(visible(&fixture), "mode 0 admits the same subgroup");

    fixture.session.set_group_visibility_mode(1);
    set_group(&mut fixture, 50_003, 0, 4, true);
    assert!(visible(&fixture), "mode 1 ignores subgroup differences");

    fixture.session.set_group_visibility_mode(3);
    assert!(!visible(&fixture), "mode 3 disables the group bypass");
}

#[test]
fn unknown_and_negative_group_visibility_modes_use_subgroup_policy() {
    let mut fixture = fixture();
    for (index, mode) in [-1, 99].into_iter().enumerate() {
        fixture.session.set_group_visibility_mode(mode);
        set_group(&mut fixture, 50_011 + (index as u64 * 2), 2, 2, true);
        assert!(
            visible(&fixture),
            "mode {mode} uses same-subgroup admission"
        );
        set_group(&mut fixture, 50_012 + (index as u64 * 2), 2, 3, true);
        assert!(!visible(&fixture), "mode {mode} rejects another subgroup");
    }
}

#[test]
fn mode_two_compares_canonical_team_even_when_races_differ() {
    let mut fixture = fixture();
    fixture.session.set_group_visibility_mode(2);
    let viewer_handle = fixture.session.core.player_handle_like_cpp.unwrap();
    set_player_state(&fixture.manager, viewer_handle, |player| {
        player.unit_mut().set_race(1);
    });
    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        player.unit_mut().set_race(2);
    });
    let (viewer_team, target_team) = {
        let manager = fixture.manager.lock().expect("canonical manager");
        (
            manager
                .with_player_like_cpp(viewer_handle, wow_entities::Player::team)
                .expect("viewer"),
            manager
                .with_player_like_cpp(fixture.target_handle, wow_entities::Player::team)
                .expect("target"),
        )
    };
    assert_eq!(viewer_team, target_team);
    assert!(visible(&fixture));
}

#[test]
fn stale_group_membership_and_other_group_do_not_bypass_ghost_gate() {
    let mut fixture = fixture();
    set_group(&mut fixture, 50_021, 0, 0, true);
    fixture.session.set_group_visibility_mode(1);
    assert!(visible(&fixture));

    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        player.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
            group_guid: ObjectGuid::create_group(50_022),
            leader_guid: fixture.target,
            role_mask: 0,
            subgroup: 0,
        }));
    });
    assert!(
        !visible(&fixture),
        "canonical target group must match the viewer"
    );

    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        player.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
            group_guid: ObjectGuid::create_group(50_021),
            leader_guid: fixture.viewer,
            role_mask: 0,
            subgroup: 0,
        }));
    });
    set_group(&mut fixture, 50_021, 0, 0, false);
    assert!(
        !visible(&fixture),
        "registry membership must include both Players"
    );
}

#[test]
fn group_visibility_does_not_bypass_phase_private_or_gm_gates() {
    let mut fixture = fixture();
    set_group(&mut fixture, 50_031, 0, 0, true);
    fixture.session.set_group_visibility_mode(1);
    assert!(visible(&fixture));

    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        *player.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([20]);
    });
    assert!(!visible(&fixture), "group bypass cannot cross a phase gate");

    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        player.unit_mut().world_mut().phase_shift_mut().clear();
        player
            .unit_mut()
            .set_private_object_owner_like_cpp(ObjectGuid::create_player(1, 93_199));
    });
    assert!(
        !visible(&fixture),
        "group bypass cannot cross private-object ownership"
    );

    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        player
            .unit_mut()
            .set_private_object_owner_like_cpp(ObjectGuid::EMPTY);
        player.unit_mut().set_server_side_gm_visibility_like_cpp(1);
    });
    assert!(!visible(&fixture), "group bypass cannot cross the GM gate");

    set_player_state(&fixture.manager, fixture.target_handle, |player| {
        player.unit_mut().set_server_side_gm_visibility_like_cpp(0);
    });
    assert!(visible(&fixture));
}

#[test]
fn corpse_visibility_allows_ghost_player_without_group_bypass() {
    let mut fixture = fixture();
    fixture.session.set_group_visibility_mode(3);
    let viewer_handle = fixture.session.core.player_handle_like_cpp.unwrap();
    set_player_state(&fixture.manager, viewer_handle, |player| {
        player.unit_mut().set_health(1);
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Dead);
        player
            .unit_mut()
            .set_server_side_ghost_visibility_like_cpp(0x1);
        player
            .unit_mut()
            .set_server_side_ghost_visibility_detect_like_cpp(0);
    });
    assert!(
        !visible(&fixture),
        "without a corpse, mode 3 cannot bypass the ghost mask"
    );

    let corpse_guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Corpse,
        0,
        1,
        MAP_ID as u16,
        INSTANCE_ID,
        0,
        93_110,
    );
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse
        .world_mut()
        .set_map(MAP_ID, INSTANCE_ID)
        .expect("corpse map");
    corpse.world_mut().relocate(Position::ZERO);
    corpse.set_owner_guid(fixture.viewer);
    fixture
        .manager
        .lock()
        .expect("canonical manager")
        .find_map_mut(MAP_ID, INSTANCE_ID)
        .expect("canonical map")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_corpse(corpse).unwrap(),
        )
        .expect("insert corpse");

    assert!(visible(&fixture));
}

#[test]
fn directory_presence_and_residence_must_match_canonical_candidate() {
    let mut fixture = fixture();
    set_group(&mut fixture, 50_041, 0, 0, true);
    fixture.session.set_group_visibility_mode(1);
    assert!(visible(&fixture));

    assert!(
        fixture
            .registry
            .fixture_update(fixture.target, |placement| {
                placement.is_in_world = false;
            })
    );
    assert!(
        !visible(&fixture),
        "directory must still report current presence"
    );

    assert!(
        fixture
            .registry
            .fixture_update(fixture.target, |placement| {
                placement.is_in_world = true;
                placement.map_id = 0;
            })
    );
    assert!(
        !visible(&fixture),
        "stale directory map placement cannot create"
    );

    assert!(
        fixture
            .registry
            .fixture_update(fixture.target, |placement| {
                placement.map_id = MAP_ID as u16;
            })
    );
    assert!(visible(&fixture));
}

#[test]
fn directory_binding_rejects_replacement_canonical_incarnation() {
    let mut fixture = fixture();
    set_group(&mut fixture, 50_051, 0, 0, true);
    fixture.session.set_group_visibility_mode(1);
    assert!(visible(&fixture));

    let replacement_handle = {
        let mut manager = fixture.manager.lock().expect("canonical manager");
        let mut replacement = Box::new(wow_entities::Player::new(Some(1), false));
        replacement
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(fixture.target);
        replacement.unit_mut().set_max_health(100);
        replacement.unit_mut().set_health(1);
        replacement
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Dead);
        replacement
            .unit_mut()
            .set_server_side_ghost_visibility_like_cpp(0x2);
        let handle = manager
            .install_detached_player_like_cpp(replacement)
            .expect("replace canonical incarnation");
        manager
            .attach_player_like_cpp(
                handle,
                wow_map::MapKey::new(MAP_ID, INSTANCE_ID),
                Position::new(10.0, 0.0, 0.0, 0.0),
            )
            .expect("attach replacement");
        handle
    };
    set_player_state(&fixture.manager, replacement_handle, |player| {
        player.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
            group_guid: ObjectGuid::create_group(50_051),
            leader_guid: fixture.viewer,
            role_mask: 0,
            subgroup: 0,
        }));
    });

    assert_ne!(replacement_handle, fixture.target_handle);
    assert!(
        !visible(&fixture),
        "the old directory registration cannot project a replacement Player with the same GUID"
    );
}

#[test]
fn stale_viewer_handle_cannot_select_player_candidates() {
    let mut fixture = fixture();
    set_group(&mut fixture, 50_061, 0, 0, true);
    fixture.session.set_group_visibility_mode(1);

    let viewer_handle = fixture.session.core.player_handle_like_cpp.unwrap();
    let replacement_handle = {
        let mut manager = fixture.manager.lock().expect("canonical manager");
        let mut replacement = Box::new(wow_entities::Player::new(Some(1), false));
        replacement
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(fixture.viewer);
        replacement.unit_mut().set_max_health(100);
        replacement.unit_mut().set_health(100);
        manager
            .install_detached_player_like_cpp(replacement)
            .expect("replace viewer incarnation")
    };
    assert_ne!(replacement_handle, viewer_handle);
    assert!(!visible(&fixture));
}

#[test]
fn directory_projection_rejects_old_residence_revision_after_same_map_reattach() {
    let fixture = fixture();
    let registration = fixture
        .registry
        .control_address(fixture.target)
        .expect("current target registration")
        .registration();
    let (map_key, old_revision) = fixture
        .manager
        .lock()
        .expect("canonical manager")
        .player_active_residence_revision_like_cpp(fixture.target_handle)
        .expect("active target residence");

    {
        let mut manager = fixture.manager.lock().expect("canonical manager");
        manager
            .detach_player_like_cpp(fixture.target_handle)
            .expect("detach target");
        manager
            .attach_player_like_cpp(
                fixture.target_handle,
                map_key,
                Position::new(10.0, 0.0, 0.0, 0.0),
            )
            .expect("reattach target to the same map");
    }
    let (current_key, current_revision) = fixture
        .manager
        .lock()
        .expect("canonical manager")
        .player_active_residence_revision_like_cpp(fixture.target_handle)
        .expect("reattached target residence");
    assert_eq!(current_key, map_key);
    assert_ne!(current_revision, old_revision);

    assert!(
        fixture
            .registry
            .player_visibility_create_snapshot_for_candidate(
                registration,
                fixture.target_handle,
                map_key,
                old_revision,
                fixture.viewer,
            )
            .is_none()
    );
    assert!(
        fixture
            .registry
            .player_visibility_create_snapshot_for_candidate(
                registration,
                fixture.target_handle,
                current_key,
                current_revision,
                fixture.viewer,
            )
            .is_some()
    );
}

#[test]
fn directory_projection_rejects_old_candidate_after_same_guid_replacement_and_reregister() {
    let fixture = fixture();
    let old_registration = fixture
        .registry
        .control_address(fixture.target)
        .expect("original target registration")
        .registration();
    let (old_map, old_revision) = fixture
        .manager
        .lock()
        .expect("canonical manager")
        .player_active_residence_revision_like_cpp(fixture.target_handle)
        .expect("original target residence");
    let new_handle = {
        let mut manager = fixture.manager.lock().expect("canonical manager");
        let mut replacement = Box::new(wow_entities::Player::new(Some(1), false));
        replacement
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(fixture.target);
        replacement.unit_mut().set_max_health(100);
        replacement.unit_mut().set_health(100);
        let handle = manager
            .install_detached_player_like_cpp(replacement)
            .expect("replace target incarnation");
        manager
            .attach_player_like_cpp(
                handle,
                wow_map::MapKey::new(MAP_ID, INSTANCE_ID),
                Position::new(10.0, 0.0, 0.0, 0.0),
            )
            .expect("attach replacement");
        handle
    };
    let (new_map, new_revision) = fixture
        .manager
        .lock()
        .expect("canonical manager")
        .player_active_residence_revision_like_cpp(new_handle)
        .expect("replacement residence");
    let (send_tx, _send_rx) = flume::bounded(1);
    let new_registration = register_visibility_player(
        &fixture.registry,
        fixture.target,
        new_handle,
        Position::new(10.0, 0.0, 0.0, 0.0),
        MAP_ID,
        INSTANCE_ID,
        send_tx,
    );

    assert_ne!(old_registration, new_registration);
    assert_ne!(fixture.target_handle, new_handle);
    assert!(
        fixture
            .registry
            .player_visibility_create_snapshot_for_candidate(
                old_registration,
                fixture.target_handle,
                old_map,
                old_revision,
                fixture.viewer,
            )
            .is_none()
    );
    assert!(
        fixture
            .registry
            .player_visibility_create_snapshot_for_candidate(
                new_registration,
                fixture.target_handle,
                old_map,
                old_revision,
                fixture.viewer,
            )
            .is_none()
    );
    assert!(
        fixture
            .registry
            .player_visibility_create_snapshot_for_candidate(
                new_registration,
                new_handle,
                new_map,
                new_revision,
                fixture.viewer,
            )
            .is_some()
    );
}
