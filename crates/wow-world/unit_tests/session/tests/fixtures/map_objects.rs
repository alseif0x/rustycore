//! Canonical map and world-object fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn shared_map_manager() -> crate::map_manager::SharedMapManager {
    Arc::new(std::sync::RwLock::new(crate::map_manager::MapManager::new()))
}

pub(in crate::session::tests) fn shared_canonical_map_manager() -> SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

pub(in crate::session::tests) fn configure_player_shape_mount_collision_stores_like_cpp(
    session: &mut WorldSession,
) {
    let native_display_id = crate::handlers::character::default_display_id(
        session.player_race_like_cpp(),
        session.player_gender_like_cpp(),
    );
    session.set_creature_template_mount_store(Arc::new(
        wow_data::CreatureTemplateMountStoreLikeCpp::from_entries([
            wow_data::CreatureTemplateMountEntryLikeCpp {
                entry: 1234,
                vehicle_id: 55,
                models: vec![wow_data::CreatureTemplateMountModelLikeCpp {
                    display_id: 4321,
                    display_scale: 1.0,
                    probability: 0.0,
                }],
            },
        ]),
    ));
    session.set_creature_display_info_store(Arc::new(
        wow_data::CreatureDisplayInfoStore::from_entries([
            wow_data::CreatureDisplayInfoEntry {
                id: native_display_id,
                model_id: 100,
                extended_display_info_id: 0,
                creature_model_scale: 1.2,
            },
            wow_data::CreatureDisplayInfoEntry {
                id: 4321,
                model_id: 200,
                extended_display_info_id: 0,
                creature_model_scale: 1.5,
            },
        ]),
    ));
    session.set_creature_model_data_store(Arc::new(
        wow_data::CreatureModelDataStore::from_entries([
            wow_data::CreatureModelDataEntry {
                id: 100,
                flags: 0,
                file_data_id: 0,
                collision_height: 2.0,
                hover_height: 0.75,
                model_scale: 1.1,
                mount_height: 0.0,
            },
            wow_data::CreatureModelDataEntry {
                id: 200,
                flags: 0,
                file_data_id: 0,
                collision_height: 0.0,
                hover_height: 1.25,
                model_scale: 1.0,
                mount_height: 4.0,
            },
        ]),
    ));
}

pub(in crate::session::tests) fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

pub(in crate::session::tests) fn test_vehicle_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Vehicle, 0, 1, 0, 0, 2, counter)
}

pub(in crate::session::tests) fn test_pet_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 3, counter)
}

pub(in crate::session::tests) fn test_gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

pub(in crate::session::tests) fn add_canonical_test_creature(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
) {
    add_canonical_test_creature_on_map(canonical, guid, entry, position, npc_flags, 571, 0);
}

pub(in crate::session::tests) fn add_canonical_test_creature_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
) {
    add_canonical_test_creature_on_map_with_world_state(
        canonical,
        guid,
        entry,
        position,
        npc_flags,
        map_id,
        instance_id,
        true,
    );
}

pub(in crate::session::tests) fn add_canonical_test_creature_indexed_on_map_with_level(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
    level: u8,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(level);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, 0, 0);

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_test_creature_on_map_with_world_state(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
    is_in_world: bool,
) {
    add_canonical_test_creature_on_map_with_world_state_and_owner(
        canonical,
        guid,
        entry,
        position,
        npc_flags,
        map_id,
        instance_id,
        is_in_world,
        None,
    );
}

