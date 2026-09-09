use super::*;

fn write_test_msb_bits(buffer: &mut [u8], bit_offset: usize, bit_count: usize, value: u32) {
    assert!(bit_count <= 32);
    assert!(bit_offset + bit_count <= buffer.len() * 8);
    for index in 0..bit_count {
        let source_bit = (value >> (bit_count - 1 - index)) & 1;
        if source_bit != 0 {
            let destination = bit_offset + index;
            buffer[destination / 8] |= 1 << (7 - destination % 8);
        }
    }
}
fn creature_spell_test_body(
    is_spell_go: bool,
    miss: bool,
) -> (Vec<u8>, usize, usize, (u64, u64), (u64, u64)) {
    let caster = create_creature_guid_raw(
        CREATURE_SPELL_FIXTURE_MAP_ID,
        CREATURE_SPELL_FIXTURE_ENTRY,
        9_876,
    );
    let player = create_player_guid_raw(CREATURE_SPELL_FIXTURE_CHARACTER_GUID, 1);
    let cast_id = (0x1234, 0x0400_0000_0000_0001);
    let mut body = Vec::new();
    body.extend(build_packed_guid(caster.0, caster.1));
    body.extend(build_packed_guid(caster.0, caster.1));
    body.extend(build_packed_guid(cast_id.0, cast_id.1));
    body.extend(build_packed_guid(0, 0));
    body.extend(CREATURE_SPELL_FIXTURE_SPELL_ID.to_le_bytes());
    body.extend(CREATURE_SPELL_FIXTURE_SPELL_X_VISUAL_ID.to_le_bytes());
    body.extend(
        (if is_spell_go {
            0x0000_0100u32
        } else {
            0x0000_0002u32
        })
        .to_le_bytes(),
    );
    body.extend(0u32.to_le_bytes()); // CastFlagsEx
    body.extend(
        (if is_spell_go { 12_345u32 } else { 0u32 }).to_le_bytes(), // CastTime/timestamp
    );
    body.extend(0u32.to_le_bytes()); // trajectory time
    body.extend(0u32.to_le_bytes()); // trajectory pitch
    body.push(0); // destination index
    body.extend(0u32.to_le_bytes()); // immunity school
    body.extend(0u32.to_le_bytes()); // immunity value
    body.extend(0u32.to_le_bytes()); // heal prediction points
    body.push(0); // heal prediction type
    body.extend(build_packed_guid(0, 0)); // prediction beacon
    let counts_offset = body.len();
    let mut counts = [0u8; 10];
    write_test_msb_bits(&mut counts, 0, 16, u32::from(is_spell_go && !miss));
    write_test_msb_bits(&mut counts, 16, 16, u32::from(miss));
    write_test_msb_bits(&mut counts, 32, 16, u32::from(miss));
    body.extend(counts);
    let target_offset = body.len();
    let mut target_header = [0u8; 5];
    write_test_msb_bits(&mut target_header, 0, 28, 0x2);
    body.extend(target_header);
    body.extend(build_packed_guid(player.0, player.1));
    body.extend(build_packed_guid(0, 0));
    if is_spell_go && !miss {
        body.extend(build_packed_guid(player.0, player.1));
    }
    if miss {
        body.extend(build_packed_guid(player.0, player.1));
    }
    if is_spell_go {
        body.push(0); // FullCombatLog=false plus zero padding
    }
    (body, counts_offset, target_offset, caster, player)
}
fn equipment_set_test_options() -> EquipmentSetSmokeOptions {
    EquipmentSetSmokeOptions {
        phase: EquipmentSetSmokePhase::Save,
        set_type: 0,
        set_id: 7,
        set_name: "QA Equipment".to_string(),
        set_icon: "INV_Sword_01".to_string(),
        expected_guid: None,
        save_barrier: None,
        timeout_secs: 10,
    }
}
fn vendor_inventory_fixture(has_bonus: u8, modifier_count: u8) -> (Vec<u8>, Vec<u8>) {
    let vendor_guid = build_packed_guid(0x1234, 0xF130_0000_485D_0001);
    let mut payload = vendor_guid.clone();
    payload.push(0); // VendorInventoryReason::None.
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(&0u64.to_le_bytes()); // Price.
    payload.extend_from_slice(&37i32.to_le_bytes()); // MUID.
    payload.extend_from_slice(&1i32.to_le_bytes()); // Item type.
    payload.extend_from_slice(&0i32.to_le_bytes()); // Durability.
    payload.extend_from_slice(&1i32.to_le_bytes()); // Stack count.
    payload.extend_from_slice(&(-1i32).to_le_bytes()); // Unlimited quantity.
    payload.extend_from_slice(&1642i32.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes()); // Player condition failure.
    payload.push(0); // Vendor flags.
    payload.extend_from_slice(&30183i32.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes()); // Random seed.
    payload.extend_from_slice(&0i32.to_le_bytes()); // Random property.
    payload.push(has_bonus);
    payload.push(modifier_count);
    (payload, vendor_guid)
}
fn pack_msb_fields(fields: &[(u32, usize)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut current = 0u8;
    let mut used = 0usize;
    for &(value, width) in fields {
        for bit in (0..width).rev() {
            current |= (((value >> bit) & 1) as u8) << (7 - used);
            used += 1;
            if used == 8 {
                bytes.push(current);
                current = 0;
                used = 0;
            }
        }
    }
    if used != 0 {
        bytes.push(current);
    }
    bytes
}
fn bind_spell_go_fixture(
    caster_low: u64,
    caster_high: u64,
    player_low: u64,
    player_high: u64,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(build_packed_guid(caster_low, caster_high));
    payload.extend(build_packed_guid(caster_low, caster_high));
    payload.extend(build_packed_guid(1, 0));
    payload.extend(build_packed_guid(1, 0));
    payload.extend_from_slice(&3286u32.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&0x0004_0101u32.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&1234u32.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes());
    payload.extend_from_slice(&0f32.to_le_bytes());
    payload.push(0);
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.push(0);
    payload.extend(build_packed_guid(0, 0));
    payload.extend(pack_msb_fields(&[
        (1, 16),
        (0, 16),
        (0, 16),
        (0, 9),
        (0, 1),
        (0, 16),
        (0, 1),
        (0, 1),
    ]));
    payload.extend(pack_msb_fields(&[(0x2, 28), (0, 4), (0, 7)]));
    payload.extend(build_packed_guid(player_low, player_high));
    payload.extend(build_packed_guid(0, 0));
    payload.extend(build_packed_guid(player_low, player_high));
    payload.push(0);
    payload
}
fn rested_xp_create_object_fixture(
    map_id: u16,
    entry: u32,
    counter: u64,
    x: f32,
    y: f32,
    z: f32,
) -> Vec<u8> {
    let (low, high) = create_creature_guid_raw(map_id, entry, counter);
    let mut payload = vec![1]; // CreateObject1
    payload.extend(build_packed_guid(low, high));
    payload.push(5); // TypeId::Unit
    payload.extend_from_slice(&[0x10, 0, 0]); // MovementUpdate bit 3, MSB-first.
    payload.extend(build_packed_guid(low, high)); // movement MoverGUID
    payload.extend_from_slice(&[0; 12]); // movement flags
    payload.extend_from_slice(&123u32.to_le_bytes()); // MoveTime
    payload.extend_from_slice(&x.to_le_bytes());
    payload.extend_from_slice(&y.to_le_bytes());
    payload.extend_from_slice(&z.to_le_bytes());
    payload
}
fn rested_xp_target_fixture(guid_counter: u64) -> ResolvedCreatureTarget {
    let (low, high) = create_creature_guid_raw(530, 15_274, guid_counter);
    ResolvedCreatureTarget {
        entry: 15_274,
        spawn_guid: 54_931,
        guid_counter,
        map_id: 530,
        x: 10_187.8,
        y: -6_347.56,
        z: 30.459,
        orientation: 0.0,
        packed_guid: build_packed_guid(low, high),
    }
}
fn issue20_item_create_fixture(
    item_guid: u64,
    item_entry: u32,
    owner_guid: u64,
    random_properties_id: i32,
    extra_enchantment: Option<(usize, i32)>,
    include_create_tail: bool,
) -> Vec<u8> {
    let (item_low, item_high) = item_guid_raw(item_guid, 1);
    let (owner_low, owner_high) = create_player_guid_raw(owner_guid, 1);
    let mut values = vec![0x01];
    values.extend_from_slice(&(item_entry as i32).to_le_bytes());
    values.extend_from_slice(&0u32.to_le_bytes());
    values.extend_from_slice(&1.0_f32.to_bits().to_le_bytes());
    values.extend(build_packed_guid(owner_low, owner_high));
    values.extend(build_packed_guid(owner_low, owner_high));
    values.extend(build_packed_guid(0, 0));
    values.extend(build_packed_guid(0, 0));
    values.extend_from_slice(&1i32.to_le_bytes());
    values.extend_from_slice(&0i32.to_le_bytes());
    for _ in 0..5 {
        values.extend_from_slice(&0i32.to_le_bytes());
    }
    values.extend_from_slice(&0u32.to_le_bytes());
    for slot in 0..ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT {
        let enchantment_id = extra_enchantment
            .filter(|(extra_slot, _)| *extra_slot == slot)
            .map(|(_, id)| id)
            .unwrap_or_else(|| issue20_expected_enchantment_ids()[slot]);
        values.extend_from_slice(&enchantment_id.to_le_bytes());
        values.extend_from_slice(&0u32.to_le_bytes());
        values.extend_from_slice(&0u16.to_le_bytes());
        values.extend_from_slice(&[0, 0]);
    }
    values.extend_from_slice(&0i32.to_le_bytes());
    values.extend_from_slice(&random_properties_id.to_le_bytes());
    if include_create_tail {
        values.extend_from_slice(&[0; 56]);
    }
    let mut block = vec![1];
    block.extend(build_packed_guid(item_low, item_high));
    block.push(1);
    block.extend_from_slice(&[0, 0, 0]);
    block.extend_from_slice(&0i32.to_le_bytes());
    block.extend_from_slice(&(values.len() as u32).to_le_bytes());
    block.extend(values);
    block
}

mod scenarios_1;
mod scenarios_2;
