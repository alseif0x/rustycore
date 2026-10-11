//! Session-free creature AI fixtures: aggro candidates and configs, spell catalog rows and
//! the creature spell wire decoders used by the creature spell scenarios.
//!
//! Moved from `wow-world` `session::tests::fixtures::creature_ai` by #1363 slice 0.

use std::sync::Arc;
use wow_constants::UnitFlags;
use wow_core::ObjectGuid;
use wow_core::Position;
use wow_data::progression_rewards::FactionEntry;
use wow_data::progression_rewards::FactionStore;
use wow_entities::PhaseShift;
use wow_entities::UnitVisibilityDetectionStateLikeCpp;
use wow_packet::WorldPacket;
use wow_world_core::session::SharedCanonicalMapManager;
use wow_world_entities::creature_aggro_contracts::{
    CreatureAggroCandidateLikeCpp, CreatureAggroConfigLikeCpp,
};

use crate::map_objects::{
    add_canonical_test_creature_on_map, add_canonical_test_player_on_map_with_difficulty,
};

pub fn aggro_candidate_like_cpp(
    player_guid: ObjectGuid,
    position: Position,
) -> CreatureAggroCandidateLikeCpp {
    CreatureAggroCandidateLikeCpp {
        player_guid,
        map_id: 0,
        instance_id: 0,
        map_difficulty_id: 0,
        position,
        player_visibility_represented: true,
        player_phase_shift: PhaseShift::default(),
        player_visibility_detection: UnitVisibilityDetectionStateLikeCpp::default(),
        player_combat_reach: 0.0,
        player_detected_range_aura_mod: 0.0,
        player_level: 80,
        player_gray_level: 70,
        player_liquid_status_like_cpp: 0,
        player_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
        player_unit_flags2: 0,
        player_unit_state: 0,
        player_is_game_master: false,
        player_is_contested_pvp: false,
        player_faction_template_id: 1,
        player_reputation_standings: Vec::new(),
        player_reputation_state_flags: Vec::new(),
        player_forced_reputation_ranks: Vec::new(),
        player_forced_reputation_faction_ids: Vec::new(),
        player_school_immunity_mask: 0,
        player_damage_immunity_mask: 0,
        player_has_confuse_aura: false,
        player_has_breakable_stun_aura: false,
    }
}

pub fn aggro_relation_config_like_cpp(
    creature_faction_template: wow_data::progression_rewards::FactionTemplateEntry,
    player_faction_template: wow_data::progression_rewards::FactionTemplateEntry,
    creature_faction: FactionEntry,
) -> CreatureAggroConfigLikeCpp {
    CreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                creature_faction_template,
                player_faction_template,
            ]),
        )),
        faction_store: Some(Arc::new(FactionStore::from_entries([creature_faction]))),
        ..Default::default()
    }
}

pub fn represented_creature_spell_test_attributes_like_cpp(no_attack_miss: bool) -> [u32; 15] {
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY;
    if no_attack_miss {
        attributes[7] = 0x0200_0000; // SPELL_ATTR7_NO_ATTACK_MISS
    }
    attributes
}

pub fn spell_misc_entry_like_cpp(
    id: u32,
    spell_id: u32,
    range_index: u16,
) -> wow_data::SpellMiscEntry {
    wow_data::SpellMiscEntry {
        id,
        attributes: represented_creature_spell_test_attributes_like_cpp(true)
            .map(|attribute| attribute as i32),
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index,
        school_mask: 0x01,
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

pub fn spell_range_entry_like_cpp(
    id: u32,
    range_min: f32,
    range_max: f32,
) -> wow_data::SpellRangeEntry {
    wow_data::SpellRangeEntry {
        id,
        display_name: String::new(),
        display_name_short: String::new(),
        flags: 0,
        range_min: [range_min, range_min],
        range_max: [range_max, range_max],
    }
}

pub fn creature_ai_test_spell_info_like_cpp(
    spell_id: i32,
    implicit_target_1: u32,
    implicit_target_2: u32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        // SpellStore hydrates GetRecoveryTime=max(RecoveryTime,
        // CategoryRecoveryTime). CombatAI must still use raw 6s below.
        recovery_time_ms: 12_000,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
        effect_base_points: 7,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: 7,
            implicit_target_1,
            implicit_target_2,
            ..Default::default()
        }],
    }
}

pub fn add_canonical_creature_spell_test_pair_like_cpp(
    canonical: &SharedCanonicalMapManager,
    creature_guid: ObjectGuid,
    victim_guid: ObjectGuid,
) {
    add_canonical_creature_spell_test_pair_with_difficulty_like_cpp(
        canonical,
        creature_guid,
        victim_guid,
        0,
    );
}

