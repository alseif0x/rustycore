//! Production AuraUpdate observer selection and committed delivery.

use super::*;
use crate::session::PlayerRegistry;
use crate::session::mailbox::SendPlayerAuraUpdateCommand;
use std::sync::Arc;

struct Observer {
    session: crate::session::WorldSession,
    output: flume::Receiver<Vec<u8>>,
}

fn setup_registry(fixture: &mut GhostFixture) -> Arc<PlayerRegistry> {
    let registry = Arc::new(PlayerRegistry::new());
    fixture.session.set_player_registry(Arc::clone(&registry));
    fixture.session.core.state = crate::session::SessionState::LoggedIn;
    fixture.session.register_in_player_registry();
    registry
}

fn add_observer(
    fixture: &GhostFixture,
    registry: &Arc<PlayerRegistry>,
    counter: i64,
    position: Position,
    phase: Option<u32>,
    visible_at_commit: bool,
) -> Observer {
    let (mut session, _input, output) = make_session();
    let guid = ObjectGuid::create_player(1, counter);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&fixture.canonical));
    session.set_player_registry(Arc::clone(registry));

    let handle = {
        let mut manager = fixture.canonical.lock().expect("canonical manager");
        manager.create_world_map(571, 0);
        let mut player = Box::new(wow_entities::Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(guid);
        player
            .unit_mut()
            .world_mut()
            .set_name(format!("Observer{counter}"));
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        let handle = manager
            .install_detached_player_like_cpp(player)
            .expect("install observer canonical owner");
        manager
            .attach_player_like_cpp(handle, wow_map::MapKey::new(571, 0), position)
            .expect("attach indexed observer");
        if let Some(phase) = phase {
            manager
                .with_player_mut_like_cpp(handle, |player| {
                    player
                        .unit_mut()
                        .world_mut()
                        .phase_shift_mut()
                        .insert(phase);
                })
                .expect("observer owner after attach");
        }
        handle
    };
    session.core.player_handle_like_cpp = Some(handle);
    session.core.state = crate::session::SessionState::LoggedIn;
    if visible_at_commit {
        session
            .core
            .client_visible_guids_like_cpp
            .insert(fixture.player);
    }
    session.register_in_player_registry();
    Observer { session, output }
}

