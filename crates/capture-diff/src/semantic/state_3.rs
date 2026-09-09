//! Semantic capture diff state definitions, part 3 of 5.
//!
//! Separated from the semantic.rs root under #658. Behaviour is preserved.

use super::*;

pub(super) fn decode_update_object_inv_slots_candidate_inner(
    body: &[u8],
) -> Result<DecodedUpdateObjectInvSlotsBody, UpdateObjectInvSlotsFailure> {
    let mut cursor = 0usize;
    let num_updates = read_u32(body, &mut cursor, "NumObjUpdates").map_err(|_| {
        UpdateObjectInvSlotsFailure::NotEligible("body is too short for NumObjUpdates")
    })?;
    if num_updates != 1 {
        return Err(UpdateObjectInvSlotsFailure::NotEligible(
            "NumObjUpdates is not exactly one",
        ));
    }

    let map_id =
        read_u16(body, &mut cursor, "MapID").map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    let destroy_or_out_of_range = read_u8(body, &mut cursor, "HasDestroyOrOutOfRange byte")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if destroy_or_out_of_range == 0x80 {
        return Err(UpdateObjectInvSlotsFailure::NotEligible(
            "packet carries destroy or out-of-range GUIDs",
        ));
    }
    if destroy_or_out_of_range != 0 {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "HasDestroyOrOutOfRange byte has non-canonical padding bits: 0x{destroy_or_out_of_range:02X}"
        )));
    }

    let declared_blocks_len = read_u32(body, &mut cursor, "update blocks length")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)? as usize;
    let actual_blocks_len = body.len().saturating_sub(cursor);
    if declared_blocks_len != actual_blocks_len {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "update blocks length declares {declared_blocks_len} bytes but {actual_blocks_len} remain"
        )));
    }
    let blocks = &body[cursor..];
    let mut block_cursor = 0usize;
    let update_type = read_u8(blocks, &mut block_cursor, "UpdateType")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if update_type != 0 {
        return Err(UpdateObjectInvSlotsFailure::NotEligible(
            "the sole update block is not UpdateType::Values",
        ));
    }

    let (player_low, player_high) = read_packed_guid(blocks, &mut block_cursor, "Player")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if ((player_high >> 58) & 0x3F) as u8 != HIGH_GUID_PLAYER {
        return Err(UpdateObjectInvSlotsFailure::NotEligible(
            "the VALUES owner is not a Player GUID",
        ));
    }
    if player_low == 0 {
        return Err(UpdateObjectInvSlotsFailure::Malformed(
            "Player GUID has a zero low identity".to_string(),
        ));
    }
    if player_high & GLOBAL_GUID_RESERVED_HIGH_BITS_MASK != 0 {
        return Err(UpdateObjectInvSlotsFailure::Malformed(
            "Player GUID sets reserved high-word bits 0..41".to_string(),
        ));
    }

    let declared_values_len = read_u32(blocks, &mut block_cursor, "values length")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)? as usize;
    let actual_values_len = blocks.len().saturating_sub(block_cursor);
    if declared_values_len != actual_values_len {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "values length declares {declared_values_len} bytes but {actual_values_len} remain"
        )));
    }
    let values = &blocks[block_cursor..];
    let mut values_cursor = 0usize;
    let has_empty_unit_power_parent =
        decode_reviewed_changed_object_type_mask(values, &mut values_cursor)?;

    if has_empty_unit_power_parent {
        decode_empty_unit_power_parent(values, &mut values_cursor)?;
    }

    let inv_slots = decode_active_player_inv_slots(values, &mut values_cursor)
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if values_cursor != values.len() {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "trailing bytes after ActivePlayer InvSlots: decoded {values_cursor} of {} bytes",
            values.len()
        )));
    }

    Ok(DecodedUpdateObjectInvSlotsBody {
        body: UpdateObjectInvSlotsBody {
            map_id,
            player: ExactObjectGuid {
                low: player_low,
                high: player_high,
            },
            inv_slots,
        },
        has_empty_unit_power_parent,
    })
}

