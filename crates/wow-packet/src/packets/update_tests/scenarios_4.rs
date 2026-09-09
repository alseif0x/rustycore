//! Object update-block regressions, part 4 of 5.
//!
//! Moved out of the update_tests.rs root under #640; every test is unchanged.

use super::*;

#[test]
fn create_player_self_current_power_can_use_saved_db_value_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut combat = PlayerCombatStats::default();
    combat.max_mana = 1000;
    combat.base_mana = 155;
    let mut packet = UpdateObject::create_player(
        guid,
        1,
        5,
        0,
        1,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        combat,
        Vec::new(),
        0,
        Vec::new(),
    );

    packet.set_player_current_power0_like_cpp(321);

    let UpdateBlock::CreateObject { create_data, .. } = &packet.blocks[0] else {
        panic!("create_player should emit one CreateObject block");
    };
    assert_eq!(
        create_data.current_power_for_slot0(),
        321,
        "C++ login self UpdateObject serializes current UnitData::Power[0] from characters.power1"
    );
    assert_eq!(
        create_data.max_power_for_slot0(),
        1000,
        "current power must not overwrite UnitData::MaxPower[0]"
    );
    assert_eq!(
        create_data.base_mana_for_create_like_cpp(),
        155,
        "C++ BaseMana keeps GtBaseMP separate from intellect-inflated MaxPower"
    );
}

#[test]
fn create_player_self_xp_can_use_saved_db_value_like_cpp() {
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut packet = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        10,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );

    packet.set_player_xp_like_cpp(1_234);
    packet.set_player_max_level_like_cpp(70);
    packet.set_player_scaling_level_delta_like_cpp(-1);

    let UpdateBlock::CreateObject { create_data, .. } = &packet.blocks[0] else {
        panic!("create_player should emit one CreateObject block");
    };
    assert_eq!(create_data.xp, 1_234);
    assert_eq!(create_data.max_level, 70);
    assert_eq!(create_data.scaling_player_level_delta, -1);

    let mut active = WorldPacket::new_empty();
    create_data.write_active_player_data(&mut active);
    let active = active.into_data();
    assert_eq!(
        i32::from_le_bytes(active[298..302].try_into().unwrap()),
        1_234
    );
    assert_eq!(
        i32::from_le_bytes(active[6_444..6_448].try_into().unwrap()),
        70
    );
    assert_eq!(
        i32::from_le_bytes(active[6_448..6_452].try_into().unwrap()),
        -1
    );
}

#[test]
fn create_player_non_self() {
    // Non-self player should be smaller (no ActivePlayerData)
    let guid = ObjectGuid::create_player(1, 42);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let self_pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        true,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let other_pkt = UpdateObject::create_player(
        guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );
    let self_bytes = self_pkt.to_bytes();
    let other_bytes = other_pkt.to_bytes();
    // Self packet should be much larger due to ActivePlayerData
    assert!(
        self_bytes.len() > other_bytes.len() + 1000,
        "Self ({}) should be much larger than other ({})",
        self_bytes.len(),
        other_bytes.len()
    );
}

#[test]
fn power_type_mapping() {
    assert_eq!(power_type_for_class(1), 1); // Warrior → Rage
    assert_eq!(power_type_for_class(2), 0); // Paladin → Mana
    assert_eq!(power_type_for_class(4), 3); // Rogue → Energy
    // DeathKnight DisplayPower = POWER_RUNIC_POWER (6), NOT POWER_RUNES (5) — C++
    // CalculateDisplayPowerType / ChrClasses (SharedDefines.h:287). #NEXT.R8.ENTITIES.1213.
    assert_eq!(power_type_for_class(6), 6); // DK → Runic Power
}

#[test]
fn player_unit_data_health_aura_state_matches_cpp_modify_aura_state() {
    // #NEXT.R8.ENTITIES.1212 — C++ Unit::Update/ModifyAuraState seeds health-based aura
    // states on EVERY alive unit incl. the player (Unit.cpp:469-476). Full HP => 0x00D00000.
    assert_eq!(health_aura_state_like_cpp(100, 100, true), 0x00D0_0000);
    assert_eq!(health_aura_state_like_cpp(0, 100, false), 0); // dead
    assert_eq!(health_aura_state_like_cpp(50, 0, true), 0); // no max
    // Low HP (<=20%): WOUND_HEALTH_20_80 (0x100000) set, HEALTHY_75 clear.
    let low = health_aura_state_like_cpp(10, 100, true);
    assert_ne!(low & 0x0010_0000, 0);
    assert_eq!(low & 0x0040_0000, 0);
}

