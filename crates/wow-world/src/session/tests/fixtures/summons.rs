//! Summoning and teleport spell fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn summon_go_template_store_like_cpp(
    entry: u32,
) -> Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp> {
    Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry,
                go_type: 6,
                display_id: 44,
                name: "spell summoned gameobject".to_string(),
                size: 1.0,
                data: [0; wow_entities::MAX_GAMEOBJECT_DATA],
                content_tuning_id: 80,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: String::new(),
                addon: None,
            },
        ]),
    )
}

pub(in crate::session::tests) fn summon_go_spell_misc_entry_like_cpp(
    spell_id: u32,
    duration_index: u16,
) -> wow_data::SpellMiscEntry {
    wow_data::SpellMiscEntry {
        id: spell_id,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index,
        range_index: 0,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id,
    }
}

pub(in crate::session::tests) fn summon_object_wild_effect_like_cpp(
    entry: i32,
) -> wow_data::SpellEffectInfo {
    wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD,
        effect_misc_value_1: entry,
        ..Default::default()
    }
}

pub(in crate::session::tests) fn summon_object_slot_effect_like_cpp(
    entry: i32,
    slot: u32,
) -> wow_data::SpellEffectInfo {
    wow_data::SpellEffectInfo {
        effect_index: slot,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_SLOT1 + slot,
        effect_misc_value_1: entry,
        ..Default::default()
    }
}

pub(in crate::session::tests) fn gameobject_summon_spell_info_like_cpp(
    spell_id: i32,
    requires_spell_focus: u32,
    effects: Vec<wow_data::SpellEffectInfo>,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus,
        power_costs: Vec::new(),
        effects,
    }
}

pub(in crate::session::tests) fn teleport_units_spell_info_like_cpp(
    spell_id: i32,
    effects: Vec<wow_data::SpellEffectInfo>,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects,
    }
}

pub(in crate::session::tests) fn bind_spell_info_like_cpp(
    spell_id: i32,
    effects: Vec<wow_data::SpellEffectInfo>,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects,
    }
}

pub(in crate::session::tests) fn environmental_damage_spell_info_like_cpp(
    spell_id: i32,
    effects: Vec<wow_data::SpellEffectInfo>,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects,
    }
}

pub(in crate::session::tests) fn configure_gameobject_summon_live_session_like_cpp(
    session: &mut WorldSession,
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    player_position: Position,
    template_store: Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    spell_info: wow_data::SpellInfo,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Summoner".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(canonical, player_guid, player_position, 571, 0);
    session.set_gameobject_template_lifecycle_store(template_store);
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        summon_go_spell_misc_entry_like_cpp(spell_info.spell_id as u32, 0),
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 0,
            duration: 0,
            duration_per_level: 0,
            max_duration: 0,
        },
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_info.spell_id, spell_info);
    session.set_spell_store(Arc::new(spell_store));
}

pub(in crate::session::tests) fn target_data_with_destination_like_cpp(
    position: Position,
) -> SpellTargetData {
    SpellTargetData {
        dst_location: Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position,
        }),
        ..Default::default()
    }
}

pub(in crate::session::tests) fn insert_test_player_into_canonical_map_like_cpp(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    position: Position,
) {
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();

    let mut manager = canonical.lock().unwrap();
    manager
        .create_world_map(map_id, instance_id)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_player(player).unwrap(),
        )
        .unwrap();
}

pub(in crate::session::tests) fn decode_spell_go_target_data_like_cpp(
    bytes: &[u8],
    expected_spell_id: i32,
) -> SpellTargetData {
    let mut spell_go = WorldPacket::from_bytes(bytes);
    assert_eq!(
        spell_go.read_uint16().expect("SpellGo opcode"),
        ServerOpcodes::SpellGo as u16
    );
    for label in ["caster", "caster unit", "cast id", "original cast id"] {
        spell_go.read_packed_guid().expect(label);
    }
    assert_eq!(spell_go.read_int32().expect("spell id"), expected_spell_id);
    wow_packet::packets::spell::SpellCastVisual::read(&mut spell_go).expect("spell visual");
    spell_go.read_uint32().expect("cast flags");
    spell_go.read_uint32().expect("cast flags ex");
    spell_go.read_uint32().expect("cast time");
    spell_go.read_int32().expect("missile travel time");
    spell_go.read_float().expect("missile pitch");
    spell_go.read_uint8().expect("destination cast index");
    spell_go.read_uint32().expect("immunity school");
    spell_go.read_uint32().expect("immunity value");
    spell_go.read_uint32().expect("heal prediction points");
    spell_go.read_uint8().expect("heal prediction type");
    spell_go.read_packed_guid().expect("heal prediction beacon");
    spell_go.read_bits(16).expect("hit target count");
    spell_go.read_bits(16).expect("miss target count");
    spell_go.read_bits(16).expect("miss status count");
    spell_go.read_bits(9).expect("remaining power count");
    spell_go.read_bit().expect("remaining runes presence");
    spell_go.read_bits(16).expect("target point count");
    spell_go.read_bit().expect("ammo display presence");
    spell_go.read_bit().expect("ammo inventory presence");
    SpellTargetData::read(&mut spell_go).expect("SpellGo target data")
}
