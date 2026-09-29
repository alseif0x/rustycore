use crate::session::WorldSession;

pub fn set_reputation_standing_for_test(
    session: &mut WorldSession,
    reputation_list_id: u32,
    standing: i32,
) {
    session
        .mutate_reputation_mgr_like_cpp(|manager| {
            manager
                .get_state_mut(reputation_list_id)
                .expect("reputation state")
                .standing = standing;
        })
        .expect("reputation state");
}

pub fn reputation_standing_for_test(
    session: &WorldSession,
    reputation_list_id: u32,
) -> Option<i32> {
    session
        .with_reputation_mgr_like_cpp(|manager| {
            manager
                .get_state(reputation_list_id)
                .map(|state| state.standing)
        })
        .flatten()
}
