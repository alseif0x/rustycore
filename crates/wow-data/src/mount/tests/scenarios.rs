//! Mount store regressions.
//!
//! Moved out of mount.rs under #685; every test is unchanged.

use super::*;

#[test]
fn mount_store_indexes_by_source_spell_like_cpp() {
    let store = MountStore::from_entries([
        MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 42,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: -1,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ]);

    let mount = store.get_by_source_spell_id_like_cpp(100).unwrap();
    assert_eq!(mount.id, 1);
    assert_eq!(mount.player_condition_id, 42);
    assert!(store.get_by_source_spell_id_like_cpp(999).is_none());
}

#[test]
fn mount_type_capabilities_are_grouped_and_sorted_like_cpp_set() {
    let store = MountTypeXCapabilityStore::from_entries([
        MountTypeXCapabilityEntry {
            id: 1,
            mount_type_id: 7,
            mount_capability_id: 70,
            order_index: 2,
        },
        MountTypeXCapabilityEntry {
            id: 2,
            mount_type_id: 7,
            mount_capability_id: 71,
            order_index: 1,
        },
        MountTypeXCapabilityEntry {
            id: 3,
            mount_type_id: 7,
            mount_capability_id: 72,
            order_index: 1,
        },
    ]);

    let capabilities = store.capabilities_for_mount_type_like_cpp(7).unwrap();
    assert_eq!(
        capabilities
            .iter()
            .map(|entry| entry.mount_capability_id)
            .collect::<Vec<_>>(),
        vec![71, 70]
    );
    assert!(store.capabilities_for_mount_type_like_cpp(99).is_none());
}

#[test]
fn mount_displays_are_grouped_by_mount_like_cpp() {
    let store = MountXDisplayStore::from_entries([
        MountXDisplayEntry {
            id: 1,
            creature_display_info_id: 1000,
            player_condition_id: 42,
            mount_id: 7,
        },
        MountXDisplayEntry {
            id: 2,
            creature_display_info_id: 1001,
            player_condition_id: 0,
            mount_id: 7,
        },
    ]);

    let displays = store.displays_for_mount_like_cpp(7).unwrap();
    assert_eq!(displays.len(), 2);
    assert_eq!(displays[0].creature_display_info_id, 1000);
    assert!(store.displays_for_mount_like_cpp(99).is_none());
}

#[test]
fn hotfix_overlays_replace_rows_and_rebuild_derived_mount_indices_like_cpp() {
    let mut capabilities = MountCapabilityStore::from_entries([MountCapabilityEntry {
        id: 1,
        flags: 1,
        req_riding_skill: 75,
        req_area_id: 0,
        req_spell_aura_id: 0,
        req_spell_known_id: 0,
        mod_spell_aura_id: 10,
        req_map_id: -1,
    }]);
    assert_eq!(
        capabilities.apply_hotfix_entries_like_cpp([MountCapabilityEntry {
            id: 1,
            flags: 2,
            req_riding_skill: 150,
            req_area_id: 3,
            req_spell_aura_id: 4,
            req_spell_known_id: 5,
            mod_spell_aura_id: 6,
            req_map_id: 7,
        }]),
        1
    );
    assert_eq!(capabilities.len(), 1);
    assert_eq!(capabilities.get(1).unwrap().req_riding_skill, 150);

    let mut type_capabilities =
        MountTypeXCapabilityStore::from_entries([MountTypeXCapabilityEntry {
            id: 1,
            mount_type_id: 7,
            mount_capability_id: 70,
            order_index: 1,
        }]);
    assert_eq!(
        type_capabilities.apply_hotfix_entries_like_cpp([MountTypeXCapabilityEntry {
            id: 1,
            mount_type_id: 8,
            mount_capability_id: 80,
            order_index: 2,
        }]),
        1
    );
    assert!(
        type_capabilities
            .capabilities_for_mount_type_like_cpp(7)
            .is_none()
    );
    assert_eq!(
        type_capabilities.capabilities_for_mount_type_like_cpp(8),
        Some(
            [MountTypeXCapabilityEntry {
                id: 1,
                mount_type_id: 8,
                mount_capability_id: 80,
                order_index: 2,
            }]
            .as_slice()
        )
    );

    let mut displays = MountXDisplayStore::from_entries([MountXDisplayEntry {
        id: 1,
        creature_display_info_id: 100,
        player_condition_id: 0,
        mount_id: 7,
    }]);
    assert_eq!(
        displays.apply_hotfix_entries_like_cpp([MountXDisplayEntry {
            id: 1,
            creature_display_info_id: 200,
            player_condition_id: 9,
            mount_id: 8,
        }]),
        1
    );
    assert!(displays.displays_for_mount_like_cpp(7).is_none());
    assert_eq!(
        displays
            .displays_for_mount_like_cpp(8)
            .unwrap()
            .first()
            .unwrap()
            .creature_display_info_id,
        200
    );
}

