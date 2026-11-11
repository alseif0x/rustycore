use super::*;

#[test]
fn player_guid_preserves_all_realm_bits_without_region_or_site() {
    let guid = player_guid(0x0201_8001, 17);
    assert_eq!(guid.high_value() as u64, 0x0A00_0400_0000_0000);
    assert_eq!(guid.low_value(), 17);
    assert_eq!(guid, player_guid(0xFEFF_8001, 17));
    let normal = player_guid(0x0201_0001, 17);
    assert_eq!(normal.high_value() as u64, 0x0800_0400_0000_0000);
    assert_eq!(normal.low_value(), 17);
}

#[test]
fn raw_player_guid_assembly_preserves_counter_bits_without_admission_claim() {
    // Encoder-only bounds; creation admission separately limits usable counters.
    assert_eq!(player_guid(1, u64::MAX).low_value() as u64, u64::MAX);
}
