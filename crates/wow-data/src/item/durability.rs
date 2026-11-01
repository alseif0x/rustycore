//! ObjectMgr.cpp:2978–3063 effective ItemTemplate durability, after DB2 load.

pub fn fill_max_durability_like_cpp(
    item_class: u32,
    item_subclass: u32,
    inventory_type: u32,
    quality: u32,
    item_level: u32,
) -> u32 {
    if item_class != 4 && item_class != 2 {
        return 0;
    }
    const QUALITY: [f32; 8] = [0.92, 0.92, 0.92, 1.11, 1.32, 1.61, 0.0, 0.0];
    const ARMOR: [f32; 21] = [
        0.0, 0.60, 0.0, 0.60, 0.0, 1.0, 0.33, 0.72, 0.48, 0.33, 0.33, 0.0, 0.0, 0.0, 0.72, 0.0,
        0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    const WEAPON: [f32; 21] = [
        0.91, 1.0, 1.0, 1.0, 0.91, 1.0, 1.0, 0.91, 1.0, 1.0, 1.0, 0.0, 0.0, 0.66, 0.0, 0.66, 0.0,
        0.0, 1.0, 0.66, 0.66,
    ];
    let Some(quality) = QUALITY.get(quality as usize).copied() else {
        return 0;
    };
    let level_penalty = if item_level <= 28 {
        0.966_f32 - (28 - item_level) as f32 / 54.0_f32
    } else {
        1.0_f32
    };
    let (base, multiplier) = if item_class == 4 {
        (25.0_f32, ARMOR.get(inventory_type as usize))
    } else {
        (18.0_f32, WEAPON.get(item_subclass as usize))
    };
    let Some(multiplier) = multiplier.copied() else {
        return 0;
    };
    5 * (base * quality * multiplier * level_penalty).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpp_effective_durability_matches_low_level_chest_and_tables() {
        // Item2121: the reference Item CREATE has current40 / effective max60.
        assert_eq!(fill_max_durability_like_cpp(4, 1, 5, 1, 5), 60);
        assert_eq!(fill_max_durability_like_cpp(4, 1, 20, 1, 5), 60);
        assert_eq!(fill_max_durability_like_cpp(4, 1, 5, 2, 28), 110);
        assert_eq!(fill_max_durability_like_cpp(4, 1, 5, 2, 29), 115);
        assert_eq!(fill_max_durability_like_cpp(4, 1, 1, 3, 80), 85);
        assert_eq!(fill_max_durability_like_cpp(2, 0, 13, 1, 80), 75);
        assert_eq!(fill_max_durability_like_cpp(2, 20, 17, 1, 80), 55);
    }

    #[test]
    fn cpp_durability_guards_and_nondurable_table_entries_return_zero() {
        for args in [
            (0, 0, 5, 1),
            (4, 0, 21, 1),
            (4, 0, 5, 8),
            (2, 21, 13, 1),
            (4, 0, 2, 1),
            (4, 0, 5, 6),
            (2, 16, 25, 1),
        ] {
            assert_eq!(
                fill_max_durability_like_cpp(args.0, args.1, args.2, args.3, 80),
                0
            );
        }
    }
}
