use super::{calculate_pct_i32_like_cpp, calculate_pct_u64_like_cpp, self_resurrection_values_like_cpp};

#[test]
fn flat_self_resurrection_preserves_unsigned_source_conversions() {
    assert_eq!(
        self_resurrection_values_like_cpp(-35, -3, u64::MAX, i32::MAX),
        Ok((35, u32::MAX - 2)),
    );
}

#[test]
fn flat_int_min_is_unavailable_before_native_resurrection() {
    assert_eq!(
        self_resurrection_values_like_cpp(i32::MIN, 77, 100, 100),
        Err("Self-resurrection flat health is outside Source's defined domain"),
    );
}

#[test]
fn percentage_uses_source_f32_rounding_truncation_and_u32_narrowing() {
    assert_eq!(calculate_pct_u64_like_cpp(9_999, 33), Ok(3_299));
    assert_eq!(calculate_pct_i32_like_cpp(101, 33), Ok(33));

    let full_width_percent = self_resurrection_values_like_cpp(50, 0, u64::MAX, 101)
        .expect("50 percent of the full u64 health value is in Source's cast domain");
    let source_health = ((u64::MAX as f32) * 50.0_f32 / 100.0_f32) as u64;
    assert_eq!(full_width_percent, (source_health as u32, 50));
    assert_eq!(full_width_percent.0, 0, "u32 narrowing happens after u64 calculation");
}

#[test]
fn percentage_rejects_values_outside_source_float_cast_domains() {
    assert!(calculate_pct_u64_like_cpp(u64::MAX, 100).is_err());
    assert!(calculate_pct_i32_like_cpp(i32::MAX, 100).is_err());
}

#[test]
fn percentage_mana_uses_only_a_positive_maximum() {
    assert_eq!(self_resurrection_values_like_cpp(25, 999, 400, 200), Ok((100, 50)));
    assert_eq!(self_resurrection_values_like_cpp(25, 999, 400, 0), Ok((100, 0)));
    assert_eq!(self_resurrection_values_like_cpp(25, 999, 400, -1), Ok((100, 0)));
}
