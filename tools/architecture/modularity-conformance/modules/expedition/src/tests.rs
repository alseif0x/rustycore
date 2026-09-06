use super::*;
use conformance_contract::decode_canonical;

#[test]
fn variable_codec_has_exact_canonical_roundtrip_at_every_supported_length() {
    for count in 0..=MAX_CHECKPOINTS {
        let state = ExpeditionState {
            resets: 7,
            accepted_total: 100,
            checkpoints: (1..=count as u8).collect(),
        };
        let encoded = state.encode();
        assert_eq!(encoded.len(), HEADER_BYTES + count);
        assert_eq!(
            decode_canonical::<ExpeditionState>(&encoded, STATE_LIMIT),
            Ok(state)
        );
    }
    assert_eq!(ExpeditionState::default().encode().len(), 15);
    assert_eq!(Expedition::manifest().state_limit, 23);
}

#[test]
fn codec_rejects_noncanonical_shapes_instead_of_sorting_or_truncating_them() {
    let state = ExpeditionState {
        resets: 0,
        accepted_total: 3,
        checkpoints: vec![2, 5, 8],
    };
    let bytes = state.encode();
    for length in 0..bytes.len() {
        assert_eq!(
            ExpeditionState::decode(&bytes[..length]),
            Err(Fault::Invalid)
        );
    }
    for (index, value) in [(0, 0), (1, 2), (14, 2), (15, 0), (16, 2), (17, 32)] {
        let mut invalid = bytes.clone();
        invalid[index] = value;
        assert_eq!(ExpeditionState::decode(&invalid), Err(Fault::Invalid));
    }
    let mut trailing = bytes.clone();
    trailing.push(9);
    assert_eq!(ExpeditionState::decode(&trailing), Err(Fault::Invalid));
    let mut reordered = bytes.clone();
    reordered.swap(15, 17);
    assert_eq!(ExpeditionState::decode(&reordered), Err(Fault::Invalid));
    let mut lost_history = bytes;
    lost_history[6..14].copy_from_slice(&2_u64.to_le_bytes());
    assert_eq!(ExpeditionState::decode(&lost_history), Err(Fault::Invalid));
    assert_eq!(
        ExpeditionState::decode(&[0; STATE_LIMIT + 1]),
        Err(Fault::Limit)
    );
}
