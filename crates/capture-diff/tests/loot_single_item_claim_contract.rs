//! Fail-closed tests for the complete issue-#106 required-flow contract.

use capture_diff::flow::{load_flow, load_requirement};
use capture_diff::{Capture, Direction, pkt, rustdump};

fn committed_pair() -> (Capture, Capture) {
    let flow = load_flow("loot-single-item-claim").expect("committed loot flow");
    let cpp = pkt::parse_pkt_file(&flow.golden_pkt).expect("C++ fixture");
    let rust = rustdump::parse_rust_dump(&flow.reference_rust).expect("Rust fixture");
    (cpp, rust)
}

fn packet_index(capture: &Capture, direction: Direction, opcode: u16) -> usize {
    capture
        .packets
        .iter()
        .position(|packet| packet.direction == direction && packet.opcode == opcode)
        .expect("required packet")
}

#[test]
fn committed_pair_satisfies_correlated_single_claim_semantics() {
    let requirement = load_requirement("loot-single-item-claim").expect("required contract");
    let (cpp, rust) = committed_pair();

    requirement.validate_capture(&cpp).expect("C++ evidence");
    requirement.validate_capture(&rust).expect("Rust evidence");
}

#[test]
fn extra_item_push_on_wrong_socket_cannot_hide_behind_exact_anchor_count() {
    let requirement = load_requirement("loot-single-item-claim").expect("required contract");
    let (_, mut rust) = committed_pair();
    let push_index = packet_index(&rust, Direction::S2C, 0x2623);
    let mut duplicate = rust.packets[push_index].clone();
    duplicate.connection_id = 1;
    rust.packets.insert(push_index + 1, duplicate);

    let error = requirement
        .validate_capture(&rust)
        .expect_err("wrong-route duplicate must invalidate evidence");
    assert!(error.to_string().contains("contains 7 packet(s)"));
}

#[test]
fn loot_request_must_contain_one_correlated_object_and_list() {
    let requirement = load_requirement("loot-single-item-claim").expect("required contract");
    let (_, rust) = committed_pair();
    let request_index = packet_index(&rust, Direction::C2S, 0x3211);

    let mut multiple = rust.clone();
    multiple.packets[request_index].body[..4].copy_from_slice(&2_u32.to_le_bytes());
    let error = requirement
        .validate_capture(&multiple)
        .expect_err("Count=2 must fail");
    assert!(error.to_string().contains("contains 2 request(s)"));

    let mut other_object = rust.clone();
    // Count (4), packed low/high masks (2), then the LootObject low counter.
    other_object.packets[request_index].body[6] = 2;
    let error = requirement
        .validate_capture(&other_object)
        .expect_err("request/removal LootObj mismatch must fail");
    assert!(
        error
            .to_string()
            .contains("does not match SMSG_LOOT_REMOVED")
    );

    let mut other_list = rust;
    other_list.packets[request_index].body[11] = 1;
    let error = requirement
        .validate_capture(&other_list)
        .expect_err("a different LootListID must fail");
    assert!(error.to_string().contains("LootListID is 1"));
}

#[test]
fn item_create_is_one_owned_item_and_correlates_with_the_grant() {
    let requirement = load_requirement("loot-single-item-claim").expect("required contract");
    let (_, rust) = committed_pair();
    let create_index = rust
        .packets
        .iter()
        .enumerate()
        .find_map(|(index, packet)| {
            (packet.direction == Direction::S2C && packet.opcode == 0x27CB).then_some(index)
        })
        .expect("item CreateObject packet");

    let mut two_updates = rust.clone();
    two_updates.packets[create_index].body[..4].copy_from_slice(&2_u32.to_le_bytes());
    let error = requirement
        .validate_capture(&two_updates)
        .expect_err("a multi-block create packet must fail");
    assert!(error.to_string().contains("2 object updates"));

    let mut values_instead_of_create = rust.clone();
    values_instead_of_create.packets[create_index].body[11] = 0;
    let error = requirement
        .validate_capture(&values_instead_of_create)
        .expect_err("a non-CreateObject update must fail");
    assert!(error.to_string().contains("expected CreateObject (1)"));

    let mut non_item = rust.clone();
    non_item.packets[create_index].body[19] = 5;
    let error = requirement
        .validate_capture(&non_item)
        .expect_err("a Unit CreateObject must fail");
    assert!(error.to_string().contains("expected Item (1)"));

    let mut other_item_guid = rust.clone();
    // Top-level fields (11), UpdateType (1), packed GUID masks (2), then item low.
    other_item_guid.packets[create_index].body[14] ^= 1;
    let error = requirement
        .validate_capture(&other_item_guid)
        .expect_err("created and granted Item GUIDs must correlate");
    assert!(error.to_string().contains("does not match ItemPushResult"));

    let mut other_entry = rust.clone();
    other_entry.packets[create_index].body[32..36].copy_from_slice(&30_713_i32.to_le_bytes());
    let error = requirement
        .validate_capture(&other_entry)
        .expect_err("a foreign created item entry must fail");
    assert!(error.to_string().contains("expected fixture item 30712"));

    let mut other_owner = rust.clone();
    // Values start at 31; Owner masks start at 44 and its low byte is 46.
    other_owner.packets[create_index].body[46] = 16;
    let error = requirement
        .validate_capture(&other_owner)
        .expect_err("a foreign item owner must fail");
    assert!(error.to_string().contains("expected capture player"));

    let mut other_container = rust.clone();
    // ContainedIn immediately follows the five-byte Owner packed GUID; its
    // first nonzero low byte is body byte 51.
    other_container.packets[create_index].body[51] = 16;
    let error = requirement
        .validate_capture(&other_container)
        .expect_err("a foreign containing player must fail");
    assert!(error.to_string().contains("expected capture player"));

    let mut stack_two = rust;
    stack_two.packets[create_index].body[58..62].copy_from_slice(&2_u32.to_le_bytes());
    let error = requirement
        .validate_capture(&stack_two)
        .expect_err("StackCount=2 must fail the single-item contract");
    assert!(error.to_string().contains("stack=2"));
}

