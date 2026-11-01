use super::effective_instant_logout_permission_like_cpp;

#[test]
fn instant_logout_permission_account_grants_apply_to_players() {
    assert!(effective_instant_logout_permission_like_cpp(
        0,
        &[(1, true)]
    ));
    assert!(effective_instant_logout_permission_like_cpp(
        0,
        &[(194, true)]
    ));
}

#[test]
fn instant_logout_permission_defaults_to_granted_for_staff() {
    assert!(effective_instant_logout_permission_like_cpp(1, &[]));
}

#[test]
fn instant_logout_permission_direct_account_deny_revokes_default() {
    assert!(!effective_instant_logout_permission_like_cpp(
        1,
        &[(1, false)]
    ));
}

#[test]
fn instant_logout_permission_linked_role_deny_revokes_default() {
    assert!(!effective_instant_logout_permission_like_cpp(
        1,
        &[(194, false)]
    ));
}

#[test]
fn instant_logout_permission_defaults_to_denied_for_players() {
    assert!(!effective_instant_logout_permission_like_cpp(
        0,
        &[(2, true)]
    ));
}

#[test]
fn instant_logout_permission_account_deny_is_applied_last() {
    assert!(!effective_instant_logout_permission_like_cpp(
        1,
        &[(194, false), (1, true)]
    ));
    assert!(!effective_instant_logout_permission_like_cpp(
        1,
        &[(1, true), (194, false)]
    ));
}
