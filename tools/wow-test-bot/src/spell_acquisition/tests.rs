use super::*;

#[test]
fn trainer_request_matches_cpp_packed_guid_then_signed_ids() {
    assert_eq!(
        trainer_buy(&[1, 0, 7], 42, 133),
        vec![1, 0, 7, 42, 0, 0, 0, 133, 0, 0, 0]
    );
}

#[test]
fn self_cast_has_cpp_optional_counts_bit_sections_and_explicit_unit() {
    let bytes = cast_self(133, (1, 0), (7, 0));
    // NPC/cast/target GUIDs use the same two-mask PackedGuid codec as C++.
    assert_eq!(&bytes[..3], &[1, 0, 1]);
    assert_eq!(&bytes[11..15], &[133, 0, 0, 0]);
    assert_eq!(&bytes[31..33], &[0, 0]); // no crafting NPC
    assert_eq!(&bytes[33..45], &[0; 12]); // all three counts
    assert_eq!(&bytes[45..52], &[0, 0, 0, 0, 0, 0x20, 0]);
    assert_eq!(&bytes[52..], &[1, 0, 7, 0, 0]);
}

#[test]
fn learned_rows_parse_favorite_and_all_optional_fields() {
    let payload = [
        2, 0, 0, 0, 0, 0, 0, 0, 0, 133, 0, 0, 0, 0xf0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 134, 0,
        0, 0, 0,
    ];
    assert_eq!(learned(&payload).unwrap(), vec![(133, true), (134, false)]);
    for length in 0..payload.len() {
        assert!(
            learned(&payload[..length]).is_err(),
            "accepted truncation at {length}"
        );
    }
    let mut trailing = payload.to_vec();
    trailing.push(0);
    assert!(learned(&trailing).is_err());
}

#[test]
fn malformed_counts_and_negative_spell_ids_fail_closed() {
    assert!(learned(&[255; 9]).is_err());
    assert!(learned(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0]).is_err());
}

#[test]
fn typed_plan_accepts_each_action_and_rejects_unknown_action_fields() {
    for plan in [
        r#"{"expected_spell":133,"action":"verify"}"#,
        r#"{"expected_spell":133,"action":"cast","spell":100,"cast_low":1,"cast_high":0}"#,
        r#"{"expected_spell":133,"action":"trainer","guid_low":7,"guid_high":1,"trainer_id":42,"offer_spell":133,"fee":10}"#,
        r#"{"expected_spell":133,"action":"trainer","guid_low":7,"guid_high":1,"trainer_id":42,"offer_spell":133,"fee":10,"gossip_option":0}"#,
        r#"{"expected_spell":6197,"action":"trainer","guid_low":7,"guid_high":1,"trainer_id":7,"offer_spell":6197,"fee":1140,"gossip_option":-1702912}"#,
        r#"{"expected_spell":133,"action":"trainer","guid_low":0,"guid_high":0,"trainer_id":42,"offer_spell":133,"fee":10,"spawn":{"entry":7,"map":530,"position":[1,2,3]}}"#,
    ] {
        serde_json::from_str::<Plan>(plan).unwrap();
    }
    assert!(serde_json::from_str::<Plan>(
        r#"{"expected_spell":133,"action":"verify","cleanup":true}"#
    )
    .is_err());
}
