//! Characterization before the #578 cinematic catalog ownership cut.
use super::*;

#[test]
fn cinematic_catalog_wiring_changes_camera_state_not_trigger_admission() {
    // Production currently never installs this store; camera tests inject it.
    // Wiring it is a behavior repair, not merely deleting a Session field.
    for present in [false, true] {
        let (mut session, _, send_rx) = make_session();
        install_canonical_player_owner_for_test(&mut session, 571, 0);
        if present {
            session.set_cinematic_sequences_store(Arc::new(
                wow_data::CinematicSequencesStore::from_entries([
                    wow_data::CinematicSequencesEntry {
                        id: 444,
                        sound_id: 0,
                        camera: [11, 22, 0, 0, 0, 0, 0, 0],
                    },
                ]),
            ));
        }
        session.send_represented_cinematic_start_like_cpp(444);
        assert!(send_rx.try_recv().is_ok());
        assert!(send_rx.try_recv().is_err());
        let state = session.player_cinematic_state_snapshot_like_cpp().unwrap();
        assert_eq!(state.cinematic_id, present.then_some(444));
        assert_eq!(
            state.camera_ids,
            present.then_some([11, 22, 0, 0, 0, 0, 0, 0])
        );
    }
}
