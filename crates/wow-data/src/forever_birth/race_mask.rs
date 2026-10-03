//! Target RaceMask.h::GetRaceBit/HasRace at 02245dcd.
//! New race IDs are NOT race-ID-minus-one. Both int32 words are preserved.
pub fn race_in_mask(mask: u64, race: u32) -> bool {
    let bit = match race {
        1..=11 | 22 | 24..=32 => race - 1,
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        70 => 15,
        52 => 16,
        84 => 17,
        85 => 18,
        91 => 19,
        86 => 20,
        95 => 32,
        96 => 33,
        _ => return false,
    };
    mask & (1u64 << bit) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_target_races_follow_source_bit_mapping_including_second_word() {
        let mut pairs: Vec<(u32, u32)> = (1..=11).map(|id| (id, id - 1)).collect();
        pairs.push((22, 21));
        pairs.extend((24..=32).map(|id| (id, id - 1)));
        pairs.extend([
            (34, 11),
            (35, 12),
            (36, 13),
            (37, 14),
            (70, 15),
            (52, 16),
            (84, 17),
            (85, 18),
            (91, 19),
            (86, 20),
            (95, 32),
            (96, 33),
        ]);
        for &(race, bit) in &pairs {
            assert!(race_in_mask(1 << bit, race));
            assert!(!race_in_mask(0, race));
            for &(other, _) in &pairs {
                assert_eq!(race_in_mask(1 << bit, other), other == race);
            }
        }
    }

    #[test]
    fn unrecognized_races_never_match_even_an_all_bits_mask() {
        for race in [0, 12, 21, 23, 33, 38, 51, 69, 71, 87, 92, 94, 97, u32::MAX] {
            assert!(!race_in_mask(u64::MAX, race));
        }
        assert!(race_in_mask(u64::MAX, 95));
        assert!(race_in_mask(u64::MAX, 96));
    }
}
