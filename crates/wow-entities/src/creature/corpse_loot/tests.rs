use super::looted_corpse_decay_seconds as looted_corpse_decay_secs_like_cpp;

#[test]
fn looted_corpse_decay_uses_cpp_rate_and_ignore_flag() {
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, false, 0.5),
        60
    );
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, true, 0.5),
        120
    );
    assert_eq!(
        looted_corpse_decay_secs_like_cpp(false, 120, false, -1.0),
        0
    );
    assert_eq!(looted_corpse_decay_secs_like_cpp(true, 120, false, 0.5), 0);
}