pub fn add_canonical_creature_spell_test_pair_with_difficulty_like_cpp(
    canonical: &SharedCanonicalMapManager,
    creature_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    difficulty_id: u8,
) {
    add_canonical_test_player_on_map_with_difficulty(
        canonical,
        victim_guid,
        Position::new(11.0, 10.0, 0.0, 0.0),
        0,
        0,
        difficulty_id,
    );
    add_canonical_test_creature_on_map(
        canonical,
        creature_guid,
        9_001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    let mut manager = canonical.lock().unwrap();
    let map = manager.find_map_mut(0, 0).unwrap().map_mut();
    let creature = map.get_typed_creature_mut(creature_guid).unwrap();
    creature.unit_mut().set_faction(14);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_cast_log_aura_authority_inert_like_cpp(true);
    let player = map.get_typed_player_mut(victim_guid).unwrap();
    player.unit_mut().set_faction(1);
    player.unit_mut().set_level(80);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player
        .unit_mut()
        .set_death_state(wow_constants::DeathState::Alive);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_hit_aura_authority_inert_like_cpp(true);
}

#[derive(Debug, PartialEq, Eq)]
pub struct CreatureSpellWireHeaderLikeCpp {
    pub opcode: u16,
    pub caster: ObjectGuid,
    pub caster_unit: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub original_cast_id: ObjectGuid,
    pub spell_id: i32,
    pub spell_x_spell_visual_id: u32,
    pub cast_flags: u32,
    pub cast_flags_ex: u32,
    pub cast_time_ms: u32,
    pub target_flags: u32,
    pub target_unit: ObjectGuid,
    pub hit_targets: Vec<ObjectGuid>,
    pub miss_targets: Vec<(ObjectGuid, u8)>,
}

pub fn decode_creature_spell_wire_header_and_tail_like_cpp(
    bytes: &[u8],
) -> (CreatureSpellWireHeaderLikeCpp, WorldPacket) {
    let mut packet = WorldPacket::from_bytes(bytes);
    let opcode = packet.opcode_raw();
    packet.skip_opcode();
    let caster = packet.read_packed_guid().unwrap();
    let caster_unit = packet.read_packed_guid().unwrap();
    let cast_id = packet.read_packed_guid().unwrap();
    let original_cast_id = packet.read_packed_guid().unwrap();
    let spell_id = packet.read_int32().unwrap();
    let spell_x_spell_visual_id = packet.read_uint32().unwrap();
    let cast_flags = packet.read_uint32().unwrap();
    let cast_flags_ex = packet.read_uint32().unwrap();
    let cast_time_ms = packet.read_uint32().unwrap();
    assert_eq!(packet.read_int32().unwrap(), 0); // missile travel time
    assert_eq!(packet.read_float().unwrap(), 0.0); // missile pitch
    assert_eq!(packet.read_uint8().unwrap(), 0); // destination cast index
    assert_eq!(packet.read_uint32().unwrap(), 0); // immunity school
    assert_eq!(packet.read_uint32().unwrap(), 0); // immunity value
    assert_eq!(packet.read_uint32().unwrap(), 0); // heal prediction
    assert_eq!(packet.read_uint8().unwrap(), 0); // prediction type
    assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    let hit_count = packet.read_bits(16).unwrap() as usize;
    let miss_count = packet.read_bits(16).unwrap() as usize;
    assert_eq!(packet.read_bits(16).unwrap() as usize, miss_count);
    assert_eq!(packet.read_bits(9).unwrap(), 0); // remaining power
    assert!(!packet.read_bit().unwrap()); // remaining runes
    assert_eq!(packet.read_bits(16).unwrap(), 0); // target points
    assert!(!packet.read_bit().unwrap()); // ammo display
    assert!(!packet.read_bit().unwrap()); // ammo inventory type
    let target = wow_packet::packets::spell::SpellTargetData::read(&mut packet).unwrap();
    let hit_targets = (0..hit_count)
        .map(|_| packet.read_packed_guid().unwrap())
        .collect();
    let miss_guids: Vec<_> = (0..miss_count)
        .map(|_| packet.read_packed_guid().unwrap())
        .collect();
    let miss_targets = miss_guids
        .into_iter()
        .map(|guid| {
            let reason = packet.read_uint8().unwrap();
            if reason == wow_packet::packets::spell::SpellMissReason::Reflect as u8 {
                let _reflect_status = packet.read_uint8().unwrap();
            }
            (guid, reason)
        })
        .collect();
    let header = CreatureSpellWireHeaderLikeCpp {
        opcode,
        caster,
        caster_unit,
        cast_id,
        original_cast_id,
        spell_id,
        spell_x_spell_visual_id,
        cast_flags,
        cast_flags_ex,
        cast_time_ms,
        target_flags: target.flags,
        target_unit: target.unit,
        hit_targets,
        miss_targets,
    };
    (header, packet)
}

pub fn decode_creature_spell_wire_header_like_cpp(bytes: &[u8]) -> CreatureSpellWireHeaderLikeCpp {
    decode_creature_spell_wire_header_and_tail_like_cpp(bytes).0
}

pub fn decode_creature_spell_full_log_like_cpp(
    bytes: &[u8],
) -> wow_packet::packets::spell::SpellCastLogData {
    use wow_packet::packets::spell::{SpellCastLogData, SpellLogPowerData};

    let (_, mut packet) = decode_creature_spell_wire_header_and_tail_like_cpp(bytes);
    assert!(packet.read_bit().expect("full combat-log presence bit"));
    let health = packet.read_int64().expect("combat-log health");
    let attack_power = packet.read_int32().expect("combat-log attack power");
    let spell_power = packet.read_int32().expect("combat-log spell power");
    let armor = packet.read_int32().expect("combat-log armor");
    let power_count = packet.read_bits(9).expect("combat-log power count") as usize;
    let power_data = (0..power_count)
        .map(|_| SpellLogPowerData {
            power_type: packet.read_int32().expect("combat-log power type"),
            amount: packet.read_int32().expect("combat-log power amount"),
            cost: packet.read_int32().expect("combat-log power cost"),
        })
        .collect();
    assert!(packet.is_empty());
    SpellCastLogData {
        health,
        attack_power,
        spell_power,
        armor,
        power_data,
    }
}
