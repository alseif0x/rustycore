//! Shared fixtures for the spell handler scenarios.
//!
//! Split out of the inline test module under #624; behaviour unchanged.

use super::*;

pub(super) fn make_session() -> (crate::session::WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(100);
    let (send_tx, send_rx) = flume::bounded(100);

    (
        crate::session::WorldSession::new(
            1,
            "TestAccount".into(),
            0,
            2,
            9,
            54261,
            vec![0u8; 40],
            "esES".into(),
            pkt_rx,
            send_tx,
        ),
        send_rx,
    )
}
pub(super) fn shared_canonical_map_manager() -> SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}
pub(super) fn add_canonical_test_player_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("SpellHandlerPlayer");
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
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}
pub(super) fn install_canonical_player(
    session: &mut crate::session::WorldSession,
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
) {
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "SpellHandlerPlayer".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(canonical, player_guid, position, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session.set_player_moved_unit_guid_like_cpp(player_guid);
    session.set_legacy_creature_aggro_config_like_cpp(
        crate::session::LegacyCreatureAggroConfigLikeCpp {
            spell_x_spell_visual_store: Some(Arc::new(
                wow_data::SpellXSpellVisualStore::from_entries([]),
            )),
            ..Default::default()
        },
    );
}
pub(super) fn add_canonical_test_pet_on_map(
    canonical: &SharedCanonicalMapManager,
    owner_guid: ObjectGuid,
    pet_guid: ObjectGuid,
    spell_id: u32,
    alive: bool,
) {
    let mut pet = Pet::new(owner_guid, PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .relocate(Position::new(10.5, 20.5, 30.0, 0.0));
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    pet.creature_mut().unit_mut().set_max_health(100);
    pet.creature_mut()
        .unit_mut()
        .set_health(if alive { 100 } else { 0 });
    if !alive {
        pet.creature_mut()
            .unit_mut()
            .set_death_state(DeathState::Dead);
    }
    let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
    pet.creature_mut()
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(aura);

    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
}
pub(super) fn add_canonical_test_creature_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    is_totem: bool,
) {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(777);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    if is_totem {
        creature.add_unit_type_mask_like_cpp(UNIT_MASK_TOTEM);
    }

    canonical
        .lock()
        .unwrap()
        .find_map_mut(map_id, instance_id)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}
pub(super) fn set_canonical_player_pet_guid(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    pet_guid: ObjectGuid,
) {
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .control
        .set_pet_guid(pet_guid);
}
pub(super) fn set_canonical_player_charmed_guid(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    charmed_guid: ObjectGuid,
) {
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .control
        .charmed_guid = Some(charmed_guid);
}
pub(super) fn canonical_pet_has_applied_aura(
    canonical: &SharedCanonicalMapManager,
    pet_guid: ObjectGuid,
    aura: AppliedAuraRef,
) -> bool {
    canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_pet(pet_guid)
        .unwrap()
        .creature()
        .unit()
        .subsystems()
        .auras
        .has_applied(aura)
}
pub(super) fn canonical_creature_has_applied_aura(
    canonical: &SharedCanonicalMapManager,
    creature_guid: ObjectGuid,
    aura: AppliedAuraRef,
) -> bool {
    canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(creature_guid, |creature| {
            creature.unit().subsystems().auras.has_applied(aura)
        })
        .unwrap()
}
pub(super) fn set_canonical_player_summon_slot(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    slot: usize,
    guid: ObjectGuid,
) {
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .control
        .set_summon_slot(slot, guid);
}
pub(super) fn canonical_player_summon_slot(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    slot: usize,
) -> ObjectGuid {
    canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(player_guid)
        .unwrap()
        .unit()
        .subsystems()
        .control
        .summon_slots[slot]
}
pub(super) fn canonical_creature_exists(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> bool {
    canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, |_| ())
        .is_some()
}
pub(super) fn install_active_spell_cast(
    session: &mut crate::session::WorldSession,
    spell_id: i32,
    cast_id: ObjectGuid,
) {
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_active_spell_cast_like_cpp(Some(SpellCastState {
        spell_id,
        target_guid: player_guid,
        target_data: wow_entities::SpellCastTargetsLikeCpp {
            flags: 0x2, // SpellCastTargetFlags::Unit
            unit: player_guid,
            ..Default::default()
        },
        cast_id,
        cast_start_time: std::time::Instant::now(),
        cast_time_ms: 30_000,
        spell_visual: wow_entities::SpellCastVisualLikeCpp {
            spell_visual_id: 1,
            script_visual_id: 0,
        },
        metadata: SpellCastMetadata::default(),
    }));
}
pub(super) fn install_pending_spell_cast_request(
    session: &mut crate::session::WorldSession,
    spell_id: i32,
    cast_id: ObjectGuid,
) {
    session.request_represented_spell_cast_like_cpp(RepresentedPendingSpellCastRequestLikeCpp {
        cast_id,
        spell_id,
        casting_unit_guid: ObjectGuid::create_player(1, 42),
        target_guid: ObjectGuid::create_player(1, 42),
        target_data: wow_entities::SpellCastTargetsLikeCpp {
            flags: 0x2,
            unit: ObjectGuid::create_player(1, 42),
            ..Default::default()
        },
        spell_visual: wow_entities::SpellCastVisualLikeCpp {
            spell_visual_id: 0,
            script_visual_id: 0,
        },
        metadata: SpellCastMetadata::default(),
    });
}
pub(super) fn install_canonical_channeled_spell(
    session: &mut crate::session::WorldSession,
    player_guid: ObjectGuid,
    spell_id: u32,
) -> wow_entities::CurrentSpellRef {
    let spell = wow_entities::CurrentSpellRef::new(spell_id, Some(player_guid), None)
        .with_state(wow_constants::SpellState::Delayed);
    session.mutate_canonical_player_like_cpp(|player| {
        player
            .unit_mut()
            .set_current_cast_spell(wow_entities::CurrentSpellSlot::Channeled, spell);
    });
    spell
}
pub(super) fn canonical_channeled_spell_id(
    session: &mut crate::session::WorldSession,
) -> Option<u32> {
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit()
                .current_spell(wow_entities::CurrentSpellSlot::Channeled)
                .map(|spell| spell.spell_id)
        })
        .flatten()
}
pub(super) fn cancel_cast_packet(cast_id: ObjectGuid, spell_id: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&cast_id);
    pkt.write_uint32(spell_id);
    pkt.reset_read();
    pkt
}
pub(super) fn cancel_channelling_packet(channel_spell: i32, reason: i32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(channel_spell);
    pkt.write_int32(reason);
    pkt.reset_read();
    pkt
}
pub(super) fn cast_spell_packet(spell_id: i32, caster_guid: ObjectGuid) -> WorldPacket {
    cast_spell_packet_with_move_update(spell_id, caster_guid, None)
}
pub(super) fn cast_spell_packet_with_move_update(
    spell_id: i32,
    caster_guid: ObjectGuid,
    move_update: Option<MovementInfo>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&caster_guid);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(spell_id);
    SpellCastVisual {
        spell_visual_id: 0,
        script_visual_id: 0,
    }
    .write(&mut pkt);
    pkt.write_float(0.0);
    pkt.write_float(0.0);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_bits(0, 5);
    pkt.write_bit(move_update.is_some());
    pkt.write_bits(0, 2);
    pkt.write_bit(false);
    pkt.flush_bits();
    SpellTargetData {
        flags: 0x2,
        unit: caster_guid,
        ..SpellTargetData::default()
    }
    .write(&mut pkt);
    if let Some(move_update) = move_update {
        move_update.write(&mut pkt);
    }
    pkt.reset_read();
    pkt
}
pub(super) fn set_canonical_player_mana_like_cpp(
    session: &mut crate::session::WorldSession,
    current: i32,
    max: i32,
) {
    session.mutate_canonical_player_like_cpp(|player| {
        player.set_power_index(PowerType::Mana, Some(0));
        player.unit_mut().set_create_mana_like_cpp(max);
        player.unit_mut().set_max_power(PowerType::Mana, max);
        player.unit_mut().set_power(PowerType::Mana, current);
    });
}
pub(super) fn canonical_player_mana_like_cpp(session: &mut crate::session::WorldSession) -> i32 {
    session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
        .unwrap_or(0)
}
pub(super) fn canonical_player_power_like_cpp(
    session: &mut crate::session::WorldSession,
    power: PowerType,
) -> i32 {
    session
        .mutate_canonical_player_like_cpp(|player| player.get_power(power))
        .unwrap_or(0)
}
pub(super) fn active_shapeshift_aura_for_test(
    spell_id: i32,
    caster_guid: ObjectGuid,
) -> AuraApplication {
    AuraApplication {
        spell_id,
        difficulty_id: 0,
        caster_guid,
        slot: 0,
        duration_total: 0,
        duration_remaining: 0,
        stack_count: 1,
        aura_flags: 0x0000_0001,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    }
}
pub(super) fn set_canonical_player_display_for_test(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    display_id: u32,
    set_native: bool,
) {
    let mut manager = canonical.lock().unwrap();
    let player = manager
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player_guid)
        .unwrap();
    player.unit_mut().set_display_id(display_id, set_native);
}
pub(super) fn creature_display_info_extra_for_test(
    id: u32,
    display_race_id: i8,
) -> wow_data::CreatureDisplayInfoExtraEntry {
    wow_data::CreatureDisplayInfoExtraEntry {
        id,
        display_race_id,
        display_sex_id: 0,
        display_class_id: 0,
        skin_id: 0,
        face_id: 0,
        hair_style_id: 0,
        hair_color_id: 0,
        facial_hair_id: 0,
        flags: 0,
        bake_material_resources_id: 0,
        hd_bake_material_resources_id: 0,
        custom_display_option: [0; 3],
    }
}
pub(super) fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        if let Some(opcode) = WorldPacket::from_bytes(&bytes).server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}
pub(super) fn drain_server_packet_bytes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        packets.push(bytes);
    }
    packets
}
pub(super) fn cast_failed_reason_like_cpp(bytes: &[u8]) -> i32 {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::CastFailed),
        "expected CastFailed packet"
    );
    let _ = packet.read_uint16().expect("opcode");
    let _ = packet.read_packed_guid().expect("cast id");
    let _ = packet.read_int32().expect("spell id");
    let _ = SpellCastVisual::read(&mut packet).expect("visual");
    packet.read_int32().expect("reason")
}
pub(super) fn cast_failed_fields_like_cpp(bytes: &[u8]) -> (ObjectGuid, i32, i32) {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::CastFailed),
        "expected CastFailed packet"
    );
    let _ = packet.read_uint16().expect("opcode");
    let cast_id = packet.read_packed_guid().expect("cast id");
    let spell_id = packet.read_int32().expect("spell id");
    let _ = SpellCastVisual::read(&mut packet).expect("visual");
    let reason = packet.read_int32().expect("reason");
    (cast_id, spell_id, reason)
}
pub(super) fn spell_go_spell_id_like_cpp(bytes: &[u8]) -> i32 {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::SpellGo),
        "expected SpellGo packet"
    );
    let _ = packet.read_uint16().expect("opcode");
    let _ = packet.read_packed_guid().expect("caster");
    let _ = packet.read_packed_guid().expect("caster unit");
    let _ = packet.read_packed_guid().expect("cast id");
    let _ = packet.read_packed_guid().expect("original cast id");
    packet.read_int32().expect("spell id")
}
pub(super) fn mount_result_like_cpp(bytes: &[u8]) -> i32 {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::MountResult),
        "expected MountResult packet"
    );
    let _ = packet.read_uint16().expect("opcode");
    packet.read_int32().expect("result")
}
pub(super) fn cancel_aura_packet(spell_id: i32, caster_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(spell_id);
    pkt.write_packed_guid(&caster_guid);
    pkt.reset_read();
    pkt
}
pub(super) fn cancel_mod_speed_no_control_packet(target_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&target_guid);
    pkt.reset_read();
    pkt
}
pub(super) fn int32_spell_packet(spell_id: i32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(spell_id);
    pkt.reset_read();
    pkt
}
pub(super) fn pet_cancel_aura_packet(pet_guid: ObjectGuid, spell_id: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&pet_guid);
    pkt.write_uint32(spell_id);
    pkt.reset_read();
    pkt
}
pub(super) fn totem_destroyed_packet(slot: u8, totem_guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(slot);
    pkt.write_packed_guid(&totem_guid);
    pkt.reset_read();
    pkt
}
pub(super) fn install_canonical_totem_for_session(
    session: &mut crate::session::WorldSession,
    canonical: &SharedCanonicalMapManager,
    slot: usize,
    is_totem: bool,
) -> (ObjectGuid, ObjectGuid) {
    let player_guid = ObjectGuid::create_player(1, 42);
    let totem_guid =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, slot as i64);
    install_canonical_player(session, canonical, player_guid);
    set_canonical_player_summon_slot(canonical, player_guid, slot, totem_guid);
    add_canonical_test_creature_on_map(
        canonical,
        totem_guid,
        Position::new(10.5, 20.5, 30.0, 0.0),
        571,
        0,
        is_totem,
    );
    (player_guid, totem_guid)
}
pub(super) fn condition(
    else_group: u32,
    condition_type_or_reference: i32,
    value1: u32,
    negative: bool,
) -> LootConditionRowLikeCpp {
    LootConditionRowLikeCpp {
        else_group,
        condition_type_or_reference,
        condition_target: 0,
        value1,
        value2: 0,
        value3: 0,
        string_value1: String::new(),
        negative,
        script_name: String::new(),
    }
}