#[test]
fn creature_create_serializes() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 1234, 5678);
    let pos = Position::new(-8949.0, -132.0, 83.0, 0.0);
    let data = CreatureCreateData {
        guid,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
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
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let mut values = WorldPacket::new_empty();
    data.write_values_create(&mut values);
    let values = values.into_data();
    assert_eq!(
        values.len(),
        536,
        "C++ Unit::BuildValuesCreate writes size prefix plus 532 bytes for base creature ObjectData+UnitData"
    );
    // Regression for the world-entry ERROR #132 render-worker NULL deref: every creature
    // CREATE must carry StateAnimID = 1772 (C++ Creature::UpdateEntry seeds
    // DB2Manager::GetEmptyAnimStateID(); DB2Stores.cpp:1765 hardcodes 1772 because the
    // Classic client expects the retail AnimationData storage size). StateSpellVisualID
    // and StateAnimKitID stay 0. Shipping StateAnimID=0 crashed the 3.4.3 client ~4s in-world.
    assert!(
        values
            .windows(12)
            .any(|w| w == [0, 0, 0, 0, 0xEC, 0x06, 0, 0, 0, 0, 0, 0]),
        "creature CREATE must serialize StateSpellVisualID=0, StateAnimID=1772, StateAnimKitID=0"
    );
    assert_eq!(&values[0..4], &532u32.to_le_bytes());
    assert_eq!(values[4], 0, "creature create uses no owner/party flags");

    let block = UpdateObject::create_creature_block(data.clone(), &pos);
    let pkt = UpdateObject::create_creatures(vec![block], 0);
    let bytes = pkt.to_bytes();
    // Creature packet should be much smaller than player (no PlayerData/ActivePlayerData)
    assert!(
        bytes.len() > 100,
        "Creature packet too small: {} bytes",
        bytes.len()
    );
    assert!(
        bytes.len() < 2000,
        "Creature packet too large: {} bytes",
        bytes.len()
    );

    let mut vehicle_data = data.clone();
    vehicle_data.vehicle_id = 686;
    let normal_block = UpdateObject::create_creature_block(data, &pos);
    let vehicle_block = UpdateObject::create_creature_block(vehicle_data, &pos);
    let normal = UpdateObject::create_creatures(vec![normal_block], 0).to_bytes();
    let vehicle = UpdateObject::create_creatures(vec![vehicle_block], 0).to_bytes();
    let mut expected_vehicle_payload = Vec::new();
    expected_vehicle_payload.extend_from_slice(&686u32.to_le_bytes());
    expected_vehicle_payload.extend_from_slice(&pos.orientation.to_le_bytes());
    assert_eq!(
        vehicle.len(),
        normal.len() + 8,
        "C++ CreateObjectBits::Vehicle writes VehicleRecID plus InitialRawFacing"
    );
    assert!(
        vehicle
            .windows(expected_vehicle_payload.len())
            .any(|window| window == expected_vehicle_payload),
        "vehicle create block must include the C++ VehicleRecID/orientation payload"
    );
}

#[test]
fn creature_create_serializes_cpp_anim_kit_block_when_any_anim_kit_is_set() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 1234, 5678);
    let pos = Position::new(-8949.0, -132.0, 83.0, 0.0);
    let data = CreatureCreateData {
        guid,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
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
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 11,
        movement_anim_kit_id: 22,
        melee_anim_kit_id: 33,
    };
    let block = UpdateObject::create_creature_block(data, &pos);
    let pkt = UpdateObject::create_creatures(vec![block], 0);
    let bytes = pkt.to_bytes();

    assert!(
        bytes
            .windows(6)
            .any(|window| window == [11, 0, 22, 0, 33, 0]),
        "C++ CreateObjectBits::AnimKit writes AiID, MovementID, MeleeID as u16 payload"
    );
}

