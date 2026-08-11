//! Synthetic wire-contract tests for the future creature-spell-casting flow.
//!
//! These packets are deliberately built independently of `wow-packet`; they
//! exercise the C++ `SpellCastData` layout without pretending to be captured
//! acceptance artifacts.

use capture_diff::diff::DiffReport;
use capture_diff::flow::RequirementSemanticContract;
use capture_diff::model::{Capture, CapturedPacket, Direction};
use capture_diff::semantic::{
    CorrelatedSpellGuidBody, ExactObjectGuid, SMSG_SPELL_GO, SMSG_SPELL_START,
    decode_spell_go_body, decode_spell_start_body, validate_creature_spell_casting_capture,
};

const HIGH_GUID_CREATURE: u8 = 8;
const HIGH_GUID_PLAYER: u8 = 2;
const HIGH_GUID_CAST: u8 = 47;
const CAST_SOURCE_NORMAL: u8 = 3;

fn world_guid(
    high_type: u8,
    subtype: u8,
    realm: u16,
    map: u16,
    server: u32,
    entry: u32,
    counter: u64,
) -> ExactObjectGuid {
    ExactObjectGuid {
        low: (u64::from(server & 0xFF_FFFF) << 40) | (counter & 0xFF_FFFF_FFFF),
        high: (u64::from(high_type & 0x3F) << 58)
            | (u64::from(realm & 0x1FFF) << 42)
            | (u64::from(map & 0x1FFF) << 29)
            | (u64::from(entry & 0x7F_FFFF) << 6)
            | u64::from(subtype & 0x3F),
    }
}

fn empty_guid() -> ExactObjectGuid {
    ExactObjectGuid { low: 0, high: 0 }
}

fn player_guid(counter: u64) -> ExactObjectGuid {
    world_guid(HIGH_GUID_PLAYER, 0, 1, 0, 0, 0, counter)
}

fn push_packed_guid(out: &mut Vec<u8>, guid: ExactObjectGuid) {
    let low = guid.low.to_le_bytes();
    let high = guid.high.to_le_bytes();
    let low_mask = low.iter().enumerate().fold(0u8, |mask, (index, byte)| {
        mask | (u8::from(*byte != 0) << index)
    });
    let high_mask = high.iter().enumerate().fold(0u8, |mask, (index, byte)| {
        mask | (u8::from(*byte != 0) << index)
    });
    out.push(low_mask);
    out.push(high_mask);
    out.extend(low.into_iter().filter(|byte| *byte != 0));
    out.extend(high.into_iter().filter(|byte| *byte != 0));
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    used: u8,
}

