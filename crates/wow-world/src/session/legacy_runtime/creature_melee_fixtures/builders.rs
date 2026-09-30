//! Original fixture builders; no negative admission is repaired by this facade.
use super::*;

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

pub fn register_test_creature(
    session: &mut WorldSession,
    manager: crate::map_manager::SharedMapManager,
    guid: ObjectGuid,
    hp: u32,
) {
    session.set_map_manager(manager);
    session.current_map_id = 0;
    if session.player_position_like_cpp().is_none() {
        session.set_player_map_position_like_cpp(0, Position::new(10.0, 10.0, 0.0, 0.0));
    }
    session.register_world_creature(
        0,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(guid, 9001, hp),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session
        .mutate_world_creature(guid, |creature| {
            // This test fixture represents a fully hydrated DB-backed
            // creature with no addon/template-addon auras.
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_hit_aura_authority_inert_like_cpp(true);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_cast_log_aura_authority_inert_like_cpp(true);
            creature.seed_runtime_rng_like_cpp(0x5E11_117);
        })
        .expect("registered test creature must remain available");

    // Fixtures assert exact melee damage terms, so the represented attack table
    // is inert for them until a test opts back in.
    disable_represented_melee_attack_table_for_test_like_cpp(session, guid);
}

pub fn disable_represented_melee_attack_table_for_test_like_cpp(
    session: &mut WorldSession,
    creature_guid: ObjectGuid,
) {
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        let mut stats = *player.effective_combat_stats_like_cpp();
        // `7.5` is C++'s base, plus the `19` dual-wield penalty
        // `MeleeSpellMissChance` adds, so both hands stay table-inert.
        stats.melee_hit_chance_pct = 26.5;
        player.replace_effective_combat_stats_like_cpp(stats);
    });
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