pub(in crate::session::tests) fn add_canonical_test_creature_on_map_with_world_state_and_owner(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
    is_in_world: bool,
    owner_guid: Option<ObjectGuid>,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_owner_guid(owner_guid);
    if is_in_world {
        creature.unit_mut().world_mut().object_mut().add_to_world();
    }

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_test_pet(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
) {
    add_canonical_test_pet_with_visible_aura(
        canonical, guid, owner_guid, entry, position, npc_flags, None, None,
    );
}

pub(in crate::session::tests) fn add_canonical_test_pet_with_visible_aura(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    visible_aura: Option<(u8, u32, ObjectGuid, u32)>,
    visible_application: Option<wow_entities::VisibleAuraApplicationLikeCpp>,
) {
    let mut pet = wow_entities::Pet::new(owner_guid, wow_entities::PetType::Summon);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    pet.creature_mut().unit_mut().world_mut().relocate(position);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_combat_reach(1.0);
    pet.creature_mut().unit_mut().set_level(80);
    pet.creature_mut().unit_mut().set_max_health(100);
    pet.creature_mut().unit_mut().set_health(100);
    pet.creature_mut()
        .set_ai_identity_runtime(1, 35, npc_flags, 0);
    if let Some((slot, spell_id, caster_guid, effect_mask)) = visible_aura {
        let aura = wow_entities::AppliedAuraRef::new(spell_id, caster_guid, slot, effect_mask);
        pet.creature_mut()
            .unit_mut()
            .subsystems_mut()
            .auras
            .add_applied(aura);
        if let Some(application) = visible_application {
            pet.creature_mut()
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_visible_with_application_like_cpp(slot, aura.aura_ref(), application);
        } else {
            pet.creature_mut()
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_visible(slot, aura.aura_ref());
        }
    }
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_test_pet_with_number(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
    entry: u32,
    position: Position,
    pet_number: u32,
    created_by_spell_id: u32,
    duration_ms: i32,
) {
    let mut pet = wow_entities::Pet::new(owner_guid, wow_entities::PetType::Hunter);
    pet.set_created_by_spell_id_like_cpp(created_by_spell_id);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    pet.creature_mut().unit_mut().world_mut().relocate(position);
    pet.creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .init_charm_info()
        .pet_number = pet_number;
    pet.set_duration(duration_ms);

    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_test_gameobject(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
) {
    add_canonical_test_gameobject_on_map(canonical, guid, entry, position, 571, 0);
}

pub(in crate::session::tests) fn add_canonical_test_player_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    add_canonical_test_player_on_map_with_difficulty(
        canonical,
        guid,
        position,
        map_id,
        instance_id,
        0,
    );
}

/// C++ `Player::GetNPCIfCanInteractWith` requires the canonical Player to be
/// in-world, alive and to carry a resolvable faction before a positive NPC
/// interaction fixture is meaningful.
pub(in crate::session::tests) fn adopt_live_canonical_test_player_for_interaction_like_cpp(
    session: &mut WorldSession,
) {
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            player.unit_mut().set_faction(1);
        })
        .expect("live canonical Player interaction fixture");
}

pub(in crate::session::tests) fn bind_canonical_test_player_to_registry_like_cpp(
    session: &mut WorldSession,
    registry: &Arc<PlayerRegistry>,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
) -> SharedCanonicalMapManager {
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, guid, position, map_id, 0);
    canonical
}

pub(in crate::session::tests) fn canonical_party_type_for_test(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> [u8; 2] {
    with_canonical_player_at_like_cpp(canonical, guid, 571, 0, |player| player.data().party_type)
        .expect("canonical player")
}

pub(in crate::session::tests) fn add_canonical_test_player_on_map_with_difficulty(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    difficulty_id: u8,
) {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("InstanceOwner");
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();

    canonical
        .lock()
        .unwrap()
        .create_map_entry(
            map_id,
            instance_id,
            difficulty_id,
            wow_map::ManagedMapKind::World,
        )
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_test_gameobject_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject.world().clone());
    gameobject.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_lifecycle_gameobject_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    rotation: [f32; 4],
    map_id: u32,
    instance_id: u32,
) {
    let mut template_data = [0_u32; wow_entities::MAX_GAMEOBJECT_DATA];
    template_data[wow_entities::GAMEOBJECT_DATA_CHEST_LOOT] = 9_001;
    let gameobject =
        GameObject::try_create_from_lifecycle(wow_entities::GameObjectCreateLifecycleRecord {
            guid,
            map_id,
            instance_id,
            position,
            rotation,
            anim_progress: 33,
            go_state: wow_entities::GoState::Ready,
            art_kit: 4,
            dynamic: false,
            spawn_id: 98_765,
            template: wow_entities::GameObjectTemplateLifecycleRecord {
                entry,
                name: "canonical visible chest".to_string(),
                go_type: wow_entities::GAMEOBJECT_TYPE_CHEST,
                display_id: 7_777,
                scale: 1.75,
                faction: 35,
                flags: 0x24,
                data: template_data,
                world_effect_id: 0,
                anim_kit_id: 0,
                level: 80,
                percent_health: 100,
                custom_param: 0,
            },
        })
        .expect("valid gameobject lifecycle");

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    map.map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_spell_focus_gameobject_on_map_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    focus_type: u32,
    radius: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_represented_spell_focus_use_source_like_cpp(Some(
        wow_entities::SpellFocusUseSource {
            focus_type,
            radius,
            linked_trap_entry: 0,
        },
    ));

    let mut guard = canonical.lock().unwrap();
    guard
        .create_world_map(map_id, instance_id)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_visibility_on_destroy_gameobject_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    spawn_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(5);
    gameobject.set_spawn_id(u64::from(spawn_id));
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_respawn_delay_time(30);
    gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject.world().clone());
    gameobject.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_visual_despawn_gameobject_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    spawn_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(5);
    gameobject.set_spawn_id(u64::from(spawn_id));
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_go_anim_progress_like_cpp(1);
    gameobject.set_represented_baseline_flags_like_cpp(Some(0x08));
    gameobject.set_flags(0x88);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject.world().clone());
    gameobject.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn add_canonical_capture_point_delete_gameobject_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    spawn_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(wow_entities::GAMEOBJECT_TYPE_CAPTURE_POINT as u8);
    gameobject.set_spawn_id(u64::from(spawn_id));
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    assert!(gameobject.schedule_despawn_or_unsummon_like_cpp(1, 0));
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject.world().clone());
    gameobject.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}