pub(super) fn decode_reviewed_changed_object_type_mask(
    values: &[u8],
    cursor: &mut usize,
) -> Result<bool, UpdateObjectInvSlotsFailure> {
    let changed_object_type_mask = read_u32(values, cursor, "ChangedObjectTypeMask")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    match changed_object_type_mask {
        VALUES_TYPE_ACTIVE_PLAYER => Ok(false),
        mask if mask == VALUES_TYPE_UNIT | VALUES_TYPE_ACTIVE_PLAYER => Ok(true),
        _ => Err(UpdateObjectInvSlotsFailure::NotEligible(
            "ChangedObjectTypeMask is not ActivePlayer-only or Unit+ActivePlayer",
        )),
    }
}

pub(super) fn decode_empty_unit_power_parent(
    values: &[u8],
    cursor: &mut usize,
) -> Result<(), UpdateObjectInvSlotsFailure> {
    let blocks_mask = read_u8(values, cursor, "UnitData blocks mask")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if blocks_mask != UNIT_POWER_PARENT_BLOCKS_MASK {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "UnitData blocks mask is 0x{blocks_mask:02X}, expected only block 3"
        )));
    }

    let block_3 = read_u32_be(values, cursor, "UnitData block 3")
        .map_err(UpdateObjectInvSlotsFailure::Malformed)?;
    if block_3 != UNIT_POWER_PARENT_BLOCK_3 {
        return Err(UpdateObjectInvSlotsFailure::Malformed(format!(
            "UnitData block 3 is 0x{block_3:08X}, expected only parent bit 116"
        )));
    }
    Ok(())
}

