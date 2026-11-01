//! Wire-level checks for the selected AuraUpdate scalar projection.

use super::*;
use std::time::Instant;
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_packet::{ServerPacket, WorldPacket, packets::misc::AuraUpdate};

const SPELL_ID: i32 = 8_326;

#[derive(Debug, PartialEq)]
struct DecodedAura {
    slot: u8,
    cast_id: ObjectGuid,
    spell_id: i32,
    spell_visual_id: i32,
    flags: u16,
    active_flags: u32,
    cast_level: u16,
    applications: u8,
    cast_unit: Option<ObjectGuid>,
    duration_ms: Option<i32>,
    remaining_ms: Option<i32>,
    points: Vec<f32>,
    estimated_points_count: u32,
    target_guid: ObjectGuid,
}

fn application() -> AuraApplication {
    AuraApplication {
        spell_id: SPELL_ID,
        difficulty_id: 0,
        caster_guid: ObjectGuid::create_player(1, 700),
        slot: 5,
        duration_total: 60_000,
        duration_remaining: 12_345,
        stack_count: 1,
        aura_flags: 0x0004,
        effect_mask: 0b001,
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

fn provenance() -> AuraCastProvenance {
    AuraCastProvenance {
        cast_id: ObjectGuid::create_world_object(HighGuid::Cast, 3, 1, 571, 0, SPELL_ID as u32, 55),
        spell_visual_id: 0x1122_3344,
        caster_level: None,
    }
}

fn snapshot(mask: u32) -> AuraThreatSnapshot {
    AuraThreatSnapshot::new(
        [0; 2],
        (0..3)
            .filter_map(|index| {
                let bit = 1_u32 << index;
                (mask & bit != 0).then_some((bit, 100 + index as i32, 0, 0))
            })
            .collect(),
    )
}

fn facts() -> SelectedAuraWireFacts {
    SelectedAuraWireFacts {
        captured_caster_level: 80,
        cast_item_level: 0,
        catalog_stack_amount: 0,
        canonical_charges: 0,
        attributes5: 0,
        attributes11: 0,
    }
}

fn target_guid() -> ObjectGuid {
    ObjectGuid::create_player(1, 701)
}

fn decode(bytes: &[u8]) -> DecodedAura {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AuraUpdate as u16
    );
    assert!(!packet.read_bit().unwrap());
    assert_eq!(packet.read_bits(9).unwrap(), 1);
    let slot = packet.read_uint8().unwrap();
    assert!(packet.read_bit().unwrap());

    let cast_id = packet.read_packed_guid().unwrap();
    let spell_id = packet.read_int32().unwrap();
    let spell_visual_id = packet.read_int32().unwrap();
    let flags = packet.read_uint16().unwrap();
    let active_flags = packet.read_uint32().unwrap();
    let cast_level = packet.read_uint16().unwrap();
    let applications = packet.read_uint8().unwrap();
    assert_eq!(packet.read_int32().unwrap(), 0);

    let has_cast_unit = flags & 0x0001 == 0;
    assert_eq!(packet.read_bit().unwrap(), has_cast_unit);
    let has_duration = packet.read_bit().unwrap();
    let has_remaining = packet.read_bit().unwrap();
    assert!(!packet.read_bit().unwrap());
    let point_count = packet.read_bits(6).unwrap() as usize;
    let estimated_points_count = packet.read_bits(6).unwrap();
    assert_eq!(estimated_points_count, 0);
    assert!(!packet.read_bit().unwrap());

    packet.reset_bits();
    let cast_unit = has_cast_unit.then(|| packet.read_packed_guid().unwrap());
    let duration_ms = has_duration.then(|| packet.read_int32().unwrap());
    let remaining_ms = has_remaining.then(|| packet.read_int32().unwrap());
    assert_eq!(has_duration, has_remaining);
    let points = (0..point_count)
        .map(|_| packet.read_float().unwrap())
        .collect();
    let target_guid = packet.read_packed_guid().unwrap();
    assert_eq!(packet.remaining(), 0);

    DecodedAura {
        slot,
        cast_id,
        spell_id,
        spell_visual_id,
        flags,
        active_flags,
        cast_level,
        applications,
        cast_unit,
        duration_ms,
        remaining_ms,
        points,
        estimated_points_count,
        target_guid,
    }
}

fn serialize(
    aura: &AuraApplication,
    provenance: &AuraCastProvenance,
    snapshot: &AuraThreatSnapshot,
    facts: SelectedAuraWireFacts,
) -> DecodedAura {
    let info = selected_aura_info(aura, provenance, snapshot, facts).unwrap();
    let packet = AuraUpdate {
        unit_guid: target_guid(),
        update_all: false,
        auras: vec![info],
    };
    let decoded = decode(&packet.to_bytes());
    assert_eq!(decoded.slot, aura.slot);
    assert_eq!(decoded.cast_id, provenance.cast_id);
    assert_eq!(decoded.spell_id, aura.spell_id);
    assert_eq!(decoded.spell_visual_id, provenance.spell_visual_id);
    assert_eq!(decoded.active_flags, aura.effect_mask);
    assert_eq!(decoded.target_guid, target_guid());
    decoded
}

fn assert_rejected_unchanged(
    aura: &AuraApplication,
    provenance: &AuraCastProvenance,
    snapshot: &AuraThreatSnapshot,
    facts: SelectedAuraWireFacts,
) {
    let aura_before = aura.clone();
    let provenance_before = *provenance;
    let snapshot_before = snapshot.clone();
    assert!(selected_aura_info(aura, provenance, snapshot, facts).is_err());
    assert_eq!(aura, &aura_before);
    assert_eq!(*provenance, provenance_before);
    assert_eq!(snapshot, &snapshot_before);
}

#[test]
fn selected_wire_serialization_uses_stack_amount_before_charges() {
    for (stack_count, catalog_stack_amount, charges, expected) in [
        (1, 0, 0, 0),
        (1, 0, 3, 3),
        (3, 0, 0, 3),
        (1, 5, 3, 1),
        (3, 5, 0, 3),
    ] {
        let mut aura = application();
        aura.stack_count = stack_count;
        let mut wire_facts = facts();
        wire_facts.catalog_stack_amount = catalog_stack_amount;
        wire_facts.canonical_charges = charges;
        let decoded = serialize(&aura, &provenance(), &snapshot(1), wire_facts);
        assert_eq!(decoded.applications, expected);
        assert_eq!(decoded.slot, aura.slot);
    }
}

#[test]
fn selected_wire_serialization_uses_captured_level_or_exact_item_level_cast() {
    for (attributes11, item_level, expected) in [
        (0, -1, 80),
        (0x0000_0004, -1, 65_535),
        (0x0000_0004, 420, 420),
    ] {
        let mut wire_facts = facts();
        wire_facts.attributes11 = attributes11;
        wire_facts.cast_item_level = item_level;
        let decoded = serialize(&application(), &provenance(), &snapshot(1), wire_facts);
        assert_eq!(decoded.cast_level, expected);
    }
}

#[test]
fn selected_wire_duration_flag_controls_exact_signed_duration_bytes() {
    for (maximum, remaining, attributes5, initial_flags, expected) in [
        (0, 0, 0, 0x0204, (false, None, None)),
        (-1, -2, 0, 0x0204, (false, None, None)),
        (90, 0, 0, 0, (true, Some(90), Some(0))),
        (90, -2, 0, 0x0204, (true, Some(90), Some(-2))),
        (90, -2, 0x0000_0400, 0x0204, (false, None, None)),
    ] {
        let mut aura = application();
        aura.duration_total = maximum;
        aura.duration_remaining = remaining;
        aura.aura_flags = initial_flags;
        let mut wire_facts = facts();
        wire_facts.attributes5 = attributes5;
        let decoded = serialize(&aura, &provenance(), &snapshot(1), wire_facts);
        assert_eq!(decoded.flags & 0x0004 != 0, expected.0);
        assert_eq!(decoded.flags & !0x0004, initial_flags as u16 & !0x0004);
        assert_eq!(decoded.duration_ms, expected.1);
        assert_eq!(decoded.remaining_ms, expected.2);
    }
}

#[test]
fn selected_wire_preserves_unit_caster_and_empties_nonunit_cast_unit() {
    let mut aura = application();
    aura.caster_guid = ObjectGuid::create_creature_like_cpp(1, 571, 123, 456);
    let unit_caster = aura.caster_guid;
    let decoded = serialize(&aura, &provenance(), &snapshot(1), facts());
    assert_eq!(decoded.cast_unit, Some(unit_caster));
    assert_eq!(aura.caster_guid, unit_caster);

    aura.caster_guid = ObjectGuid::create_player(1, 702);
    aura.aura_flags = 0x0201;
    let decoded = serialize(&aura, &provenance(), &snapshot(1), facts());
    assert_eq!(decoded.flags & !0x0004, 0x0201);
    assert_eq!(decoded.cast_unit, None);

    aura.caster_guid = ObjectGuid::create_gameobject_like_cpp(571, 77, 88);
    aura.aura_flags = 0;
    let nonunit_caster = aura.caster_guid;
    let decoded = serialize(&aura, &provenance(), &snapshot(1), facts());
    assert_eq!(decoded.cast_unit, Some(ObjectGuid::EMPTY));
    assert_eq!(aura.caster_guid, nonunit_caster);
}

#[test]
fn selected_wire_points_keep_active_effect_indexes_and_omit_estimates() {
    let mut aura = application();
    aura.effect_mask = 0b101;
    aura.aura_flags = 0x0008;
    let sparse = AuraThreatSnapshot::new(
        [0; 2],
        vec![
            (
                0b100,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                4,
                0,
            ),
            (0b001, wow_data::spell::aura_types::SPELL_AURA_GHOST, 12, 0),
        ],
    );
    let decoded = serialize(&aura, &provenance(), &sparse, facts());
    assert_eq!(decoded.active_flags, 0b101);
    assert_eq!(decoded.points, [12.0, 0.0, 4.0]);
    assert_eq!(decoded.estimated_points_count, 0);

    aura.aura_flags = 0;
    let decoded = serialize(&aura, &provenance(), &sparse, facts());
    assert!(decoded.points.is_empty());
}

#[test]
fn selected_wire_rejects_invalid_identity_flags_masks_and_snapshot_coverage() {
    let mut aura = application();
    let mut cast = provenance();
    let mut active = snapshot(1);

    cast.cast_id = ObjectGuid::EMPTY;
    assert_rejected_unchanged(&aura, &cast, &active, facts());

    cast = provenance();
    for invalid_mask in [0, 0b1000] {
        aura.effect_mask = invalid_mask;
        assert_rejected_unchanged(&aura, &cast, &active, facts());
    }
    aura = application();

    aura.aura_flags = u32::from(u16::MAX) + 1;
    assert_rejected_unchanged(&aura, &cast, &active, facts());
    aura = application();

    aura.effect_mask = 0b101;
    for invalid_snapshot in [
        AuraThreatSnapshot::new([0; 2], vec![(1, 10, 0, 0), (1, 11, 0, 0), (4, 12, 0, 0)]),
        AuraThreatSnapshot::new([0; 2], vec![(1, 10, 0, 0), (2, 11, 0, 0), (4, 12, 0, 0)]),
        AuraThreatSnapshot::new([0; 2], vec![(1, 10, 0, 0)]),
        AuraThreatSnapshot::new([0; 2], vec![(3, 10, 0, 0), (4, 12, 0, 0)]),
    ] {
        assert_rejected_unchanged(&aura, &cast, &invalid_snapshot, facts());
    }

    aura.caster_guid = ObjectGuid::create_gameobject_like_cpp(571, 77, 88);
    aura.aura_flags = 0x0001;
    active = snapshot(0b101);
    assert_rejected_unchanged(&aura, &cast, &active, facts());
}