#[test]
fn item_push_recipient_quantity_item_and_inventory_value_are_correlated() {
    let requirement = load_requirement("loot-single-item-claim").expect("required contract");
    let (_, rust) = committed_pair();
    let push_index = packet_index(&rust, Direction::S2C, 0x2623);

    let mut other_player = rust.clone();
    // Packed PlayerGUID low byte follows its two masks.
    other_player.packets[push_index].body[2] = 16;
    let error = requirement
        .validate_capture(&other_player)
        .expect_err("wrong recipient must fail");
    assert!(error.to_string().contains("PlayerGUID"));

    let mut quantity_two = rust.clone();
    quantity_two.packets[push_index].body[14..18].copy_from_slice(&2_i32.to_le_bytes());
    let error = requirement
        .validate_capture(&quantity_two)
        .expect_err("Quantity=2 must fail");
    assert!(error.to_string().contains("quantity=2/1"));

    let mut other_slot = rust.clone();
    // Packed PlayerGUID occupies five bytes; Slot is byte 5 and SlotInBag is
    // bytes 6..10. The restored Doctor-key fixture uses keyring slot 106.
    other_slot.packets[push_index].body[6..10].copy_from_slice(&35_i32.to_le_bytes());
    let error = requirement
        .validate_capture(&other_slot)
        .expect_err("a backpack/bank/equipment destination must not satisfy this fixture");
    assert!(error.to_string().contains("slot_in_bag=35"));

    let mut other_item_guid = rust.clone();
    // ItemGUID masks start at 42; byte 44 is its first nonzero low byte.
    other_item_guid.packets[push_index].body[44] ^= 1;
    let error = requirement
        .validate_capture(&other_item_guid)
        .expect_err("CreateObject/ItemPush/InvSlots ItemGUID mismatch must fail");
    assert!(error.to_string().contains("does not match ItemPushResult"));

    let mut other_item_entry = rust;
    other_item_entry.packets[push_index].body[50..54].copy_from_slice(&30_713_i32.to_le_bytes());
    let error = requirement
        .validate_capture(&other_item_entry)
        .expect_err("wrong ItemInstance entry must fail");
    assert!(
        error
            .to_string()
            .contains("item CreateObject entry 30712 does not match ItemPushResult 30713")
    );
}

#[test]
fn inventory_update_must_name_the_same_awarded_item() {
    let requirement = load_requirement("loot-single-item-claim").expect("required contract");
    let (_, mut rust) = committed_pair();
    let inventory_index = rust
        .packets
        .iter()
        .enumerate()
        .rfind(|(_, packet)| packet.direction == Direction::S2C && packet.opcode == 0x27CB)
        .map(|(index, _)| index)
        .expect("post-claim InvSlots packet");
    // The InvSlots packed Item masks are bytes 39/40 and its first low byte is
    // 41 in the reviewed 46-byte Rust body.
    rust.packets[inventory_index].body[41] ^= 1;

    let error = requirement
        .validate_capture(&rust)
        .expect_err("InvSlots must contain the ItemPush/CreateObject GUID");
    assert!(error.to_string().contains("does not match InvSlots item"));
}

#[test]
fn deterministic_ping_payload_is_part_of_the_claim_boundary() {
    let requirement = load_requirement("loot-single-item-claim").expect("required contract");
    let (_, mut rust) = committed_pair();
    let ping_index = packet_index(&rust, Direction::C2S, 0x3768);
    rust.packets[ping_index].body[0] ^= 1;

    let error = requirement
        .validate_capture(&rust)
        .expect_err("a different fence serial must fail");
    assert!(error.to_string().contains("fixed TOOL/zero-latency"));
}