pub(super) fn decode_active_player_inv_slots(
    values: &[u8],
    cursor: &mut usize,
) -> Result<Vec<InvSlotValue>, String> {
    let blocks_group_0 = read_u32(values, cursor, "ActivePlayer blocks group 0")?;
    let blocks_group_1 = read_u16_be(values, cursor, "ActivePlayer blocks group 1")?;
    if blocks_group_1 != 0 {
        return Err(format!(
            "ActivePlayer blocks group 1 is 0x{blocks_group_1:04X}; InvSlots uses only blocks 3-8"
        ));
    }

    let mut blocks = [0u32; 32];
    for (index, block) in blocks.iter_mut().enumerate() {
        if blocks_group_0 & (1 << index) != 0 {
            *block = read_u32_be(values, cursor, "ActivePlayer block")?;
            if *block == 0 {
                return Err(format!("ActivePlayer group mask names empty block {index}"));
            }
        }
    }

    let mut has_inv_slots_parent = false;
    let mut changed_slots = Vec::new();
    for (block_index, block) in blocks.iter().copied().enumerate() {
        for bit in 0..32u32 {
            if block & (1 << bit) == 0 {
                continue;
            }
            let field = block_index as u32 * 32 + bit;
            match field {
                124 => has_inv_slots_parent = true,
                125..=265 => changed_slots.push((field - 125) as u16),
                _ => {
                    return Err(format!(
                        "ActivePlayer mask contains non-InvSlots field {field}"
                    ));
                }
            }
        }
    }
    if !has_inv_slots_parent {
        return Err("ActivePlayer InvSlots parent bit 124 is absent".to_string());
    }
    if changed_slots.len() != 1 {
        return Err(format!(
            "reviewed loot update requires exactly one InvSlots child, found {}",
            changed_slots.len()
        ));
    }

    let mut inv_slots = Vec::with_capacity(1);
    for slot in changed_slots {
        let (item_low, item_high) = read_packed_guid(values, cursor, "InvSlots item")?;
        if ((item_high >> 58) & 0x3F) as u8 != HIGH_GUID_ITEM || item_low == 0 {
            return Err(format!("InvSlots[{slot}] is not a non-empty Item GUID"));
        }
        if item_high & GLOBAL_GUID_RESERVED_HIGH_BITS_MASK != 0 {
            return Err(format!(
                "InvSlots[{slot}] Item GUID sets reserved high-word bits 0..41"
            ));
        }
        inv_slots.push(InvSlotValue {
            slot,
            item: ExactObjectGuid {
                low: item_low,
                high: item_high,
            },
        });
    }
    Ok(inv_slots)
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DecodedSingleLootItemRequest {
    pub(super) loot_obj: ExactObjectGuid,
    pub(super) loot_list_id: u8,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DecodedIssue106ItemPushResult {
    pub(super) player: ExactObjectGuid,
    pub(super) slot_in_bag: i32,
    pub(super) quantity: i32,
    pub(super) item_guid: ExactObjectGuid,
    pub(super) item_entry: i32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DecodedIssue106ItemCreate {
    pub(super) map_id: u16,
    pub(super) item_guid: ExactObjectGuid,
    pub(super) owner: ExactObjectGuid,
    pub(super) contained_in: ExactObjectGuid,
    pub(super) item_entry: i32,
    pub(super) stack_count: u32,
}

/// Reconstruct the compressed linear spline represented by one MonsterMove.
///
/// C++ emits only the final point plus signed quarter-yard deltas from the
/// midpoint of the first/current and final points. The returned vector is
/// `[current, intermediates..., final]`.
pub fn reconstruct_monster_move_path(body: &MonsterMoveBody) -> Result<Vec<[f32; 3]>, String> {
    let [final_point] = body.points.as_slice() else {
        return Err(format!(
            "compressed issue-#24 movement requires exactly one endpoint, found {}",
            body.points.len()
        ));
    };
    let start = body.current_position.xyz();
    let end = final_point.xyz();
    let middle = [
        (start[0] + end[0]) * 0.5,
        (start[1] + end[1]) * 0.5,
        (start[2] + end[2]) * 0.5,
    ];
    let mut path = Vec::with_capacity(body.packed_deltas.len() + 2);
    path.push(start);
    for packed in &body.packed_deltas {
        let delta = unpack_monster_move_delta(*packed);
        path.push([
            middle[0] - delta[0],
            middle[1] - delta[1],
            middle[2] - delta[2],
        ]);
    }
    path.push(end);
    Ok(path)
}

/// Validate one issue-#24 movement body independently of cross-runtime diff.
///
/// This proves that the packet belongs to the reserved map-1 Tender fixture
/// and carries a compressed path around (not through) the declared missing
/// navmesh square.
pub fn validate_detour_chase_monster_move(
    decoded: &DecodedMonsterMoveBody,
) -> Result<Vec<[f32; 3]>, String> {
    validate_detour_chase_monster_move_for_side(decoded, false)
}

pub fn validate_legacy_cpp_detour_chase_monster_move(
    decoded: &DecodedMonsterMoveBody,
) -> Result<Vec<[f32; 3]>, String> {
    validate_detour_chase_monster_move_for_side(decoded, true)
}

pub(super) fn validate_detour_chase_monster_move_for_side(
    decoded: &DecodedMonsterMoveBody,
    legacy_cpp: bool,
) -> Result<Vec<[f32; 3]>, String> {
    let movement = &decoded.body;
    if movement.mover != ISSUE_24_CREATURE_IDENTITY {
        return Err(format!(
            "MonsterMove mover {:?} is not the issue-#24 fixture {:?}",
            movement.mover, ISSUE_24_CREATURE_IDENTITY
        ));
    }
    if decoded.mover_runtime_counter == 0 {
        return Err("issue-#24 fixture movement has zero runtime GUID counter".to_string());
    }
    if decoded.spline_id == 0 {
        return Err("issue-#24 fixture movement has zero spline ID".to_string());
    }
    require_position_near(
        movement.current_position,
        ISSUE_24_CREATURE_START,
        ISSUE_24_POSITION_EPSILON,
        "MonsterMove current position",
    )?;
    if movement.destination != WirePosition::new(0, 0, 0) {
        return Err(format!(
            "outer MovementMonsterSpline destination is {:?}, expected exact positive-zero C++ default",
            movement.destination
        ));
    }
    if movement.crz_teleport
        || movement.stop_distance_tolerance != 0
        || movement.flags & 0x0040_0000 != 0
        || movement.elapsed != 0
        || movement.move_time == 0
        || movement.fade_object_time != 0
        || movement.mode != 0
        || movement.transport != (ExactObjectGuid { low: 0, high: 0 })
        || movement.vehicle_seat != -1
        || movement.vehicle_exit_voluntary
        || movement.interpolate
        || movement.spline_filter.is_some()
        || movement.spell_effect_extra.is_some()
        || movement.jump_extra.is_some()
        || movement.anim_tier_transition.is_some()
    {
        return Err(
            "MonsterMove is not the plain, compressed, non-transport chase shape".to_string(),
        );
    }
    let MonsterMoveFaceBody::Target {
        direction_bits,
        target,
    } = &movement.face
    else {
        return Err("MonsterMove does not face the chase target like C++".to_string());
    };
    if *target
        != (ExactObjectGuid {
            low: ISSUE_24_CAPTURE_PLAYER_LOW,
            high: ISSUE_24_CAPTURE_PLAYER_HIGH,
        })
    {
        return Err(format!(
            "MonsterMove facing target {target:?} is not disposable character 15 in realm 1"
        ));
    }
    let direction = f32::from_bits(*direction_bits);
    if (direction - std::f32::consts::FRAC_PI_2).abs() > 0.05 {
        return Err(format!(
            "MonsterMove target-facing direction {direction} is not approximately +pi/2"
        ));
    }
    if movement.packed_deltas.is_empty() {
        return Err(
            "MonsterMove has no packed intermediate; a straight path cannot prove the detour"
                .to_string(),
        );
    }

    let path = reconstruct_monster_move_path(movement)?;
    let destination = ISSUE_24_PLAYER_DESTINATION.xyz();
    let endpoint = *path
        .last()
        .expect("reconstruction always includes start and endpoint");
    let endpoint_distance_2d =
        ((endpoint[0] - destination[0]).powi(2) + (endpoint[1] - destination[1]).powi(2)).sqrt();
    let endpoint_distance_3d =
        (endpoint_distance_2d.powi(2) + (endpoint[2] - destination[2]).powi(2)).sqrt();
    let endpoint_invalid = if legacy_cpp {
        endpoint_distance_2d > 6.0
            || destination[2] - endpoint[2] < 20.0
            || (endpoint[2] - 190.721_01).abs() > 1.0
    } else {
        endpoint_distance_3d > 6.0
    };
    if endpoint[1] <= ISSUE_24_OBSTACLE_MAX_Y || endpoint_invalid {
        return Err(format!(
            "{} chase endpoint {endpoint:?} does not satisfy the reviewed destination contract for {destination:?}",
            if legacy_cpp {
                "legacy C++"
            } else {
                "repaired Rust"
            }
        ));
    }
    if !segment_intersects_issue_24_obstacle(path[0], destination) {
        return Err(
            "fixture start-to-player line does not cross the declared obstacle".to_string(),
        );
    }
    if !path[1..path.len() - 1].iter().any(|point| {
        point[0] < ISSUE_24_OBSTACLE_MIN_X - 0.1 || point[0] > ISSUE_24_OBSTACLE_MAX_X + 0.1
    }) {
        return Err(format!(
            "compressed path {path:?} has no lateral bend outside the obstacle"
        ));
    }
    if path
        .windows(2)
        .any(|segment| segment_intersects_issue_24_obstacle(segment[0], segment[1]))
    {
        return Err(format!(
            "compressed path {path:?} intersects the missing navmesh square"
        ));
    }
    Ok(path)
}

/// Validate the complete three-packet issue-#24 capture contract.
pub fn validate_detour_chase_capture(capture: &Capture) -> Result<(), String> {
    validate_detour_chase_capture_for_side(capture, false)
}

pub fn validate_legacy_cpp_detour_chase_capture(capture: &Capture) -> Result<(), String> {
    validate_detour_chase_capture_for_side(capture, true)
}

pub(super) fn validate_detour_chase_capture_for_side(
    capture: &Capture,
    legacy_cpp: bool,
) -> Result<(), String> {
    const EXPECTED: [(Direction, u32, u16); 3] = [
        (Direction::C2S, 1, CMSG_MOVE_HEARTBEAT),
        (Direction::S2C, 1, SMSG_ON_MONSTER_MOVE),
        (Direction::C2S, 1, CMSG_PING),
    ];
    if capture.packets.len() != EXPECTED.len() {
        return Err(format!(
            "{} contains {} packet(s); detour-chase contract requires exactly {}",
            capture.source,
            capture.packets.len(),
            EXPECTED.len()
        ));
    }
    for (index, (packet, (direction, connection_id, opcode))) in
        capture.packets.iter().zip(EXPECTED).enumerate()
    {
        if packet.direction != direction
            || packet.connection_id != connection_id
            || packet.opcode != opcode
        {
            return Err(format!(
                "{} packet {index} is {} conn={} 0x{:04X}; detour contract requires {} conn={} 0x{opcode:04X}",
                capture.source,
                packet.direction,
                packet.connection_id,
                packet.opcode,
                direction,
                connection_id
            ));
        }
    }
    validate_issue_24_heartbeat(&capture.packets[0].body)?;
    let movement = decode_monster_move_body(&capture.packets[1].body)?;
    if legacy_cpp {
        validate_legacy_cpp_detour_chase_monster_move(&movement)?;
    } else {
        validate_detour_chase_monster_move(&movement)?;
    }
    if capture.packets[2].body != ISSUE_24_PING_BODY {
        return Err(format!(
            "CMSG_PING fence body is {:02X?}, expected fixed DTOR/zero-latency body {:02X?}",
            capture.packets[2].body, ISSUE_24_PING_BODY
        ));
    }
    Ok(())
}

pub(super) fn validate_issue_24_heartbeat(body: &[u8]) -> Result<(), String> {
    let mut cursor = 0usize;
    let (player_low, player_high) = read_packed_guid(body, &mut cursor, "MovementInfo.Guid")?;
    if (player_low, player_high) != (ISSUE_24_CAPTURE_PLAYER_LOW, ISSUE_24_CAPTURE_PLAYER_HIGH) {
        return Err(format!(
            "heartbeat player GUID is ({player_low:#018X}, {player_high:#018X}), expected disposable character 15 in realm 1"
        ));
    }
    for label in ["MovementFlags", "MovementFlags2", "MovementFlags3"] {
        if read_u32(body, &mut cursor, label)? != 0 {
            return Err(format!("{label} is nonzero in deterministic heartbeat"));
        }
    }
    if read_u32(body, &mut cursor, "Time")? != 0 {
        return Err("heartbeat client time is not deterministic zero".to_string());
    }
    let position = read_wire_position(body, &mut cursor, "Position")?;
    if position != ISSUE_24_PLAYER_DESTINATION {
        return Err(format!(
            "heartbeat destination {:?} differs from fixture destination {:?}",
            position.xyz(),
            ISSUE_24_PLAYER_DESTINATION.xyz()
        ));
    }
    let orientation = read_f32_bits(body, &mut cursor, "Orientation")?;
    if orientation != ISSUE_24_PLAYER_DESTINATION_ORIENTATION_BITS {
        return Err(format!(
            "heartbeat orientation is 0x{orientation:08X}, expected fixture orientation 0x{ISSUE_24_PLAYER_DESTINATION_ORIENTATION_BITS:08X}"
        ));
    }
    for label in ["Pitch", "StepUpStartElevation"] {
        if read_f32_bits(body, &mut cursor, label)? != 0 {
            return Err(format!("{label} is nonzero in deterministic heartbeat"));
        }
    }
    if read_u32(body, &mut cursor, "RemoveMovementForcesCount")? != 0
        || read_u32(body, &mut cursor, "MoveIndex")? != 0
    {
        return Err("heartbeat force count or move index is nonzero".to_string());
    }
    let optional_bits = read_u8(body, &mut cursor, "MovementInfo optional bit byte")?;
    if optional_bits != 0 {
        return Err(format!(
            "heartbeat optional movement bit byte is 0x{optional_bits:02X}, expected zero"
        ));
    }
    if cursor != body.len() {
        return Err(format!(
            "trailing bytes after deterministic heartbeat: decoded {cursor} of {}",
            body.len()
        ));
    }
    Ok(())
}

/// Validate the semantic payload boundary for the future
/// `creature-spell-casting-v1` required flow.
///
/// Exact outer action boundaries and socket routing remain the responsibility
/// of the requirement manifest. This independent payload contract requires
/// exactly one adjacent server `SpellStart -> SpellGo` pair, correlated
/// Creature CasterGUID/CasterUnit and CastID values, identical spell/visual/
/// target data across that pair, a normal Cast GUID whose stable identity
/// matches realm/map/spell, EMPTY OriginalCastID, and complete fail-closed
/// SpellCastData decoding on both packets.
pub fn validate_creature_spell_casting_capture(capture: &Capture) -> Result<(), String> {
    let spell_starts = capture
        .packets
        .iter()
        .enumerate()
        .filter(|(_, packet)| packet.opcode == SMSG_SPELL_START)
        .collect::<Vec<_>>();
    let spell_goes = capture
        .packets
        .iter()
        .enumerate()
        .filter(|(_, packet)| packet.opcode == SMSG_SPELL_GO)
        .collect::<Vec<_>>();
    let [(start_index, start_packet)] = spell_starts.as_slice() else {
        return Err(format!(
            "{} contains {} SMSG_SPELL_START packet(s); creature-spell-casting-v1 requires exactly one",
            capture.source,
            spell_starts.len()
        ));
    };
    let [(go_index, go_packet)] = spell_goes.as_slice() else {
        return Err(format!(
            "{} contains {} SMSG_SPELL_GO packet(s); creature-spell-casting-v1 requires exactly one",
            capture.source,
            spell_goes.len()
        ));
    };
    if start_packet.direction != Direction::S2C || go_packet.direction != Direction::S2C {
        return Err(format!(
            "{} carries SpellStart/SpellGo on {}/{}, expected s2c/s2c",
            capture.source, start_packet.direction, go_packet.direction
        ));
    }
    if *go_index != *start_index + 1 {
        return Err(format!(
            "{} does not contain adjacent SMSG_SPELL_START -> SMSG_SPELL_GO (indices {start_index} and {go_index})",
            capture.source
        ));
    }

    let start = decode_spell_start_body(&start_packet.body)?;
    let go = decode_spell_go_body(&go_packet.body)?;
    validate_creature_spell_cast_shape(
        &start.body.cast,
        start.exact_caster_guid,
        start.exact_caster_unit,
        start.cast_id,
    )?;
    validate_decoded_creature_spell_go(&go)?;
    validate_issue_26_creature_spell_start(&start)?;
    validate_issue_26_creature_spell_go(&go)?;

    let mut mismatches = Vec::new();
    if start.exact_caster_guid != go.exact_caster_guid
        || start.exact_caster_unit != go.exact_caster_unit
    {
        mismatches.push("caster");
    }
    if start.cast_id != go.cast_id {
        mismatches.push("cast_id");
    }
    if start.body.cast.spell_id != go.body.spell_id {
        mismatches.push("spell_id");
    }
    if start.body.cast.spell_visual_id != go.body.spell_visual_id {
        mismatches.push("spell_visual_id");
    }
    if start.body.cast.target != go.body.target {
        mismatches.push("target");
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "SMSG_SPELL_START and SMSG_SPELL_GO do not correlate field(s): {}",
            mismatches.join(", ")
        ))
    }
}

pub(super) fn validate_issue_26_creature_spell_start(
    start: &DecodedSpellStartBody,
) -> Result<(), String> {
    validate_issue_26_creature_spell_common(&start.body.cast, "SMSG_SPELL_START")?;
    if start.body.cast.cast_flags != ISSUE_26_START_CAST_FLAGS {
        return Err(format!(
            "SMSG_SPELL_START CastFlags is 0x{:08X}, expected issue-#26 fixture value 0x{ISSUE_26_START_CAST_FLAGS:08X}",
            start.body.cast.cast_flags
        ));
    }
    if start.body.cast_time != 0 {
        return Err(format!(
            "SMSG_SPELL_START CastTime is {}, expected instant issue-#26 fixture value 0",
            start.body.cast_time
        ));
    }
    if !start.body.cast.hit_targets.is_empty() {
        return Err(
            "SMSG_SPELL_START contains HitTargets; issue-#26 requires result topology only in SMSG_SPELL_GO"
                .to_string(),
        );
    }
    Ok(())
}

pub(super) fn validate_issue_26_creature_spell_go(go: &DecodedSpellGoBody) -> Result<(), String> {
    let victim = validate_issue_26_creature_spell_common(&go.body, "SMSG_SPELL_GO")?;
    if go.body.cast_flags != ISSUE_26_GO_CAST_FLAGS {
        return Err(format!(
            "SMSG_SPELL_GO CastFlags is 0x{:08X}, expected issue-#26 fixture value 0x{ISSUE_26_GO_CAST_FLAGS:08X}",
            go.body.cast_flags
        ));
    }
    let expected_hits = [CorrelatedSpellGuidBody::Exact { guid: victim }];
    if go.body.hit_targets.as_slice() != expected_hits {
        return Err(
            "SMSG_SPELL_GO HitTargets is not exactly the explicit player victim once".to_string(),
        );
    }
    Ok(())
}

pub(super) fn validate_issue_26_creature_spell_common(
    body: &SpellGoBody,
    packet: &str,
) -> Result<ExactObjectGuid, String> {
    let caster = body.caster_guid;
    if caster.high_type != HIGH_GUID_CREATURE
        || caster.realm_id != ISSUE_26_REALM_ID
        || caster.map_id != ISSUE_26_MAP_ID
        || caster.entry != ISSUE_26_CREATURE_ENTRY
        || caster.subtype != 0
        || caster.server_id != 0
    {
        return Err(format!(
            "{packet} caster identity {:?} is not issue-#26 Cabal Interrogator entry {ISSUE_26_CREATURE_ENTRY} on realm {ISSUE_26_REALM_ID}, map {ISSUE_26_MAP_ID}",
            caster
        ));
    }
    if body.spell_id != ISSUE_26_SPELL_ID {
        return Err(format!(
            "{packet} SpellID is {}, expected issue-#26 Eviscerate {ISSUE_26_SPELL_ID}",
            body.spell_id
        ));
    }
    if body.spell_visual_id != ISSUE_26_SPELL_X_SPELL_VISUAL_ID {
        return Err(format!(
            "{packet} SpellXSpellVisualID is {}, expected issue-#26 fixture value {ISSUE_26_SPELL_X_SPELL_VISUAL_ID}",
            body.spell_visual_id
        ));
    }
    if body.cast_flags_ex != 0 {
        return Err(format!(
            "{packet} CastFlagsEx is 0x{:08X}, expected 0",
            body.cast_flags_ex
        ));
    }
    if body.missile_travel_time != 0 || body.missile_pitch_bits != 0.0_f32.to_bits() {
        return Err(format!(
            "{packet} carries a missile trajectory; issue-#26 Eviscerate is zero-speed"
        ));
    }
    if body.dest_loc_spell_cast_index != 0 {
        return Err(format!(
            "{packet} DestLocSpellCastIndex is {}, expected 0",
            body.dest_loc_spell_cast_index
        ));
    }
    if body.immunities_school != 0 || body.immunities_value != 0 {
        return Err(format!(
            "{packet} carries creature immunities; issue-#26 fixture expects none"
        ));
    }
    if body.prediction_points != 0
        || body.prediction_type != 0
        || body.prediction_beacon != (ExactObjectGuid { low: 0, high: 0 })
    {
        return Err(format!(
            "{packet} carries heal prediction; issue-#26 fixture expects none"
        ));
    }

    let CorrelatedSpellGuidBody::Exact { guid: victim } = &body.target.unit else {
        return Err(format!(
            "{packet} explicit unit target is not a distinct player victim"
        ));
    };
    let victim = *victim;
    let stable_victim = stable_object_guid(victim.low, victim.high);
    if exact_guid_high_type(victim) != HIGH_GUID_PLAYER
        || stable_victim.realm_id != ISSUE_26_REALM_ID
        || stable_victim.map_id != 0
        || stable_victim.entry != 0
        || stable_victim.subtype != 0
        || stable_victim.server_id != 0
        || victim.low & OBJECT_GUID_COUNTER_MASK != ISSUE_26_PLAYER_COUNTER
    {
        return Err(format!(
            "{packet} explicit unit target {victim:?} is not canonical issue-#26 realm-{ISSUE_26_REALM_ID} Player {ISSUE_26_PLAYER_COUNTER}"
        ));
    }
    let target = &body.target;
    if target.flags != ISSUE_26_UNIT_TARGET_FLAGS
        || target.item != (ExactObjectGuid { low: 0, high: 0 })
        || target.src_location.is_some()
        || target.dst_location.is_some()
        || target.orientation_bits.is_some()
        || target.map_id.is_some()
        || !target.name.is_empty()
    {
        return Err(format!(
            "{packet} SpellTargetData is not the exact issue-#26 unit-only target topology"
        ));
    }
    if !body.miss_targets.is_empty() || !body.miss_status.is_empty() {
        return Err(format!(
            "{packet} carries miss topology; issue-#26 fixture requires a successful hit"
        ));
    }
    if !body.remaining_power.is_empty()
        || body.remaining_runes.is_some()
        || !body.target_points.is_empty()
        || body.ammo_display_id.is_some()
        || body.ammo_inventory_type.is_some()
    {
        return Err(format!(
            "{packet} carries power/rune/target-point/ammo optionals absent from issue-#26 Eviscerate"
        ));
    }
    Ok(victim)
}

pub(super) fn require_position_near(
    actual: WirePosition,
    expected: WirePosition,
    epsilon: f32,
    label: &str,
) -> Result<(), String> {
    let actual = actual.xyz();
    let expected = expected.xyz();
    if actual
        .into_iter()
        .zip(expected)
        .any(|(actual, expected)| (actual - expected).abs() > epsilon)
    {
        return Err(format!(
            "{label} {actual:?} differs from expected {expected:?} by more than {epsilon}"
        ));
    }
    Ok(())
}

pub(super) fn segment_intersects_issue_24_obstacle(start: [f32; 3], end: [f32; 3]) -> bool {
    // Shrink the open rectangle slightly so a Detour segment along its exact
    // boundary is accepted while a segment through its interior fails.
    let min_x = ISSUE_24_OBSTACLE_MIN_X + 0.05;
    let max_x = ISSUE_24_OBSTACLE_MAX_X - 0.05;
    let min_y = ISSUE_24_OBSTACLE_MIN_Y + 0.05;
    let max_y = ISSUE_24_OBSTACLE_MAX_Y - 0.05;
    let mut t_min = 0.0f32;
    let mut t_max = 1.0f32;
    for (origin, delta, min, max) in [
        (start[0], end[0] - start[0], min_x, max_x),
        (start[1], end[1] - start[1], min_y, max_y),
    ] {
        if delta.abs() <= f32::EPSILON {
            if origin <= min || origin >= max {
                return false;
            }
            continue;
        }
        let first = (min - origin) / delta;
        let second = (max - origin) / delta;
        let enter = first.min(second);
        let leave = first.max(second);
        t_min = t_min.max(enter);
        t_max = t_max.min(leave);
        if t_min >= t_max {
            return false;
        }
    }
    t_max > 0.0 && t_min < 1.0
}

/// Validate the complete, correlated payload contract of the issue-#106
/// single-item capture.
///
/// Opcode equality alone is insufficient evidence: an unrelated item push or
/// two identically malformed packets could otherwise compare clean. This pins
/// the one-request count, deterministic fixture identities, request/removal
/// LootObject and list id, recipient, quantity, awarded Item GUID, inventory
/// item CreateObject, slot update, and fixed ping fence on each side
/// independently.
pub fn validate_loot_single_item_claim_capture(capture: &Capture) -> Result<(), String> {
    validate_issue_106_capture_topology(capture)?;

    let request = decode_single_loot_item_request(&capture.packets[0].body)?;
    let created = decode_issue_106_item_create(&capture.packets[1].body)?;
    let removed = decode_loot_removed_body_with_counter(&capture.packets[2].body)?;
    validate_issue_106_request_removal(request, removed)?;

    let pushed = decode_issue_106_item_push_result(&capture.packets[3].body)?;
    validate_issue_106_created_grant(created, pushed)?;
    validate_issue_106_inventory_update(&capture.packets[4].body, pushed)?;

    if capture.packets[5].body != ISSUE_106_PING_BODY {
        return Err(format!(
            "CMSG_PING fence body is {:02X?}, expected fixed TOOL/zero-latency body {:02X?}",
            capture.packets[5].body, ISSUE_106_PING_BODY
        ));
    }
    Ok(())
}

pub(super) fn validate_issue_106_capture_topology(capture: &Capture) -> Result<(), String> {
    const EXPECTED: [(Direction, u32, u16); 6] = [
        (Direction::C2S, 1, CMSG_LOOT_ITEM),
        (Direction::S2C, 1, SMSG_UPDATE_OBJECT),
        (Direction::S2C, 1, SMSG_LOOT_REMOVED),
        (Direction::S2C, 0, SMSG_ITEM_PUSH_RESULT),
        (Direction::S2C, 1, SMSG_UPDATE_OBJECT),
        (Direction::C2S, 1, CMSG_PING),
    ];
    if capture.packets.len() != EXPECTED.len() {
        return Err(format!(
            "{} contains {} packet(s); the semantic loot-claim contract requires exactly {}",
            capture.source,
            capture.packets.len(),
            EXPECTED.len()
        ));
    }
    for (index, (packet, (direction, connection_id, opcode))) in
        capture.packets.iter().zip(EXPECTED).enumerate()
    {
        if packet.direction != direction
            || packet.connection_id != connection_id
            || packet.opcode != opcode
        {
            return Err(format!(
                "{} packet {index} is {} conn={} 0x{:04X}; semantic contract requires {} conn={} 0x{opcode:04X}",
                capture.source,
                packet.direction,
                packet.connection_id,
                packet.opcode,
                direction,
                connection_id
            ));
        }
    }
    Ok(())
}