#[test]
fn load_mount_x_display_uses_cpp_parent_relationship_when_fixture_exists() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "enUS";
    let path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("MountXDisplay.db2");
    if !path.exists() {
        eprintln!("Skipping test: MountXDisplay.db2 not found");
        return;
    }

    let store =
        MountXDisplayStore::load(data_dir, locale).expect("failed to load MountXDisplay.db2");
    assert!(store.by_id.values().any(|display| display.mount_id != 0));
}

#[test]
fn load_real_account_mounts_resolves_source_spells_and_capabilities_when_fixture_exists() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "enUS";
    let path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("Mount.db2");
    if !path.exists() {
        eprintln!("Skipping test: Mount.db2 not found");
        return;
    }

    let mounts = MountStore::load(data_dir, locale).expect("failed to load Mount.db2");
    let capabilities =
        MountCapabilityStore::load(data_dir, locale).expect("failed to load MountCapability.db2");
    let type_capabilities = MountTypeXCapabilityStore::load(data_dir, locale)
        .expect("failed to load MountTypeXCapability.db2");

    for source_spell_id in [17229, 32243, 64658] {
        let mount = mounts
            .get_by_source_spell_id_like_cpp(source_spell_id)
            .unwrap_or_else(|| panic!("missing Mount.db2 source spell {source_spell_id}"));
        assert_ne!(
            mount.mount_type_id, 0,
            "mount spell {source_spell_id} resolved to a mount without type"
        );
        assert!(
            type_capabilities
                .capabilities_for_mount_type_like_cpp(mount.mount_type_id)
                .is_some_and(|entries| entries.iter().any(|entry| capabilities
                    .get(u32::from(entry.mount_capability_id))
                    .is_some())),
            "mount spell {source_spell_id} type {} has no usable capability",
            mount.mount_type_id
        );
    }
}

#[test]
fn mount_capability_selection_matches_cpp_filter_order() {
    let capabilities = MountCapabilityStore::from_entries([
        MountCapabilityEntry {
            id: 10,
            flags: MOUNT_CAPABILITY_FLAG_FLYING,
            req_riding_skill: 0,
            req_area_id: 0,
            req_spell_aura_id: 0,
            req_spell_known_id: 0,
            mod_spell_aura_id: 1000,
            req_map_id: -1,
        },
        MountCapabilityEntry {
            id: 11,
            flags: MOUNT_CAPABILITY_FLAG_GROUND,
            req_riding_skill: 75,
            req_area_id: 77,
            req_spell_aura_id: 123,
            req_spell_known_id: 456,
            mod_spell_aura_id: 1001,
            req_map_id: 1,
        },
    ]);
    let type_caps = MountTypeXCapabilityStore::from_entries([
        MountTypeXCapabilityEntry {
            id: 1,
            mount_type_id: 7,
            mount_capability_id: 10,
            order_index: 0,
        },
        MountTypeXCapabilityEntry {
            id: 2,
            mount_type_id: 7,
            mount_capability_id: 11,
            order_index: 1,
        },
    ]);
    let context = MountCapabilityContextLikeCpp {
        riding_skill: 75,
        mount_flags: AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS,
        is_submerged: false,
        is_in_water: false,
        map_id: 1,
        cosmetic_parent_map_id: -1,
        parent_map_id: -1,
    };

    let selected = capabilities
        .select_for_mount_type_like_cpp(
            &type_caps,
            7,
            &context,
            |area_id| area_id == 77,
            |aura_id| aura_id == 123,
            |spell_id| spell_id == 456,
        )
        .unwrap();
    assert_eq!(selected.id, 11);

    assert!(
        capabilities
            .select_for_mount_type_like_cpp(&type_caps, 7, &context, |_| false, |_| true, |_| true,)
            .is_none()
    );
    assert_eq!(
        capabilities
            .select_for_mount_type_with_reject_like_cpp(
                &type_caps,
                7,
                &context,
                |area_id| area_id == 77,
                |aura_id| aura_id == 123,
                |_| false,
            )
            .unwrap_err(),
        MountCapabilityRejectLikeCpp::KnownSpell
    );
    assert_eq!(
        capabilities
            .select_for_mount_type_with_reject_like_cpp(
                &type_caps,
                7,
                &MountCapabilityContextLikeCpp {
                    riding_skill: 10,
                    ..context
                },
                |area_id| area_id == 77,
                |aura_id| aura_id == 123,
                |spell_id| spell_id == 456,
            )
            .unwrap_err(),
        MountCapabilityRejectLikeCpp::RidingSkill
    );
}