#[test]
fn creature_create_serializes_cpp_addon_unit_fields() {
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 1234, 1);
    let pos = Position::new(1.0, 2.0, 3.0, 4.0);
    let data = CreatureCreateData {
        guid,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
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
        hover_height: 1.25,
        mount_display_id: 0x0102_0304,
        stand_state: 2,
        vis_flags: 0x12,
        anim_tier: 3,
        emote_state: 0x1122_3344,
        sheathe_state: 1,
        pvp_flags: 5,
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let bytes =
        UpdateObject::create_creatures(vec![UpdateObject::create_creature_block(data, &pos)], 0)
            .to_bytes();

    assert!(
        bytes.windows(4).any(|window| window == [4, 3, 2, 1]),
        "C++ UnitData::WriteCreate writes MountDisplayID after native display scale"
    );
    assert!(
        bytes
            .windows(8)
            .any(|window| window == [2, 0, 0x12, 3, 0, 0, 0, 0]),
        "C++ UnitData::WriteCreate writes StandState/PetTalentPoints/VisFlags/AnimTier followed by PetNumber"
    );
    assert!(
        !bytes
            .windows(5)
            .any(|window| window == [2, 0, 0x12, 0x12, 3]),
        "C++ UnitData::WriteCreate writes VisFlags once, not twice"
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == [0x44, 0x33, 0x22, 0x11]),
        "C++ UnitData::WriteCreate writes EmoteState"
    );
    let mut mod_time_rate_sequence = Vec::new();
    for _ in 0..6 {
        mod_time_rate_sequence.extend_from_slice(&1.0f32.to_le_bytes());
    }
    mod_time_rate_sequence.extend_from_slice(&0i32.to_le_bytes());
    mod_time_rate_sequence.extend_from_slice(&0x1122_3344i32.to_le_bytes());
    assert!(
        bytes
            .windows(mod_time_rate_sequence.len())
            .any(|window| window == mod_time_rate_sequence),
        "C++ UnitData::WriteCreate writes six speed/haste/time-rate floats before CreatedBySpell and EmoteState"
    );
    assert!(
        bytes.windows(4).any(|window| window == [1, 5, 0, 0]),
        "C++ UnitData::WriteCreate writes SheatheState/PvpFlags/PetFlags/ShapeshiftForm"
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == 1.25f32.to_le_bytes()),
        "C++ UnitData::WriteCreate writes UnitData::HoverHeight from CreatureModelData"
    );
}

#[test]
fn creature_create_serializes_cpp_power_and_max_power_arrays() {
    let mut power = [0; 10];
    let mut max_power = [0; 10];
    power[0] = 77;
    max_power[0] = 123;
    let data = CreatureCreateData {
        guid: ObjectGuid::EMPTY,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 0,
        power,
        max_power,
        base_mana: 123,
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
    let bytes = UpdateObject::create_creatures(
        vec![UpdateObject::create_creature_block(data, &Position::ZERO)],
        0,
    )
    .to_bytes();
    let mut expected = Vec::new();
    expected.extend_from_slice(&77i32.to_le_bytes());
    expected.extend_from_slice(&123i32.to_le_bytes());
    expected.extend_from_slice(&0.0f32.to_le_bytes());
    assert!(
        bytes
            .windows(expected.len())
            .any(|window| window == expected),
        "C++ UnitData::WriteCreate writes Power[0], MaxPower[0], ModPowerRegen[0]"
    );
}

#[test]
fn creature_create_serializes_play_hover_anim_bit_like_cpp() {
    let mut data = CreatureCreateData {
        guid: ObjectGuid::EMPTY,
        entry: 1234,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 5,
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
        movement_flags: wow_constants::movement::MovementFlag::HOVER.bits(),
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.25,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let movement = MovementBlock {
        position: Position::ZERO,
        movement_flags: data.movement_flags,
        ..Default::default()
    };
    let mut without_hover = WorldPacket::new_empty();
    write_creature_create_block(&mut without_hover, &ObjectGuid::EMPTY, &movement, &data);
    data.play_hover_anim = true;
    let mut with_hover = WorldPacket::new_empty();
    write_creature_create_block(&mut with_hover, &ObjectGuid::EMPTY, &movement, &data);

    let diffs = without_hover
        .data()
        .iter()
        .zip(with_hover.data())
        .filter_map(|(left, right)| {
            let diff = left ^ right;
            (diff != 0).then_some(diff)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        diffs,
        vec![1 << 5],
        "the third C++ CreateObjectBits WriteBit toggles PlayHoverAnim"
    );
}

#[test]
fn creature_smaller_than_player() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 100, 1);
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);

    let player_pkt = UpdateObject::create_player(
        player_guid,
        1,
        1,
        0,
        1,
        49,
        &pos,
        0,
        12,
        false,
        [(0, 0, 0); 19],
        [ObjectGuid::EMPTY; 141],
        PlayerCombatStats::default(),
        Vec::new(),
        0,
        Vec::new(),
    );

    let creature_data = CreatureCreateData {
        guid: creature_guid,
        entry: 100,
        display_id: 856,
        native_display_id: 856,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 100,
        max_health: 100,
        level: 1,
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
        current_area_id: 12,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let block = UpdateObject::create_creature_block(creature_data, &pos);
    let creature_pkt = UpdateObject::create_creatures(vec![block], 0);

    let player_bytes = player_pkt.to_bytes();
    let creature_bytes = creature_pkt.to_bytes();

    // Creature has no PlayerData, so it should be smaller than even a non-self player
    assert!(
        creature_bytes.len() < player_bytes.len(),
        "Creature ({}) should be smaller than non-self player ({})",
        creature_bytes.len(),
        player_bytes.len()
    );
}

#[test]
fn creature_batched_multiple() {
    let pos = Position::new(0.0, 0.0, 0.0, 0.0);
    let mut blocks = Vec::new();
    for i in 0..5 {
        let guid =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 100, i);
        let data = CreatureCreateData {
            guid,
            entry: 100,
            display_id: 856,
            native_display_id: 856,
            display_scale: 1.0,
            native_x_display_scale: 1.0,
            bounding_radius: 0.389,
            combat_reach: 1.5,
            health: 100,
            max_health: 100,
            level: 1,
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
            current_area_id: 12,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        };
        blocks.push(UpdateObject::create_creature_block(data, &pos));
    }
    let pkt = UpdateObject::create_creatures(blocks, 0);
    let bytes = pkt.to_bytes();

    // 5 creatures should be 5x the single creature data
    assert!(
        bytes.len() > 500,
        "Batched packet too small: {} bytes",
        bytes.len()
    );

    // Check num_updates = 5
    let num_updates = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(num_updates, 5);
}