impl BitWriter {
    fn bits(&mut self, value: u32, width: u8) {
        for shift in (0..width).rev() {
            self.current |= (((value >> shift) & 1) as u8) << (7 - self.used);
            self.used += 1;
            if self.used == 8 {
                self.bytes.push(self.current);
                self.current = 0;
                self.used = 0;
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.used != 0 {
            self.bytes.push(self.current);
        }
        self.bytes
    }
}

fn push_location(out: &mut Vec<u8>, transport: ExactObjectGuid, xyz: [f32; 3]) {
    push_packed_guid(out, transport);
    for coordinate in xyz {
        out.extend(coordinate.to_bits().to_le_bytes());
    }
}

#[derive(Clone)]
struct WireConfig {
    caster: ExactObjectGuid,
    caster_unit: ExactObjectGuid,
    cast_id: ExactObjectGuid,
    original_cast_id: ExactObjectGuid,
    spell_id: i32,
    spell_visual_id: i32,
    cast_flags: u32,
    cast_flags_ex: u32,
    cast_time: u32,
    missile_travel_time: u32,
    missile_pitch_bits: u32,
    dest_loc_spell_cast_index: u8,
    immunities_school: u32,
    immunities_value: u32,
    prediction_points: u32,
    prediction_type: u8,
    prediction_beacon: ExactObjectGuid,
    target_flags: u32,
    target_unit: ExactObjectGuid,
    target_src_location: Option<(ExactObjectGuid, [f32; 3])>,
    target_dst_location: Option<(ExactObjectGuid, [f32; 3])>,
    target_orientation_bits: Option<u32>,
    target_map_id: Option<i32>,
    target_name: Vec<u8>,
    hit_targets: Vec<ExactObjectGuid>,
    misses: Vec<(ExactObjectGuid, u8, Option<u8>)>,
    miss_status_count_override: Option<u16>,
    remaining_power: Vec<(i32, i8)>,
    rune_cooldowns: Option<Vec<u8>>,
    target_points: Vec<(ExactObjectGuid, [f32; 3])>,
    ammo_display_id: Option<i32>,
    ammo_inventory_type: Option<i32>,
    full_combat_log: bool,
}

impl WireConfig {
    fn creature(caster_counter: u64, cast_counter: u64, cast_time: u32) -> Self {
        let spell_id = 133;
        let caster = world_guid(HIGH_GUID_CREATURE, 0, 1, 530, 0, 15_274, caster_counter);
        Self {
            caster,
            caster_unit: caster,
            cast_id: world_guid(
                HIGH_GUID_CAST,
                CAST_SOURCE_NORMAL,
                1,
                530,
                0,
                spell_id as u32,
                cast_counter,
            ),
            original_cast_id: empty_guid(),
            spell_id,
            spell_visual_id: 777,
            cast_flags: 0x0004_0101,
            cast_flags_ex: 0x0000_8000,
            cast_time,
            missile_travel_time: 250,
            missile_pitch_bits: 1.25f32.to_bits(),
            dest_loc_spell_cast_index: 3,
            immunities_school: 0x10,
            immunities_value: 0x20,
            prediction_points: 42,
            prediction_type: 2,
            prediction_beacon: player_guid(88),
            target_flags: 0x80,
            target_unit: caster,
            target_src_location: Some((empty_guid(), [1.0, 2.0, 3.0])),
            target_dst_location: Some((player_guid(44), [4.0, 5.0, 6.0])),
            target_orientation_bits: Some(0.5f32.to_bits()),
            target_map_id: Some(530),
            target_name: b"self".to_vec(),
            hit_targets: vec![caster],
            misses: vec![(player_guid(99), 11, Some(4))],
            miss_status_count_override: None,
            remaining_power: vec![(-20, 0)],
            rune_cooldowns: Some(vec![1, 2, 3]),
            target_points: vec![(empty_guid(), [7.0, 8.0, 9.0])],
            ammo_display_id: Some(12_345),
            ammo_inventory_type: Some(2),
            full_combat_log: false,
        }
    }
}

fn issue_26_fixture_configs(
    caster_counter: u64,
    cast_counter: u64,
    go_cast_time: u32,
) -> (WireConfig, WireConfig) {
    let victim = player_guid(15);
    let caster = world_guid(HIGH_GUID_CREATURE, 0, 1, 530, 0, 22_378, caster_counter);
    let mut go = WireConfig::creature(caster_counter, cast_counter, go_cast_time);
    go.caster = caster;
    go.caster_unit = caster;
    go.cast_id = world_guid(
        HIGH_GUID_CAST,
        CAST_SOURCE_NORMAL,
        1,
        530,
        0,
        15_691,
        cast_counter,
    );
    go.spell_id = 15_691;
    go.spell_visual_id = 244_493;
    go.cast_flags = 0x0000_0100;
    go.cast_flags_ex = 0;
    go.missile_travel_time = 0;
    go.missile_pitch_bits = 0.0f32.to_bits();
    go.dest_loc_spell_cast_index = 0;
    go.immunities_school = 0;
    go.immunities_value = 0;
    go.prediction_points = 0;
    go.prediction_type = 0;
    go.prediction_beacon = empty_guid();
    go.target_flags = 0x2;
    go.target_unit = victim;
    go.target_src_location = None;
    go.target_dst_location = None;
    go.target_orientation_bits = None;
    go.target_map_id = None;
    go.target_name.clear();
    go.hit_targets = vec![victim];
    go.misses.clear();
    go.miss_status_count_override = None;
    go.remaining_power.clear();
    go.rune_cooldowns = None;
    go.target_points.clear();
    go.ammo_display_id = None;
    go.ammo_inventory_type = None;
    go.full_combat_log = false;

    let mut start = go.clone();
    start.cast_flags = 0x0000_0002;
    start.cast_time = 0;
    start.hit_targets.clear();
    (start, go)
}

struct EncodedBody {
    bytes: Vec<u8>,
    counts_offset: usize,
    target_bits_offset: usize,
    combat_log_offset: usize,
}

fn encode_packet(config: &WireConfig, spell_go: bool) -> EncodedBody {
    let mut out = Vec::new();
    for guid in [
        config.caster,
        config.caster_unit,
        config.cast_id,
        config.original_cast_id,
    ] {
        push_packed_guid(&mut out, guid);
    }
    out.extend(config.spell_id.to_le_bytes());
    out.extend(config.spell_visual_id.to_le_bytes());
    out.extend(config.cast_flags.to_le_bytes());
    out.extend(config.cast_flags_ex.to_le_bytes());
    out.extend(config.cast_time.to_le_bytes());
    out.extend(config.missile_travel_time.to_le_bytes());
    out.extend(config.missile_pitch_bits.to_le_bytes());
    out.push(config.dest_loc_spell_cast_index);
    out.extend(config.immunities_school.to_le_bytes());
    out.extend(config.immunities_value.to_le_bytes());
    out.extend(config.prediction_points.to_le_bytes());
    out.push(config.prediction_type);
    push_packed_guid(&mut out, config.prediction_beacon);

    let counts_offset = out.len();
    let mut counts = BitWriter::default();
    let hit_count = spell_go.then_some(config.hit_targets.len()).unwrap_or(0);
    let miss_count = spell_go.then_some(config.misses.len()).unwrap_or(0);
    counts.bits(hit_count as u32, 16);
    counts.bits(miss_count as u32, 16);
    counts.bits(
        u32::from(if spell_go {
            config
                .miss_status_count_override
                .unwrap_or(config.misses.len() as u16)
        } else {
            0
        }),
        16,
    );
    counts.bits(config.remaining_power.len() as u32, 9);
    counts.bits(u32::from(config.rune_cooldowns.is_some()), 1);
    counts.bits(config.target_points.len() as u32, 16);
    counts.bits(u32::from(config.ammo_display_id.is_some()), 1);
    counts.bits(u32::from(config.ammo_inventory_type.is_some()), 1);
    out.extend(counts.finish());

    let target_bits_offset = out.len();
    let mut target_bits = BitWriter::default();
    target_bits.bits(config.target_flags, 28);
    target_bits.bits(u32::from(config.target_src_location.is_some()), 1);
    target_bits.bits(u32::from(config.target_dst_location.is_some()), 1);
    target_bits.bits(u32::from(config.target_orientation_bits.is_some()), 1);
    target_bits.bits(u32::from(config.target_map_id.is_some()), 1);
    target_bits.bits(config.target_name.len() as u32, 7);
    out.extend(target_bits.finish());
    push_packed_guid(&mut out, config.target_unit);
    push_packed_guid(&mut out, empty_guid());
    if let Some((transport, position)) = config.target_src_location {
        push_location(&mut out, transport, position);
    }
    if let Some((transport, position)) = config.target_dst_location {
        push_location(&mut out, transport, position);
    }
    if let Some(orientation_bits) = config.target_orientation_bits {
        out.extend(orientation_bits.to_le_bytes());
    }
    if let Some(map_id) = config.target_map_id {
        out.extend(map_id.to_le_bytes());
    }
    out.extend(&config.target_name);

    if spell_go {
        for guid in &config.hit_targets {
            push_packed_guid(&mut out, *guid);
        }
        for (guid, _, _) in &config.misses {
            push_packed_guid(&mut out, *guid);
        }
        for (_, reason, reflect_status) in &config.misses {
            out.push(*reason);
            if *reason == 11 {
                out.push(reflect_status.expect("reflect test status"));
            }
        }
    }
    for (cost, power_type) in &config.remaining_power {
        out.extend(cost.to_le_bytes());
        out.push(*power_type as u8);
    }
    if let Some(cooldowns) = &config.rune_cooldowns {
        out.push(1);
        out.push(6);
        out.extend((cooldowns.len() as u32).to_le_bytes());
        out.extend(cooldowns);
    }
    for (transport, position) in &config.target_points {
        push_location(&mut out, *transport, *position);
    }
    if let Some(ammo_display_id) = config.ammo_display_id {
        out.extend(ammo_display_id.to_le_bytes());
    }
    if let Some(ammo_inventory_type) = config.ammo_inventory_type {
        out.extend(ammo_inventory_type.to_le_bytes());
    }

    let combat_log_offset = out.len();
    if spell_go {
        out.push(if config.full_combat_log { 0x80 } else { 0 });
    }
    EncodedBody {
        bytes: out,
        counts_offset,
        target_bits_offset,
        combat_log_offset,
    }
}

fn encode(config: &WireConfig) -> EncodedBody {
    encode_packet(config, true)
}

fn encode_start(config: &WireConfig) -> EncodedBody {
    encode_packet(config, false)
}

fn packet_with_opcode(
    direction: Direction,
    connection_id: u32,
    opcode: u16,
    body: Vec<u8>,
) -> CapturedPacket {
    CapturedPacket {
        direction,
        connection_id,
        opcode,
        body,
    }
}

fn packet(direction: Direction, connection_id: u32, body: Vec<u8>) -> CapturedPacket {
    packet_with_opcode(direction, connection_id, SMSG_SPELL_GO, body)
}

fn start_packet(direction: Direction, connection_id: u32, body: Vec<u8>) -> CapturedPacket {
    packet_with_opcode(direction, connection_id, SMSG_SPELL_START, body)
}

fn issue_26_capture(source: &str, start: &WireConfig, go: &WireConfig) -> Capture {
    Capture::new(
        source,
        vec![
            start_packet(Direction::S2C, 1, encode_start(start).bytes),
            packet(Direction::S2C, 1, encode(go).bytes),
        ],
    )
}

fn assert_issue_26_contract_rejects(source: &str, start: &WireConfig, go: &WireConfig) {
    let capture = issue_26_capture(source, start, go);
    assert!(
        validate_creature_spell_casting_capture(&capture).is_err(),
        "issue-#26 contract accepted {source}"
    );
}

fn report(cpp: Vec<u8>, rust: Vec<u8>) -> DiffReport {
    report_opcode(SMSG_SPELL_GO, cpp, rust)
}

fn report_opcode(opcode: u16, cpp: Vec<u8>, rust: Vec<u8>) -> DiffReport {
    DiffReport::compute(
        &Capture::new(
            "cpp",
            vec![packet_with_opcode(Direction::S2C, 1, opcode, cpp)],
        ),
        &Capture::new(
            "rust",
            vec![packet_with_opcode(Direction::S2C, 1, opcode, rust)],
        ),
        &[Direction::S2C],
    )
}

#[test]
fn full_spell_cast_data_decodes_without_ignoring_optional_fields() {
    let config = WireConfig::creature(0x102, 0x203, 0x1234_5678);
    let encoded = encode(&config);
    let decoded = decode_spell_go_body(&encoded.bytes).expect("full SpellGo decode");

    assert_eq!(decoded.exact_caster_guid, config.caster);
    assert_eq!(decoded.exact_caster_unit, config.caster);
    assert_eq!(decoded.cast_id, config.cast_id);
    assert_eq!(decoded.cast_time, 0x1234_5678);
    assert_eq!(decoded.body.original_cast_id, empty_guid());
    assert_eq!(decoded.body.spell_id, 133);
    assert_eq!(decoded.body.spell_visual_id, 777);
    assert_eq!(decoded.body.cast_flags, 0x0004_0101);
    assert_eq!(decoded.body.cast_flags_ex, 0x0000_8000);
    assert_eq!(decoded.body.missile_travel_time, 250);
    assert_eq!(decoded.body.missile_pitch_bits, 1.25f32.to_bits());
    assert_eq!(decoded.body.target.unit, CorrelatedSpellGuidBody::Caster);
    assert_eq!(
        decoded.body.hit_targets,
        vec![CorrelatedSpellGuidBody::Caster]
    );
    assert_eq!(decoded.body.miss_targets.len(), 1);
    assert_eq!(decoded.body.miss_status[0].reflect_status, Some(4));
    assert_eq!(decoded.body.remaining_power[0].cost, -20);
    assert_eq!(
        decoded.body.remaining_runes.as_ref().unwrap().cooldowns,
        [1, 2, 3]
    );
    assert_eq!(decoded.body.target_points.len(), 1);
    assert_eq!(decoded.body.ammo_display_id, Some(12_345));
    assert_eq!(decoded.body.ammo_inventory_type, Some(2));

    let start = decode_spell_start_body(&encode_start(&config).bytes).expect("full SpellStart");
    let mut expected_start_cast = decoded.body;
    expected_start_cast.hit_targets.clear();
    expected_start_cast.miss_targets.clear();
    expected_start_cast.miss_status.clear();
    assert_eq!(start.body.cast, expected_start_cast);
    assert_eq!(start.body.cast_time, 0x1234_5678);
    assert_eq!(start.cast_id, config.cast_id);
}

#[test]
fn only_reviewed_runtime_counters_and_cast_time_compare_clean() {
    let cpp = WireConfig::creature(0x101, 0x202, 100);
    let rust = WireConfig::creature(0xA0A, 0xB0B, 900);
    let clean = report(encode(&cpp).bytes, encode(&rust).bytes);
    assert!(clean.is_clean(), "{}", clean.render_text());
    assert_eq!(
        clean.ops[0]
            .body
            .as_ref()
            .unwrap()
            .semantic
            .as_ref()
            .unwrap()
            .comparator,
        "smsg_spell_go_creature_runtime_counters_and_cast_time"
    );

    let mut stable_drifts: Vec<(&str, WireConfig)> = Vec::new();

    let mut changed = rust.clone();
    changed.spell_visual_id += 1;
    stable_drifts.push(("spell_visual_id", changed));

    let mut changed = rust.clone();
    changed.cast_flags ^= 0x10;
    stable_drifts.push(("cast_flags", changed));

    let mut changed = rust.clone();
    changed.cast_flags_ex ^= 0x20;
    stable_drifts.push(("cast_flags_ex", changed));

    let mut changed = rust.clone();
    changed.missile_travel_time += 1;
    stable_drifts.push(("missile_trajectory", changed));

    let mut changed = rust.clone();
    changed.target_name = b"other".to_vec();
    stable_drifts.push(("target", changed));

    let mut changed = rust.clone();
    changed.hit_targets.push(player_guid(77));
    stable_drifts.push(("hit_targets", changed));

    let mut changed = rust.clone();
    changed.original_cast_id = player_guid(5);
    stable_drifts.push(("OriginalCastID", changed));

    let mut changed = rust.clone();
    changed.caster = world_guid(HIGH_GUID_CREATURE, 0, 1, 530, 0, 15_275, 0xA0A);
    changed.caster_unit = changed.caster;
    changed.target_unit = changed.caster;
    changed.hit_targets[0] = changed.caster;
    stable_drifts.push(("caster_guid", changed));

    for (label, changed) in stable_drifts {
        let dirty = report(encode(&cpp).bytes, encode(&changed).bytes);
        assert!(!dirty.is_clean(), "normalized stable drift {label}");
    }
}

#[test]
fn spell_start_normalizes_counters_but_keeps_cast_duration_exact() {
    let cpp = WireConfig::creature(0x101, 0x202, 1_500);
    let rust = WireConfig::creature(0xA0A, 0xB0B, 1_500);
    let clean = report_opcode(
        SMSG_SPELL_START,
        encode_start(&cpp).bytes,
        encode_start(&rust).bytes,
    );
    assert!(clean.is_clean(), "{}", clean.render_text());
    assert_eq!(
        clean.ops[0]
            .body
            .as_ref()
            .unwrap()
            .semantic
            .as_ref()
            .unwrap()
            .comparator,
        "smsg_spell_start_creature_runtime_counters"
    );

    let mut wrong_duration = rust.clone();
    wrong_duration.cast_time += 1;
    let dirty = report_opcode(
        SMSG_SPELL_START,
        encode_start(&cpp).bytes,
        encode_start(&wrong_duration).bytes,
    );
    assert!(!dirty.is_clean(), "SpellStart CastTime was normalized");
    assert!(dirty.render_text().contains("cast_time"));

    let mut wrong_original = rust;
    wrong_original.original_cast_id = player_guid(7);
    let dirty = report_opcode(
        SMSG_SPELL_START,
        encode_start(&cpp).bytes,
        encode_start(&wrong_original).bytes,
    );
    assert!(
        !dirty.is_clean(),
        "SpellStart OriginalCastID was normalized"
    );

    let malformed = encode_start(&wrong_original).bytes;
    let equally_malformed = report_opcode(SMSG_SPELL_START, malformed.clone(), malformed);
    assert!(
        !equally_malformed.is_clean(),
        "equal invalid SpellStart bodies must fail closed"
    );
}

#[test]
fn player_spell_go_does_not_gain_creature_runtime_normalization() {
    let mut cpp = WireConfig::creature(10, 20, 30);
    cpp.caster = player_guid(10);
    cpp.caster_unit = cpp.caster;
    cpp.target_unit = cpp.caster;
    cpp.hit_targets = vec![cpp.caster];
    let mut rust = cpp.clone();
    rust.cast_time = 31;
    rust.cast_id = world_guid(HIGH_GUID_CAST, CAST_SOURCE_NORMAL, 1, 530, 0, 133, 99);

    let dirty = report(encode(&cpp).bytes, encode(&rust).bytes);
    assert!(
        !dirty.is_clean(),
        "player SpellGo was normalized as Creature"
    );

    let dirty_start = report_opcode(
        SMSG_SPELL_START,
        encode_start(&cpp).bytes,
        encode_start(&rust).bytes,
    );
    assert!(
        !dirty_start.is_clean(),
        "player SpellStart was normalized as Creature"
    );
}

#[test]
fn malformed_layout_padding_counts_and_trailing_bytes_fail_closed() {
    let valid = encode(&WireConfig::creature(0x102, 0x203, 300));
    decode_spell_start_body(&valid.bytes[..valid.combat_log_offset])
        .expect("SpellStart is SpellCastData without the GO log bit");
    assert!(
        decode_spell_start_body(&valid.bytes).is_err(),
        "SpellStart accepted a GO combat-log suffix as trailing bytes"
    );

    for end in 0..valid.bytes.len() {
        assert!(
            decode_spell_go_body(&valid.bytes[..end]).is_err(),
            "accepted truncation at {end}"
        );
    }

    let mut trailing = valid.bytes.clone();
    trailing.push(0xAA);
    assert!(decode_spell_go_body(&trailing).is_err());

    let mut noncanonical_guid = valid.bytes.clone();
    noncanonical_guid[2] = 0;
    assert!(
        decode_spell_go_body(&noncanonical_guid)
            .unwrap_err()
            .contains("non-canonical packed encoding")
    );

    let mut count_padding = valid.bytes.clone();
    count_padding[valid.counts_offset + 9] |= 0x01;
    assert!(
        decode_spell_go_body(&count_padding)
            .unwrap_err()
            .contains("padding")
    );

    let mut target_padding = valid.bytes.clone();
    target_padding[valid.target_bits_offset + 4] |= 0x01;
    assert!(
        decode_spell_go_body(&target_padding)
            .unwrap_err()
            .contains("padding")
    );

    let mut full_log = WireConfig::creature(1, 2, 3);
    full_log.full_combat_log = true;
    let encoded_full_log = encode(&full_log);
    assert_eq!(
        encoded_full_log.bytes[encoded_full_log.combat_log_offset],
        0x80
    );
    assert!(
        decode_spell_go_body(&encoded_full_log.bytes)
            .unwrap_err()
            .contains("full SpellCastLogData")
    );

    let mut bad_topology = WireConfig::creature(1, 2, 3);
    bad_topology.miss_status_count_override = Some(0);
    assert!(
        decode_spell_go_body(&encode(&bad_topology).bytes)
            .unwrap_err()
            .contains("MissTargets")
    );

    let mut nan_pitch = WireConfig::creature(1, 2, 3);
    nan_pitch.missile_pitch_bits = f32::NAN.to_bits();
    assert!(
        decode_spell_go_body(&encode(&nan_pitch).bytes)
            .unwrap_err()
            .contains("not finite")
    );

    let equally_malformed = report(trailing.clone(), trailing.clone());
    assert!(
        !equally_malformed.is_clean(),
        "equal malformed SpellGo bodies must fail closed"
    );

    let malformed_report = report(valid.bytes.clone(), trailing);
    assert!(!malformed_report.is_clean());
    assert!(
        malformed_report.ops[0]
            .body
            .as_ref()
            .and_then(|body| body.semantic.as_ref())
            .is_some_and(|semantic| semantic.rust.decode_error.is_some())
    );
}

#[test]
fn required_contract_variant_and_creature_identity_checks_are_ready_for_future_flow() {
    let contract: RequirementSemanticContract =
        serde_json::from_str("\"creature-spell-casting-v1\"").unwrap();
    assert_eq!(
        contract,
        RequirementSemanticContract::CreatureSpellCastingV1
    );

    let (start_config, go_config) = issue_26_fixture_configs(1, 2, 3);
    let valid_start = encode_start(&start_config).bytes;
    let valid_go = encode(&go_config).bytes;
    let valid = Capture::new(
        "synthetic contract pair",
        vec![
            start_packet(Direction::S2C, 1, valid_start.clone()),
            packet(Direction::S2C, 1, valid_go.clone()),
        ],
    );
    validate_creature_spell_casting_capture(&valid).unwrap();

    let go_only = Capture::new("GO only", vec![packet(Direction::S2C, 1, valid_go.clone())]);
    assert!(
        validate_creature_spell_casting_capture(&go_only)
            .unwrap_err()
            .contains("SMSG_SPELL_START")
    );

    let reversed = Capture::new(
        "reversed",
        vec![
            packet(Direction::S2C, 1, valid_go.clone()),
            start_packet(Direction::S2C, 1, valid_start.clone()),
        ],
    );
    assert!(
        validate_creature_spell_casting_capture(&reversed)
            .unwrap_err()
            .contains("SMSG_SPELL_START -> SMSG_SPELL_GO")
    );

    let interrupted = Capture::new(
        "interrupted",
        vec![
            start_packet(Direction::S2C, 1, valid_start.clone()),
            packet_with_opcode(Direction::S2C, 1, 0x1234, Vec::new()),
            packet(Direction::S2C, 1, valid_go.clone()),
        ],
    );
    assert!(validate_creature_spell_casting_capture(&interrupted).is_err());

    let duplicate = Capture::new(
        "duplicate",
        vec![
            start_packet(Direction::S2C, 1, valid_start.clone()),
            packet(Direction::S2C, 1, valid_go.clone()),
            packet(Direction::S2C, 1, valid_go.clone()),
        ],
    );
    assert!(validate_creature_spell_casting_capture(&duplicate).is_err());

    let (mut wrong_original_start, mut wrong_original_go) = issue_26_fixture_configs(1, 2, 3);
    wrong_original_start.original_cast_id = player_guid(7);
    wrong_original_go.original_cast_id = player_guid(7);
    let capture = issue_26_capture(
        "nonempty original",
        &wrong_original_start,
        &wrong_original_go,
    );
    assert!(
        validate_creature_spell_casting_capture(&capture)
            .unwrap_err()
            .contains("OriginalCastID")
    );

    let (mut wrong_cast_source_start, mut wrong_cast_source_go) = issue_26_fixture_configs(1, 2, 3);
    let wrong_cast_source = world_guid(HIGH_GUID_CAST, 4, 1, 530, 0, 15_691, 2);
    wrong_cast_source_start.cast_id = wrong_cast_source;
    wrong_cast_source_go.cast_id = wrong_cast_source;
    let capture = issue_26_capture(
        "wrong cast source",
        &wrong_cast_source_start,
        &wrong_cast_source_go,
    );
    assert!(
        validate_creature_spell_casting_capture(&capture)
            .unwrap_err()
            .contains("source")
    );

    let (mut wrong_cast_realm_start, mut wrong_cast_realm_go) = issue_26_fixture_configs(1, 2, 3);
    let wrong_cast_realm = world_guid(HIGH_GUID_CAST, CAST_SOURCE_NORMAL, 0, 530, 0, 15_691, 2);
    wrong_cast_realm_start.cast_id = wrong_cast_realm;
    wrong_cast_realm_go.cast_id = wrong_cast_realm;
    let capture = issue_26_capture(
        "cast realm inherited from caster",
        &wrong_cast_realm_start,
        &wrong_cast_realm_go,
    );
    assert!(
        validate_creature_spell_casting_capture(&capture)
            .unwrap_err()
            .contains("caster realm/map")
    );

    let (mut mismatched_caster_start, mut mismatched_caster_go) = issue_26_fixture_configs(1, 2, 3);
    let other_caster = world_guid(HIGH_GUID_CREATURE, 0, 1, 530, 0, 22_378, 9);
    mismatched_caster_start.caster_unit = other_caster;
    mismatched_caster_go.caster_unit = other_caster;
    let capture = issue_26_capture(
        "mismatched caster",
        &mismatched_caster_start,
        &mismatched_caster_go,
    );
    assert!(
        validate_creature_spell_casting_capture(&capture)
            .unwrap_err()
            .contains("CasterGUID")
    );

    let (start_config, _) = issue_26_fixture_configs(1, 2, 3);
    let (_, go_config) = issue_26_fixture_configs(1, 99, 3);
    let capture = issue_26_capture("uncorrelated CastID", &start_config, &go_config);
    assert!(
        validate_creature_spell_casting_capture(&capture)
            .unwrap_err()
            .contains("cast_id")
    );
}

#[test]
fn required_contract_pins_issue_26_fixture_payload_and_success_topology() {
    let (valid_start, valid_go) = issue_26_fixture_configs(10, 20, 30);
    validate_creature_spell_casting_capture(&issue_26_capture(
        "exact issue-26 fixture",
        &valid_start,
        &valid_go,
    ))
    .unwrap();

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.caster = world_guid(HIGH_GUID_CREATURE, 0, 1, 530, 0, 22_379, 10);
    start.caster_unit = start.caster;
    go.caster = start.caster;
    go.caster_unit = start.caster;
    assert_issue_26_contract_rejects("wrong creature entry", &start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.caster = world_guid(HIGH_GUID_CREATURE, 0, 0, 530, 0, 22_378, 10);
    start.caster_unit = start.caster;
    go.caster = start.caster;
    go.caster_unit = start.caster;
    start.cast_id = world_guid(HIGH_GUID_CAST, CAST_SOURCE_NORMAL, 0, 530, 0, 15_691, 20);
    go.cast_id = start.cast_id;
    assert_issue_26_contract_rejects("wrong local realm", &start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.caster = world_guid(HIGH_GUID_CREATURE, 0, 1, 1, 0, 22_378, 10);
    start.caster_unit = start.caster;
    go.caster = start.caster;
    go.caster_unit = start.caster;
    start.cast_id = world_guid(HIGH_GUID_CAST, CAST_SOURCE_NORMAL, 1, 1, 0, 15_691, 20);
    go.cast_id = start.cast_id;
    assert_issue_26_contract_rejects("wrong caster map", &start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.spell_id = 15_692;
    go.spell_id = 15_692;
    start.cast_id = world_guid(HIGH_GUID_CAST, CAST_SOURCE_NORMAL, 1, 530, 0, 15_692, 20);
    go.cast_id = start.cast_id;
    assert_issue_26_contract_rejects("wrong spell", &start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    let wrong_realm_victim = world_guid(HIGH_GUID_PLAYER, 0, 0, 0, 0, 0, 30);
    start.target_unit = wrong_realm_victim;
    go.target_unit = wrong_realm_victim;
    go.hit_targets = vec![wrong_realm_victim];
    assert_issue_26_contract_rejects("wrong player realm", &start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    let wrong_player = player_guid(16);
    start.target_unit = wrong_player;
    go.target_unit = wrong_player;
    go.hit_targets = vec![wrong_player];
    assert_issue_26_contract_rejects("wrong fixture player", &start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.spell_visual_id += 1;
    go.spell_visual_id += 1;
    assert_issue_26_contract_rejects("wrong visual", &start, &go);

    let mut start = valid_start.clone();
    start.cast_flags = 0;
    assert_issue_26_contract_rejects("wrong START flags", &start, &valid_go);

    let mut go = valid_go.clone();
    go.cast_flags = 0x0004_0100;
    assert_issue_26_contract_rejects("wrong GO flags", &valid_start, &go);

    let mut go = valid_go.clone();
    go.cast_flags_ex = 1;
    assert_issue_26_contract_rejects("nonzero CastFlagsEx", &valid_start, &go);

    let mut start = valid_start.clone();
    start.cast_time = 1;
    assert_issue_26_contract_rejects("noninstant START", &start, &valid_go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.target_flags = 0;
    go.target_flags = 0;
    assert_issue_26_contract_rejects("wrong target flags", &start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.target_unit = start.caster;
    go.target_unit = go.caster;
    go.hit_targets = vec![go.caster];
    assert_issue_26_contract_rejects("non-player target", &start, &go);

    let mut go = valid_go.clone();
    go.hit_targets.clear();
    assert_issue_26_contract_rejects("missing successful hit", &valid_start, &go);

    let mut go = valid_go.clone();
    go.misses = vec![(go.target_unit, 1, None)];
    go.hit_targets.clear();
    assert_issue_26_contract_rejects("miss instead of hit", &valid_start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.target_dst_location = Some((empty_guid(), [1.0, 2.0, 3.0]));
    go.target_dst_location = start.target_dst_location;
    assert_issue_26_contract_rejects("destination optional", &start, &go);

    let mut start = valid_start.clone();
    let mut go = valid_go.clone();
    start.remaining_power = vec![(0, 0)];
    go.remaining_power = start.remaining_power.clone();
    assert_issue_26_contract_rejects("power optional", &start, &go);
}