pub(super) fn aura_packets(packets: impl IntoIterator<Item = Vec<u8>>) -> Vec<Vec<u8>> {
    packets
        .into_iter()
        .filter(|bytes| opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .collect()
}

async fn flush_observer(observer: &mut Observer) {
    observer
        .session
        .process_represented_session_commands_like_cpp()
        .await;
}

#[tokio::test]
async fn release_ghost_and_wisp_delta_and_removal_reach_the_committed_observer() {
    let mut fixture = fixture();
    let registry = setup_registry(&mut fixture);
    let mut observer = add_observer(
        &fixture,
        &registry,
        4_630_001,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        true,
    );

    let mut delivered = Vec::new();
    for spell_id in [WISP, GHOST] {
        cast_selected(&mut fixture, spell_id, None).await;
        let _ = outgoing(&fixture);
        wow_world_spell::tick_auras(&mut fixture.session);
        let source_delta = aura_packets(outgoing(&fixture));
        assert_eq!(
            source_delta.len(),
            1,
            "source self AuraUpdate precedes enqueue"
        );
        flush_observer(&mut observer).await;
        let observer_delta = aura_packets(observer.output.try_iter().collect::<Vec<_>>());
        assert_eq!(observer_delta, source_delta);
        delivered.extend(observer_delta);

        fixture
            .session
            .remove_aura(0)
            .expect("remove selected Aura");
        let source_remove = aura_packets(outgoing(&fixture));
        assert_eq!(source_remove.len(), 1, "removal sends one self AuraUpdate");
        flush_observer(&mut observer).await;
        let observer_remove = aura_packets(observer.output.try_iter().collect::<Vec<_>>());
        assert_eq!(observer_remove, source_remove);
        delivered.extend(observer_remove);
    }
    assert_eq!(delivered.len(), 4);
}

#[tokio::test]
async fn selection_rejects_wrong_phase_range_and_have_at_client_at_publication() {
    let mut fixture = fixture();
    let registry = setup_registry(&mut fixture);
    let mut wrong_phase = add_observer(
        &fixture,
        &registry,
        4_630_002,
        Position::new(20.0, 0.0, 0.0, 0.0),
        Some(22),
        true,
    );
    let mut out_of_range = add_observer(
        &fixture,
        &registry,
        4_630_003,
        Position::new(150.0, 0.0, 0.0, 0.0),
        None,
        true,
    );
    let mut hidden_at_commit = add_observer(
        &fixture,
        &registry,
        4_630_004,
        Position::new(21.0, 0.0, 0.0, 0.0),
        None,
        false,
    );

    cast_selected(&mut fixture, WISP, None).await;
    let _ = outgoing(&fixture);
    wow_world_spell::tick_auras(&mut fixture.session);
    assert_eq!(aura_packets(outgoing(&fixture)).len(), 1);

    flush_observer(&mut wrong_phase).await;
    flush_observer(&mut out_of_range).await;
    flush_observer(&mut hidden_at_commit).await;
    assert!(aura_packets(wrong_phase.output.try_iter().collect::<Vec<_>>()).is_empty());
    assert!(aura_packets(out_of_range.output.try_iter().collect::<Vec<_>>()).is_empty());
    assert!(aura_packets(hidden_at_commit.output.try_iter().collect::<Vec<_>>()).is_empty());
}

#[tokio::test]
async fn detached_canonical_source_keeps_self_aura_update_without_observer_fanout() {
    let mut fixture = fixture();
    let registry = setup_registry(&mut fixture);
    let mut observer = add_observer(
        &fixture,
        &registry,
        4_630_009,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        true,
    );

    cast_selected(&mut fixture, WISP, None).await;
    let _ = outgoing(&fixture);
    fixture
        .canonical
        .lock()
        .expect("canonical manager")
        .detach_player_like_cpp(fixture.session.core.player_handle_like_cpp.unwrap())
        .expect("source detached before selected AuraUpdate");
    wow_world_spell::tick_auras(&mut fixture.session);

    assert_eq!(aura_packets(outgoing(&fixture)).len(), 1);
    flush_observer(&mut observer).await;
    assert!(aura_packets(observer.output.try_iter().collect::<Vec<_>>()).is_empty());
}

#[tokio::test]
async fn unrelated_aura_delta_stays_on_the_existing_self_packet_path() {
    let mut fixture = fixture();
    let registry = setup_registry(&mut fixture);
    let mut observer = add_observer(
        &fixture,
        &registry,
        4_630_010,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        true,
    );

    wow_world_spell::send_aura_update_applied(
        &fixture.session,
        99_013,
        0,
        fixture.player,
        1_000,
        1,
        1,
    );
    assert_eq!(aura_packets(outgoing(&fixture)).len(), 1);
    flush_observer(&mut observer).await;
    assert!(aura_packets(observer.output.try_iter().collect::<Vec<_>>()).is_empty());
}

#[tokio::test]
async fn committed_observer_survives_visibility_and_source_changes_but_new_viewer_does_not() {
    let mut fixture = fixture();
    let registry = setup_registry(&mut fixture);
    let mut committed = add_observer(
        &fixture,
        &registry,
        4_630_005,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        true,
    );
    let mut newly_visible = add_observer(
        &fixture,
        &registry,
        4_630_006,
        Position::new(22.0, 0.0, 0.0, 0.0),
        None,
        false,
    );

    cast_selected(&mut fixture, WISP, None).await;
    let _ = outgoing(&fixture);
    wow_world_spell::tick_auras(&mut fixture.session);
    let source_packets = aura_packets(outgoing(&fixture));
    assert_eq!(source_packets.len(), 1);

    committed
        .session
        .core
        .client_visible_guids_like_cpp
        .remove(&fixture.player);
    newly_visible
        .session
        .core
        .client_visible_guids_like_cpp
        .insert(fixture.player);
    fixture
        .canonical
        .lock()
        .expect("canonical manager")
        .detach_player_like_cpp(fixture.session.core.player_handle_like_cpp.unwrap())
        .expect("source detached after packet commit");

    flush_observer(&mut committed).await;
    flush_observer(&mut newly_visible).await;
    assert_eq!(
        aura_packets(committed.output.try_iter().collect::<Vec<_>>()),
        source_packets,
        "the publication-time recipient receives its committed frame after later changes"
    );
    assert!(
        aura_packets(newly_visible.output.try_iter().collect::<Vec<_>>()).is_empty(),
        "later visibility does not add a recipient retroactively"
    );
}

#[tokio::test]
async fn coordinated_world_tick_leaves_selected_delta_for_map_owned_delivery() {
    let mut fixture = fixture();
    super::selected_lifetime::install_race(&mut fixture.session, 0);
    let registry = setup_registry(&mut fixture);
    let mut observer = add_observer(
        &fixture,
        &registry,
        4_630_011,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        true,
    );
    cast_selected(&mut fixture, WISP, None).await;
    let _ = outgoing(&fixture);

    fixture.session.mark_map_phase_coordinated_like_cpp();
    fixture.session.update(1).await;
    assert_eq!(
        fixture.session.core.state,
        crate::session::SessionState::LoggedIn,
        "the fixture keeps its packet sender live through the coordinated update"
    );
    assert!(aura_packets(outgoing(&fixture)).is_empty());
    assert!(
        fixture
            .canonical
            .lock()
            .expect("canonical manager")
            .with_player_like_cpp(
                fixture.session.core.player_handle_like_cpp.unwrap(),
                |player| player
                    .unit()
                    .subsystems()
                    .auras
                    .visible_auras_to_update
                    .contains(&0),
            )
            .unwrap()
    );

    let updates = {
        let catalogs = wow_world_spell::SelectedAuraPublicationCatalogs {
            races: fixture.session.catalogs.chr.races_store.as_deref(),
            spells: fixture
                .session
                .catalogs
                .spell_catalogs
                .spell_store
                .as_deref(),
            aura_options: fixture
                .session
                .catalogs
                .spell_catalogs
                .spell_aura_options_store
                .as_deref(),
            difficulties: fixture.session.catalogs.difficulty_store.as_deref(),
        };
        let mut manager = fixture.canonical.lock().expect("canonical manager");
        wow_world_spell::collect_player_pending_selected_aura_updates(
            &mut manager,
            fixture.session.core.player_handle_like_cpp.unwrap(),
            catalogs,
        )
        .expect("current Player's selected aura deltas prepare")
    };
    assert_eq!(updates.len(), 1);
    let expected = updates[0].packet_bytes().to_vec();

    // Source self delivery uses the selected command envelope without a
    // HaveAtClient membership test; observer visibility remains committed.
    fixture
        .session
        .core
        .client_visible_guids_like_cpp
        .remove(&fixture.player);
    let source_handle = fixture.session.core.player_handle_like_cpp.unwrap();
    assert!(
        registry
            .selected_aura_recipient(fixture.player, source_handle, wow_map::MapKey::new(571, 0),)
            .is_some()
    );
    updates[0].deliver_to_registry(&registry);
    fixture
        .session
        .process_represented_session_commands_like_cpp()
        .await;
    flush_observer(&mut observer).await;

    let source_packets = aura_packets(outgoing(&fixture));
    let observer_packets = aura_packets(observer.output.try_iter().collect::<Vec<_>>());
    assert_eq!(
        source_packets,
        vec![expected.clone()],
        "source self command"
    );
    assert_eq!(
        observer_packets,
        vec![expected],
        "committed observer command"
    );
    assert!(
        fixture
            .canonical
            .lock()
            .expect("canonical manager")
            .with_player_like_cpp(
                fixture.session.core.player_handle_like_cpp.unwrap(),
                |player| player
                    .unit()
                    .subsystems()
                    .auras
                    .visible_auras_to_update
                    .is_empty(),
            )
            .unwrap()
    );
}

fn command_for(observer: &Observer, packet_bytes: Vec<u8>) -> SendPlayerAuraUpdateCommand {
    let handle = observer.session.core.player_handle_like_cpp.unwrap();
    let (map_key, residence_revision) = observer
        .session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .player_active_residence_revision_like_cpp(handle)
        .expect("active receiver residence");
    SendPlayerAuraUpdateCommand {
        recipient_handle: handle,
        map_key,
        residence_revision,
        committed_visibility: observer.session.core.client_visible_guids_like_cpp.clone(),
        packet_bytes,
    }
}

#[test]
fn receiver_fences_opcode_player_identity_incarnation_and_away_back_residence() {
    let mut fixture = fixture();
    let registry = setup_registry(&mut fixture);
    let mut observer = add_observer(
        &fixture,
        &registry,
        4_630_007,
        Position::new(20.0, 0.0, 0.0, 0.0),
        None,
        true,
    );
    let bytes = (ServerOpcodes::AuraUpdate as u16)
        .to_le_bytes()
        .into_iter()
        .chain([0xA5])
        .collect::<Vec<_>>();

    let wrong_opcode = command_for(&observer, vec![0x11, 0x22]);
    wow_world_spell::handle_player_aura_update(&mut observer.session, wrong_opcode);
    assert!(observer.output.try_iter().next().is_none());

    let mut wrong_visibility = command_for(&observer, bytes.clone());
    wrong_visibility.committed_visibility = Default::default();
    wow_world_spell::handle_player_aura_update(&mut observer.session, wrong_visibility);
    assert!(observer.output.try_iter().next().is_none());

    let not_logged_in = command_for(&observer, bytes.clone());
    observer.session.core.state = crate::session::SessionState::Authed;
    wow_world_spell::handle_player_aura_update(&mut observer.session, not_logged_in);
    assert!(observer.output.try_iter().next().is_none());
    observer.session.core.state = crate::session::SessionState::LoggedIn;

    let committed = command_for(&observer, bytes.clone());
    observer
        .session
        .set_player_guid(Some(ObjectGuid::create_player(1, 4_630_008)));
    wow_world_spell::handle_player_aura_update(&mut observer.session, committed.clone());
    assert!(observer.output.try_iter().next().is_none());
    observer
        .session
        .set_player_guid(Some(committed.recipient_handle.guid()));

    let stale_residence = command_for(&observer, bytes.clone());
    let manager = observer
        .session
        .core
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .clone();
    {
        let mut manager = manager.lock().unwrap();
        manager
            .detach_player_like_cpp(stale_residence.recipient_handle)
            .unwrap();
        manager
            .attach_player_like_cpp(
                stale_residence.recipient_handle,
                stale_residence.map_key,
                Position::new(20.0, 0.0, 0.0, 0.0),
            )
            .unwrap();
    }
    wow_world_spell::handle_player_aura_update(&mut observer.session, stale_residence);
    assert!(observer.output.try_iter().next().is_none());

    let stale_incarnation = command_for(&observer, bytes);
    let new_handle = {
        let mut manager = manager.lock().unwrap();
        let mut player = Box::new(wow_entities::Player::new(Some(1), false));
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(stale_incarnation.recipient_handle.guid());
        let handle = manager.install_detached_player_like_cpp(player).unwrap();
        manager
            .attach_player_like_cpp(
                handle,
                stale_incarnation.map_key,
                Position::new(20.0, 0.0, 0.0, 0.0),
            )
            .unwrap();
        handle
    };
    observer.session.core.player_handle_like_cpp = Some(new_handle);
    wow_world_spell::handle_player_aura_update(&mut observer.session, stale_incarnation);
    assert!(observer.output.try_iter().next().is_none());
}
