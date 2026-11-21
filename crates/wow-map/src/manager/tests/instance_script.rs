use super::*;
use std::collections::HashMap;

use wow_instances::{ICC_INSTANCE_SCRIPT_NAME_LIKE_CPP, InstanceLockData};

fn register_icc_template(manager: &mut MapManager) {
    manager.set_instance_script_templates(HashMap::from([(
        631,
        ICC_INSTANCE_SCRIPT_NAME_LIKE_CPP.to_string(),
    )]));
}

#[test]
fn icc_map_manager_initializes_one_registered_script_and_reads_its_save_data() {
    let mut manager = MapManager::default();
    register_icc_template(&mut manager);
    manager.create_map_entry(
        631,
        9001,
        4,
        ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
    );

    assert!(manager.initialize_instance_script_for_map(631, 9001, None));
    let map = manager.find_map(631, 9001).unwrap();
    assert!(map.has_instance_script());
    assert_eq!(
        map.instance_script_save_data().as_deref(),
        Some("{\"Header\":\"ICCv1\",\"BossStates\":[0,0,0,0,0,0,0,0,0,0,0,0,0]}")
    );

    let saved_lock = InstanceLockData {
        data: "{\"Header\":\"ICCv1\",\"BossStates\":[3,3,3,3,3,3,3,3,3,3,3,3,3]}".to_string(),
        completed_encounters_mask: 0,
        entrance_world_safe_loc_id: 77,
    };
    manager.create_map_entry(
        631,
        9002,
        4,
        ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
    );
    assert!(manager.initialize_instance_script_for_map(631, 9002, Some(&saved_lock)));
    assert!(manager.find_map(631, 9002).unwrap().has_instance_script());
    assert_eq!(
        manager
            .find_map(631, 9002)
            .unwrap()
            .instance_script_save_data()
            .as_deref(),
        Some("{\"Header\":\"ICCv1\",\"BossStates\":[3,3,3,3,3,3,3,3,3,3,3,3,3]}"),
    );
    let repeated_lock = InstanceLockData {
        data: "{\"Header\":\"ICCv1\",\"BossStates\":[0,0,0,0,0,0,0,0,0,0,0,0,0]}".to_string(),
        ..saved_lock
    };
    assert!(manager.initialize_instance_script_for_map(631, 9002, Some(&repeated_lock)));
    assert_eq!(
        manager
            .find_map(631, 9002)
            .unwrap()
            .instance_script_save_data()
            .as_deref(),
        Some("{\"Header\":\"ICCv1\",\"BossStates\":[3,3,3,3,3,3,3,3,3,3,3,3,3]}"),
        "CreateInstanceData is one-shot after the script has been initialized"
    );
}

#[test]
fn map_manager_uses_effective_template_and_gates_icc_by_id_kind_and_difficulty() {
    let cases = [
        (
            "wrong effective script name",
            631,
            4,
            ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
            Some("instance_other"),
        ),
        (
            "wrong map id",
            632,
            4,
            ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
            Some(ICC_INSTANCE_SCRIPT_NAME_LIKE_CPP),
        ),
        (
            "world map kind",
            631,
            4,
            ManagedMapKind::World,
            Some(ICC_INSTANCE_SCRIPT_NAME_LIKE_CPP),
        ),
        (
            "non-reset dungeon kind",
            631,
            4,
            ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
            Some(ICC_INSTANCE_SCRIPT_NAME_LIKE_CPP),
        ),
        (
            "unsupported difficulty",
            631,
            7,
            ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
            Some(ICC_INSTANCE_SCRIPT_NAME_LIKE_CPP),
        ),
    ];

    for (label, map_id, difficulty, kind, registered_name) in cases {
        let mut manager = MapManager::default();
        if let Some(name) = registered_name {
            manager.set_instance_script_templates(HashMap::from([(map_id, name.to_string())]));
        }
        manager.create_map_entry(map_id, 12_345, difficulty, kind);

        assert!(
            !manager.initialize_instance_script_for_map(map_id, 12_345, None),
            "unexpected ICC registration for {label}"
        );
        assert!(
            !manager
                .find_map(map_id, 12_345)
                .unwrap()
                .has_instance_script(),
            "script should remain absent for {label}"
        );
        assert_eq!(
            manager
                .find_map(map_id, 12_345)
                .unwrap()
                .instance_script_save_data(),
            None
        );
    }

    let mut missing_registration = MapManager::default();
    missing_registration.create_map_entry(
        631,
        12_345,
        4,
        ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
    );
    assert!(!missing_registration.initialize_instance_script_for_map(631, 12_345, None));

    let mut world_map = MapManager::default();
    register_icc_template(&mut world_map);
    world_map.create_map_entry(631, 0, 4, ManagedMapKind::World);
    assert!(!world_map.initialize_instance_script_for_map(631, 0, None));
}
