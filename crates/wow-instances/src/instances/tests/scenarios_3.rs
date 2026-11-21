use super::*;

fn icc_lock_data(states: [u8; 13], entrance_world_safe_loc_id: u32) -> InstanceLockData {
    InstanceLockData {
        data: format!(
            "{{\"Header\":\"ICCv1\",\"BossStates\":[{}]}}",
            states
                .into_iter()
                .map(|state| state.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
        completed_encounters_mask: 0,
        entrance_world_safe_loc_id,
    }
}

#[test]
fn icc_instance_script_new_and_empty_attached_lock_create_all_encounters() {
    let new_script = IccInstanceScriptLikeCpp::initialize_for_binding_like_cpp(4, None);
    assert_eq!(
        new_script.boss_state_like_cpp(0),
        EncounterState::NotStarted
    );
    assert_eq!(
        new_script.boss_state_like_cpp(12),
        EncounterState::NotStarted
    );
    assert_eq!(new_script.entrance_location_like_cpp(), 0);

    let empty_lock = InstanceLockData {
        data: String::new(),
        completed_encounters_mask: 0,
        entrance_world_safe_loc_id: 817,
    };
    let attached_script =
        IccInstanceScriptLikeCpp::initialize_for_binding_like_cpp(4, Some(&empty_lock));

    assert_eq!(
        attached_script.boss_state_like_cpp(0),
        EncounterState::NotStarted
    );
    assert_eq!(
        attached_script.boss_state_like_cpp(12),
        EncounterState::NotStarted
    );
    assert_eq!(attached_script.entrance_location_like_cpp(), 817);
}

#[test]
fn icc_instance_script_saved_states_normalize_and_repair_required_bosses() {
    let mut saved_states = [0; 13];
    saved_states[0] = 3;
    saved_states[1] = 1;
    saved_states[2] = 2;
    saved_states[3] = 4;
    saved_states[4] = 5;
    saved_states[11] = 3;
    let lock_data = icc_lock_data(saved_states, 919);

    let script = IccInstanceScriptLikeCpp::initialize_for_binding_like_cpp(4, Some(&lock_data));

    assert_eq!(script.boss_state_like_cpp(0), EncounterState::Done);
    for boss_id in 1..=3 {
        assert_eq!(
            script.boss_state_like_cpp(boss_id),
            EncounterState::NotStarted,
            "transient encounter {boss_id} is normalized on load"
        );
    }
    assert_eq!(script.boss_state_like_cpp(4), EncounterState::ToBeDecided);
    assert_eq!(
        script.boss_state_like_cpp(11),
        EncounterState::NotStarted,
        "Lich King DONE is cleared when ICC prerequisites are missing"
    );
    assert_eq!(script.entrance_location_like_cpp(), 919);
    assert_eq!(script.coldflame_jets_state_like_cpp(), None);
    assert_eq!(script.upper_spire_teleporter_active_state_like_cpp(), None);
}

#[test]
fn icc_instance_script_preserves_valid_completion_chain_and_post_load_fields() {
    let lock_data = icc_lock_data([3; 13], 77);

    let script = IccInstanceScriptLikeCpp::initialize_for_binding_like_cpp(6, Some(&lock_data));

    assert!(script.check_required_bosses_like_cpp(11));
    assert_eq!(script.boss_state_like_cpp(11), EncounterState::Done);
    assert_eq!(script.boss_state_like_cpp(12), EncounterState::Done);
    assert_eq!(script.coldflame_jets_state_like_cpp(), Some(3));
    assert_eq!(
        script.upper_spire_teleporter_active_state_like_cpp(),
        Some(3)
    );
    assert_eq!(script.entrance_location_like_cpp(), 77);
    assert_eq!(
        script.get_save_data_like_cpp(),
        "{\"Header\":\"ICCv1\",\"BossStates\":[3,3,3,3,3,3,3,3,3,3,3,3,3]}"
    );
}

#[test]
fn icc_instance_script_required_bosses_follow_cpp_gates() {
    let script = IccInstanceScriptLikeCpp::initialize_for_binding_like_cpp(3, None);

    assert!(script.check_required_bosses_like_cpp(0));
    assert!(!script.check_required_bosses_like_cpp(1));
    assert!(!script.check_required_bosses_like_cpp(2));
    assert!(script.check_required_bosses_like_cpp(12));
}
