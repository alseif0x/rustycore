use super::*;

pub fn install_cached_test_creature_loot_authority_for_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) {
    if let Some(loot) = session.loot_table.get_mut(&owner_guid) {
        wow_loot::rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
    }
    sync_creature_loot_fixture_for_test(session, owner_guid, player_guid)
        .expect("the kill-time loot fixture must install into its creature authority");
}

pub fn tap_test_creature_for_loot(
    session: &mut WorldSession,
    creature_guid: ObjectGuid,
    player_guid: ObjectGuid,
) {
    let _ = session.mutate_world_creature(creature_guid, |world_creature| {
        world_creature
            .creature
            .set_tapped_by_player(player_guid, &[]);
    });
}

pub fn make_canonical_creature_for_loot_test(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Creature {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap();
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature
}

pub fn attach_canonical_creature_for_loot_test(
    session: &mut WorldSession,
    creature: Creature,
) {
    let map_id = creature.unit().world().map_id();
    let instance_id = creature.unit().world().instance_id();
    let manager = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    manager
        .lock()
        .expect("fresh canonical test map lock")
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    session.set_canonical_map_manager(manager);
}

pub fn canonical_creature_snapshot_for_loot_test(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<Creature> {
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
    map.map().with_creature_like_cpp(guid, Clone::clone)
}

pub fn set_world_creature_personal_loot_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    player_guid: ObjectGuid,
    loot: wow_entities::CreatureOwnedLoot,
) -> bool {
    session
        .mutate_world_creature(guid, |world_creature| {
            world_creature
                .creature
                .set_personal_loot_like_cpp(player_guid, loot);
        })
        .is_some()
}

pub fn register_test_creature_for_loot(session: &mut WorldSession, creature: CreatureAI) {
    if session.map_manager.is_none() {
        session.set_map_manager(Arc::new(RwLock::new(MapManager::new())));
    }

    let create_data = wow_packet::packets::update::CreatureCreateData {
        guid: creature.guid,
        entry: creature.entry,
        display_id: creature.display_id,
        native_display_id: creature.display_id,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: i64::from(creature.hp.max(1)),
        max_health: i64::from(creature.max_hp.max(1)),
        level: creature.level,
        faction_template: creature.faction as i32,
        npc_flags: u64::from(creature.npc_flags),
        unit_flags: creature.unit_flags,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: WorldCreature::health_aura_state_like_cpp(
            u64::from(creature.hp.max(1)),
            u64::from(creature.max_hp.max(1)),
            true,
        ),
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
    };
    let guid = creature.guid;
    let is_alive = creature.is_alive;
    session.register_world_creature(
        session.player_map_id_like_cpp(),
        creature.current_pos,
        create_data,
        creature.min_dmg,
        creature.max_dmg,
        creature.aggro_radius,
        creature.loot_id,
        0,
        creature.gold_min,
        creature.gold_max,
        creature.boss_id,
        creature.dungeon_encounter_id,
        0,
        0,
        0,
        -1,
    );
    if !is_alive {
        let _ = session.mutate_world_creature(guid, |world_creature| {
            world_creature.creature.mark_ai_dead(0);
        });
    }
}

pub fn canonical_corpse_snapshot_for_loot_test(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<Corpse> {
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
    map.map().get_typed_corpse(guid).cloned()
}

pub fn attach_canonical_corpse_for_loot_test(session: &mut WorldSession, corpse: Corpse) {
    let map_id = corpse.world().map_id();
    let instance_id = corpse.world().instance_id();
    let manager = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    manager
        .lock()
        .expect("fresh canonical test map lock")
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_corpse(corpse).unwrap())
        .unwrap();
    session.set_canonical_map_manager(manager);
}

pub fn canonical_world_object_for_loot_test(
    guid: ObjectGuid,
    map_id: u32,
    position: Position,
) -> WorldObject {
    let (type_id, type_mask) = if guid.is_game_object() {
        (TypeId::GameObject, TypeMask::GAME_OBJECT)
    } else {
        (TypeId::Unit, TypeMask::UNIT)
    };
    let mut object = WorldObject::new(false, type_id, type_mask);
    object.object_mut().create(guid);
    object.set_map(map_id, 0).unwrap();
    object.relocate(position);
    object.object_mut().add_to_world();
    object
}

pub fn attach_canonical_map_object_for_loot_test(
    session: &mut WorldSession,
    kind: AccessorObjectKind,
    object: WorldObject,
) {
    let map_id = object.map_id();
    let instance_id = object.instance_id();
    let manager = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    manager
        .lock()
        .expect("fresh canonical test map lock")
        .create_world_map(map_id, instance_id)
        .map_mut()
        .add_to_map_like_cpp(kind, object)
        .unwrap();
    session.set_canonical_map_manager(manager);
}

pub fn make_canonical_gameobject_for_loot_test(
    session: &WorldSession,
    guid: ObjectGuid,
    go_type: u8,
) -> GameObject {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject
        .world_mut()
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap();
    gameobject.world_mut().relocate(Position::ZERO);
    gameobject.world_mut().object_mut().add_to_world();
    gameobject.set_go_type(go_type);
    gameobject
}

pub fn attach_canonical_gameobject_for_loot_test(
    session: &mut WorldSession,
    gameobject: GameObject,
) {
    let map_id = gameobject.world().map_id();
    let instance_id = gameobject.world().instance_id();
    let manager = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    manager
        .lock()
        .expect("fresh canonical test map lock")
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
    session.set_canonical_map_manager(manager);
}

pub fn canonical_gameobject_snapshot_for_loot_test(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<GameObject> {
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
    map.map().get_typed_game_object(guid).cloned()
}

pub fn make_canonical_corpse_for_loot_test(session: &WorldSession, guid: ObjectGuid) -> Corpse {
    let mut corpse = Corpse::new_at(CorpseType::ResurrectablePvp, 1_000);
    corpse.world_mut().object_mut().create(guid);
    corpse
        .world_mut()
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap();
    corpse.world_mut().relocate(Position::ZERO);
    corpse.world_mut().object_mut().add_to_world();
    corpse.set_corpse_dynamic_flag(wow_entities::CORPSE_DYNFLAG_LOOTABLE);
    corpse.clear_corpse_data_changes();
    corpse
}
