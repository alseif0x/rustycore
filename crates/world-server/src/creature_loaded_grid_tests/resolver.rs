#![cfg(test)]

use super::*;

#[test]
fn loaded_grid_creature_lifecycle_resolver_applies_difficulty_loot_metadata_like_cpp() {
    let entry = 12_345;
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(55, entry, true)],
        [selection(entry)],
    );

    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(55, map_creature_guid(entry, 571, 55))
        .expect("resolver should carry selected difficulty loot metadata");

    assert_eq!(resolved.lifecycle_record.create.template.loot_id, 7_001);
    assert_eq!(
        resolved.lifecycle_record.create.template.skin_loot_id,
        7_002
    );
    assert_eq!(
        (
            resolved.lifecycle_record.create.template.gold_min,
            resolved.lifecycle_record.create.template.gold_max,
        ),
        (17, 29)
    );
    assert_eq!(resolved.creature.ai_ownership().loot_id, 7_001);
    assert_eq!(resolved.creature.ai_ownership().skin_loot_id, 7_002);
    assert_eq!(
        (
            resolved.creature.ai_ownership().gold_min,
            resolved.creature.ai_ownership().gold_max,
        ),
        (17, 29)
    );
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_preserves_absent_loot_metadata_like_cpp() {
    let entry = 12_345;
    let mut no_loot_template = template(entry);
    no_loot_template.loot_id = 0;
    no_loot_template.skin_loot_id = 0;
    no_loot_template.gold_min = 0;
    no_loot_template.gold_max = 0;
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [no_loot_template],
        [spawn(55, entry, true)],
        [selection(entry)],
    );

    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(55, map_creature_guid(entry, 571, 55))
        .expect("zero difficulty loot metadata remains a valid no-loot creature");

    assert_eq!(resolved.creature.ai_ownership().loot_id, 0);
    assert_eq!(resolved.creature.ai_ownership().skin_loot_id, 0);
    assert_eq!(resolved.creature.ai_ownership().gold_min, 0);
    assert_eq!(resolved.creature.ai_ownership().gold_max, 0);
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_applies_sparring_health_pct_like_cpp() {
    let entry = 12_345;
    let mut template = template(entry);
    template.sparring_health_pct = Some(35.5);
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template],
        [spawn(55, entry, true)],
        [selection(entry)],
    );

    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(55, map_creature_guid(entry, 571, 55))
        .expect("resolver should build lifecycle record");

    assert_eq!(resolved.creature.sparring_health_pct(), 35.5);
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_applies_resolved_addon_like_cpp() {
    let entry = 12_346;
    let spawn_id = 56;
    let mut template = template(entry);
    template.addon = Some(CreatureAddonLifecycleRecordLikeCpp {
        path_id: 0,
        mount_display_id: 4321,
        stand_state: wow_constants::UnitStandStateType::Kneel,
        vis_flags: 0x12,
        anim_tier: 2,
        sheath_state: wow_constants::SheathState::Ranged,
        pvp_flags: wow_constants::UnitPvpFlags::PVP | wow_constants::UnitPvpFlags::FFA_PVP,
        emote: 77,
        ai_anim_kit_id: 11,
        movement_anim_kit_id: 22,
        melee_anim_kit_id: 33,
        visibility_distance_type: wow_entities::VisibilityDistanceTypeLikeCpp::Large,
        auras: vec![70_043, 70_044],
        aura_applications: vec![
            wow_entities::CreatureAddonAuraApplicationLikeCpp {
                spell_id: 70_043,
                effect_mask: 0x1,
                flags: 0x0103,
            },
            wow_entities::CreatureAddonAuraApplicationLikeCpp {
                spell_id: 70_044,
                effect_mask: 0x1,
                flags: 0x0103,
            },
        ],
    });
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template],
        [spawn(spawn_id, entry, true)],
        [selection(entry)],
    );

    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(
            spawn_id,
            map_creature_guid(entry, 571, spawn_id as i64),
        )
        .expect("resolver should build lifecycle record");

    assert_eq!(
        resolved.lifecycle_record.create.addon,
        Some(CreatureAddonLifecycleRecordLikeCpp {
            path_id: 0,
            mount_display_id: 4321,
            stand_state: wow_constants::UnitStandStateType::Kneel,
            vis_flags: 0x12,
            anim_tier: 2,
            sheath_state: wow_constants::SheathState::Ranged,
            pvp_flags: wow_constants::UnitPvpFlags::PVP | wow_constants::UnitPvpFlags::FFA_PVP,
            emote: 77,
            ai_anim_kit_id: 11,
            movement_anim_kit_id: 22,
            melee_anim_kit_id: 33,
            visibility_distance_type: wow_entities::VisibilityDistanceTypeLikeCpp::Large,
            auras: vec![70_043, 70_044],
            aura_applications: vec![
                wow_entities::CreatureAddonAuraApplicationLikeCpp {
                    spell_id: 70_043,
                    effect_mask: 0x1,
                    flags: 0x0103,
                },
                wow_entities::CreatureAddonAuraApplicationLikeCpp {
                    spell_id: 70_044,
                    effect_mask: 0x1,
                    flags: 0x0103,
                },
            ],
        }),
        "C++ Creature::LoadFromDB/Create carries the addon selected by Creature::GetCreatureAddon"
    );
    assert_eq!(resolved.creature.unit().data().mount_display_id, 4321);
    assert_eq!(
        resolved.creature.unit().stand_state_like_cpp(),
        wow_constants::UnitStandStateType::Kneel
    );
    assert_eq!(resolved.creature.unit().ai_anim_kit_id_like_cpp(), 11);
    assert_eq!(resolved.creature.unit().movement_anim_kit_id_like_cpp(), 22);
    assert_eq!(resolved.creature.unit().melee_anim_kit_id_like_cpp(), 33);
    assert_eq!(resolved.creature.unit().vis_flags_like_cpp(), 0x12);
    assert_eq!(resolved.creature.unit().anim_tier_like_cpp(), 2);
    assert_eq!(
        resolved.creature.unit().sheath_like_cpp(),
        wow_constants::SheathState::Ranged
    );
    assert_eq!(
        resolved.creature.unit().pvp_flags_like_cpp(),
        wow_constants::UnitPvpFlags::PVP | wow_constants::UnitPvpFlags::FFA_PVP
    );
    assert_eq!(resolved.creature.unit().emote_state_like_cpp(), 77);
    assert!(
        resolved
            .creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_043),
        "C++ Creature::LoadCreaturesAddon casts each selected addon aura when the loaded-grid spawn creates the creature"
    );
    assert!(
        resolved
            .creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_044),
        "C++ Creature::LoadCreaturesAddon applies all permanent addon auras selected by Creature::GetCreatureAddon"
    );
    let map_record_creature = resolved
        .map_object_record
        .as_ref()
        .and_then(MapObjectRecord::creature)
        .expect("loaded-grid AddToMap record should carry the same created creature");
    assert!(
        map_record_creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_043),
        "C++ Map::AddToMap stores the already-created creature after LoadCreaturesAddon has applied addon auras"
    );
    let aura_subsystem = &map_record_creature.unit().subsystems().auras;
    assert_eq!(
        aura_subsystem
            .visible_auras
            .get(&0)
            .map(|aura| aura.spell_id),
        Some(70_043),
        "C++ AuraApplication constructor registers addon aura in visible slot 0"
    );
    assert_eq!(
        aura_subsystem
            .visible_aura_applications_like_cpp
            .get(&0)
            .map(|application| application.flags),
        Some(0x0103),
        "C++ addon AuraApplication flags are preserved for SMSG_AURA_UPDATE"
    );
    assert!(
        aura_subsystem
            .applied_auras
            .iter()
            .any(|aura| aura.spell_id == 70_043 && aura.effect_mask == 0x1),
        "C++ addon aura active effect mask is carried into AuraDataInfo::ActiveFlags"
    );
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_accepts_map_owned_low_guid_distinct_from_spawn_id_like_cpp()
 {
    let entry = 12_349;
    let spawn_id = 61;
    let caller_low_guid = 345_678;
    let map_object_guid = map_creature_guid(entry, 571, caller_low_guid);
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(spawn_id, entry, true)],
        [selection(entry)],
    );

    assert_ne!(spawn_id as i64, caller_low_guid);
    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(spawn_id, map_object_guid)
        .expect("C++ Creature::LoadFromDB uses map-owned lowguid, not spawn id, for live GUID");
    let creature = resolved
        .map_object_record
        .as_ref()
        .and_then(|record| record.creature())
        .expect("resolver should build a creature record");
    assert_eq!(creature.guid(), map_object_guid);
    assert_eq!(creature.lifecycle_metadata().spawn_id, spawn_id);
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_propagates_formation_info_to_record_like_cpp() {
    let entry = 12_350;
    let spawn_id = 62;
    let formation_info = CreatureFormationInfoLikeCpp {
        leader_spawn_id: 62,
        follow_dist: 0.0,
        follow_angle_radians: 0.0,
        group_ai: 7,
        leader_waypoint_ids: [101, 102],
    };
    let mut resolved_spawn = spawn(spawn_id, entry, true);
    resolved_spawn.formation_info = Some(formation_info);
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [resolved_spawn],
        [selection(entry)],
    );
    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(
            spawn_id,
            map_creature_guid(entry, 571, spawn_id as i64),
        )
        .expect("formation metadata should not block loaded-grid resolver");

    assert_eq!(
        resolved.creature.formation_info_like_cpp(),
        Some(&formation_info)
    );
    assert_eq!(
        resolved
            .map_object_record
            .as_ref()
            .and_then(MapObjectRecord::creature)
            .and_then(Creature::formation_info_like_cpp),
        Some(&formation_info)
    );
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_preserves_absent_formation_info_like_cpp() {
    let entry = 12_351;
    let spawn_id = 63;
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(spawn_id, entry, true)],
        [selection(entry)],
    );
    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(
            spawn_id,
            map_creature_guid(entry, 571, spawn_id as i64),
        )
        .expect("absence of formation metadata is the previous behavior");

    assert!(resolved.creature.formation_info_like_cpp().is_none());
    assert!(
        resolved
            .map_object_record
            .as_ref()
            .and_then(MapObjectRecord::creature)
            .and_then(Creature::formation_info_like_cpp)
            .is_none()
    );
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_respects_add_to_map_request_flag() {
    let entry = 12_346;
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(56, entry, false)],
        [selection(entry)],
    );

    let map_object_guid = map_creature_guid(entry, 571, 56);
    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(56, map_object_guid)
        .expect("resolver should build creature without insertion request");

    assert!(!resolved.map_insertion_requested);
    assert!(resolved.map_object_record.is_none());
    assert!(
        !resolved
            .creature
            .lifecycle_metadata()
            .map_insertion_requested
    );
    assert!(!resolved.creature.lifecycle_metadata().add_to_map_requested);
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_errors_without_dummy_for_missing_inputs() {
    let entry = 12_347;
    let map_object_guid = map_creature_guid(entry, 571, 99_003);
    let missing_spawn =
        CreatureLoadedGridLifecycleResolverLikeCpp::new([template(entry)], [], [selection(entry)]);
    assert_eq!(
        missing_spawn.resolve_loaded_grid_creature_like_cpp(57, map_object_guid),
        Err(CreatureLoadedGridResolveErrorLikeCpp::MissingSpawnData { spawn_id: 57 })
    );

    let missing_template = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [],
        [spawn(58, entry, true)],
        [selection(entry)],
    );
    assert_eq!(
        missing_template.resolve_loaded_grid_creature_like_cpp(58, map_object_guid),
        Err(CreatureLoadedGridResolveErrorLikeCpp::MissingTemplate { entry })
    );

    let missing_selection = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(59, entry, true)],
        [],
    );
    assert_eq!(
        missing_selection.resolve_loaded_grid_creature_like_cpp(59, map_object_guid),
        Err(CreatureLoadedGridResolveErrorLikeCpp::MissingRuntimeSelection { entry })
    );
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_rejects_wrong_map_or_high_guid() {
    let entry = 12_350;
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(62, entry, true)],
        [selection(entry)],
    );
    let wrong_map_guid = map_creature_guid(entry, 530, 99_004);
    assert_eq!(
        resolver.resolve_loaded_grid_creature_like_cpp(62, wrong_map_guid),
        Err(
            CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                guid: wrong_map_guid,
                expected_high: HighGuid::Creature,
                expected_map_id: 571,
                expected_entry: entry,
            }
        )
    );

    let wrong_high_guid = ObjectGuid::create_gameobject_like_cpp(571, entry, 99_005);
    assert_eq!(
        resolver.resolve_loaded_grid_creature_like_cpp(62, wrong_high_guid),
        Err(
            CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                guid: wrong_high_guid,
                expected_high: HighGuid::Creature,
                expected_map_id: 571,
                expected_entry: entry,
            }
        )
    );
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_rejects_same_map_wrong_entry_guid() {
    let entry = 12_352;
    let wrong_entry = entry + 1;
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(64, entry, true)],
        [selection(entry)],
    );
    let wrong_entry_guid = map_creature_guid(wrong_entry, 571, 99_008);

    assert_eq!(
        resolver.resolve_loaded_grid_creature_like_cpp(64, wrong_entry_guid),
        Err(
            CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                guid: wrong_entry_guid,
                expected_high: HighGuid::Creature,
                expected_map_id: 571,
                expected_entry: entry,
            }
        )
    );
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_accepts_vehicle_template_with_vehicle_guid_like_cpp() {
    let entry = 12_351;
    let map_object_guid = map_vehicle_guid(entry, 571, 63);
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [vehicle_template(entry, 77)],
        [spawn(63, entry, true)],
        [selection(entry)],
    );

    let resolved = resolver
        .resolve_loaded_grid_creature_like_cpp(63, map_object_guid)
        .expect("vehicle-template Creature should resolve with HighGuid::Vehicle");

    assert_eq!(resolved.lifecycle_record.create.vehicle_id, Some(77));
    assert_eq!(
        resolved.lifecycle_record.create.guid.high_type(),
        HighGuid::Vehicle
    );
    assert_eq!(resolved.creature.guid(), map_object_guid);
    assert_eq!(resolved.creature.lifecycle_metadata().vehicle_id, Some(77));
    let kit = resolved
        .creature
        .unit()
        .subsystems()
        .vehicle
        .kit
        .as_ref()
        .unwrap();
    assert_eq!(kit.kit_id(), 77);
    assert!(kit.active());
    assert!(!kit.installed());
    assert_eq!(kit.seat_count(), 0);
    let recorded = resolved
        .map_object_record
        .as_ref()
        .and_then(MapObjectRecord::creature)
        .expect("vehicle-template map record should remain typed Creature");
    assert_eq!(recorded.guid().high_type(), HighGuid::Vehicle);
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_rejects_creature_guid_for_vehicle_template_like_cpp() {
    let entry = 12_353;
    let wrong_guid = map_creature_guid(entry, 571, 99_009);
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [vehicle_template(entry, 88)],
        [spawn(65, entry, true)],
        [selection(entry)],
    );

    assert_eq!(
        resolver.resolve_loaded_grid_creature_like_cpp(65, wrong_guid),
        Err(
            CreatureLoadedGridResolveErrorLikeCpp::InvalidMapObjectGuid {
                guid: wrong_guid,
                expected_high: HighGuid::Vehicle,
                expected_map_id: 571,
                expected_entry: entry,
            }
        )
    );
}
#[test]
fn loaded_grid_creature_lifecycle_resolver_is_pure_ordered_bridge_like_cpp() {
    let plan = wow_entities::CreatureLifecyclePlan::trinity_create_load_from_db();
    assert!(plan.occurs_before(
        wow_entities::CreatureLifecycleStep::LookupTemplateAndDifficulty,
        wow_entities::CreatureLifecycleStep::InitEntryAndCreateFromProto,
    ));
    assert!(plan.occurs_before(
        wow_entities::CreatureLifecycleStep::LoadFromDbSpawnHomeRespawnInactiveChecks,
        wow_entities::CreatureLifecycleStep::AddToMap,
    ));

    let entry = 12_348;
    let resolver = CreatureLoadedGridLifecycleResolverLikeCpp::new(
        [template(entry)],
        [spawn(60, entry, true)],
        [selection(entry)],
    );
    let map_object_guid = map_creature_guid(entry, 571, 60);
    let first = resolver
        .resolve_loaded_grid_creature_like_cpp(60, map_object_guid)
        .unwrap();
    let second = resolver
        .resolve_loaded_grid_creature_like_cpp(60, map_object_guid)
        .unwrap();

    assert_eq!(first.lifecycle_record, second.lifecycle_record);
    assert_eq!(
        first.creature.lifecycle_metadata(),
        second.creature.lifecycle_metadata()
    );
    assert!(first.map_insertion_requested);
    assert!(second.map_insertion_requested);
}
