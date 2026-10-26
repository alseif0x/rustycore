use super::selected_refreshed_duration;

const PANDEMIC: u32 = 0x0010_0000;

#[test]
fn timer_refresh_uses_base_duration_without_pandemic() {
    assert_eq!(selected_refreshed_duration(600_000, 12_345, 0), Ok(600_000));
    assert_eq!(selected_refreshed_duration(1_000, -1, 0), Ok(1_000));
}

#[test]
fn pandemic_uses_source_float_rounding_and_caps_old_remaining() {
    assert_eq!(
        selected_refreshed_duration(16_777_219, 16_777_219, PANDEMIC),
        Ok(21_810_385)
    );
    assert_eq!(selected_refreshed_duration(1_000, 900, PANDEMIC), Ok(1_300));
    assert_eq!(selected_refreshed_duration(1_000, 200, PANDEMIC), Ok(1_200));
    assert_eq!(
        selected_refreshed_duration(1_000, 2_000, PANDEMIC),
        Ok(2_000)
    );
    assert_eq!(selected_refreshed_duration(1_000, -1, PANDEMIC), Ok(999));
    assert_eq!(selected_refreshed_duration(1_000, -100, PANDEMIC), Ok(900));
}

#[test]
fn pandemic_overflow_is_rejected_before_the_caller_mutates_state() {
    assert_eq!(
        selected_refreshed_duration(i32::MAX, i32::MAX, PANDEMIC),
        Err("Selected Aura pandemic duration exceeds the supported range")
    );
}

#[test]
fn permanent_selected_aura_stays_permanent() {
    assert_eq!(selected_refreshed_duration(-1, -1, 0), Ok(-1));
    assert_eq!(selected_refreshed_duration(-1, 10, PANDEMIC), Ok(10));
}
