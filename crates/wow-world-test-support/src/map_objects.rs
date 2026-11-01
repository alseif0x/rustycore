//! Session-free canonical map fixtures: test GUIDs and canonical creature, pet, player and
//! gameobject insertion on a `MapManager`.
//!
//! Moved from `wow-world` `session::tests::fixtures::map_objects` by #1363 slice 0;
//! slice 3b added `test_creature_create_data` from `fixtures::creatures`.

use crate::canonical_creatures::shared_canonical_map_manager;
use wow_core::ObjectGuid;
use wow_core::Position;
use wow_entities::AccessorObjectKind;
use wow_entities::GameObject;
use wow_entities::Player;
use wow_world_core::canonical_player_access::with_canonical_player_at_like_cpp;
use wow_world_core::session::SharedCanonicalMapManager;
/// The creature store of a scenario: a canonical map manager (#1263 F6-8E:
/// the only creature store).
pub fn shared_map_manager() -> SharedCanonicalMapManager {
    shared_canonical_map_manager()
}

pub fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

pub fn test_vehicle_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Vehicle, 0, 1, 0, 0, 2, counter)
}

pub fn test_pet_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 3, counter)
}

pub fn test_gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
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

pub fn add_canonical_test_creature(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
) {
    add_canonical_test_creature_on_map(canonical, guid, entry, position, npc_flags, 571, 0);
}

pub fn add_canonical_test_creature_on_map(
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

pub fn add_canonical_test_creature_indexed_on_map_with_level(
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

pub fn add_canonical_test_creature_on_map_with_world_state(
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

pub fn add_canonical_test_creature_on_map_with_world_state_and_owner(
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

pub fn add_canonical_test_pet(
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

pub fn add_canonical_test_pet_with_visible_aura(
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

pub fn add_canonical_test_pet_with_number(
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

pub fn add_canonical_test_gameobject(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
) {
    add_canonical_test_gameobject_on_map(canonical, guid, entry, position, 571, 0);
}

pub fn add_canonical_test_player_on_map(
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

pub fn canonical_party_type_for_test(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> [u8; 2] {
    with_canonical_player_at_like_cpp(canonical, guid, 571, 0, |player| player.data().party_type)
        .expect("canonical player")
}

pub fn add_canonical_test_player_on_map_with_difficulty(
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

pub fn add_canonical_test_gameobject_on_map(
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

pub fn add_canonical_lifecycle_gameobject_on_map(
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

pub fn add_canonical_spell_focus_gameobject_on_map_like_cpp(
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

pub fn add_canonical_visibility_on_destroy_gameobject_like_cpp(
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

pub fn add_canonical_visual_despawn_gameobject_like_cpp(
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

pub fn add_canonical_capture_point_delete_gameobject_like_cpp(
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

pub fn test_creature_create_data(
    guid: ObjectGuid,
    entry: u32,
    hp: u32,
) -> wow_packet::packets::update::CreatureCreateData {
    wow_packet::packets::update::CreatureCreateData {
        guid,
        entry,
        display_id: 100,
        native_display_id: 100,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: hp as i64,
        max_health: hp as i64,
        level: 2,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 0,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    }
}