#[test]
fn creature_npc_flags_written_correctly() {
    // Verify that NpcFlags value appears in the creature's values block.
    // NpcFlags=1 (Gossip) should be written as 0x01000000 in the packet.
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 3296, 1);
    let pos = Position::new(1600.0, -4400.0, 10.0, 0.0);
    let data = CreatureCreateData {
        guid,
        entry: 3296,
        display_id: 4500,
        native_display_id: 4500,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 500,
        max_health: 500,
        level: 55,
        faction_template: 85,
        npc_flags: 0x1_0000_0001, // Gossip flag plus NPCFlags2 bit 0
        unit_flags: 32768,
        unit_flags2: 2048,
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
        current_area_id: 1637,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };
    let block = UpdateObject::create_creature_block(data, &pos);
    let pkt = UpdateObject::create_creatures(vec![block], 1);
    let bytes = pkt.to_bytes();

    // Find NpcFlags=1 in the packet bytes.
    // The values block contains:
    //   [u8 flags=0x00]
    //   [i32 EntryId] [u32 DynamicFlags] [f32 Scale]  (ObjectData: 4+4+4=12 bytes)
    //   [i64 Health] [i64 MaxHealth] [i32 DisplayId]   (UnitData: 8+8+4=20 bytes)
    //   [u32 NpcFlags[0]] [u32 NpcFlags[1]]            (UnitData: 4+4=8 bytes)
    // So NpcFlags[0] starts at offset 1+12+20 = 33 from values block start.
    // The value 1 in little-endian is [0x01, 0x00, 0x00, 0x00].
    // Search for this pattern preceded by DisplayId (4500 = 0x94110000 LE).
    let display_le = 4500u32.to_le_bytes();
    let npc_le = 1u32.to_le_bytes();
    let npc2_le = 1u32.to_le_bytes();
    let mut found = false;
    for i in 0..bytes.len().saturating_sub(8) {
        if bytes[i..i + 4] == display_le && bytes[i + 4..i + 8] == npc_le {
            found = true;
            // Also check NpcFlags[1] = 1
            assert_eq!(bytes[i + 8..i + 12], npc2_le, "NpcFlags[1] should be 1");
            break;
        }
    }
    assert!(
        found,
        "NpcFlags=1 not found after DisplayId={} in packet ({} bytes). \
        This means NpcFlags are not being written correctly!",
        4500,
        bytes.len()
    );
}
