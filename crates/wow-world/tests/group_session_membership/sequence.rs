//! Original Group Session application cases.

use super::*;

#[test]
fn reset_group_update_sequence_does_not_reset_same_group_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(set_owned_player_group_like_cpp(&mut session, Some((group_guid, 0))));

    assert!(session.group_reset_update_sequence_for_test());
    assert_eq!(
        session.group_next_update_sequence_for_test(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(1)
    );
    assert_eq!(
        session.group_next_update_sequence_for_test(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(2)
    );

    assert!(!session.group_reset_update_sequence_for_test());
    assert_eq!(
        session.group_next_update_sequence_for_test(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(3)
    );
}


#[test]
fn reset_group_update_sequence_resets_when_group_changes_like_cpp() {
    let (mut session, _, _) = make_session();
    let first_leader = ObjectGuid::create_player(1, 42);
    let second_leader = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let first_group = GroupInfo::new(first_leader);
    let first_group_guid = first_group.group_guid;
    let second_group = GroupInfo::new(second_leader);
    let second_group_guid = second_group.group_guid;
    group_registry.register_group_like_cpp(first_group_guid, first_group);
    group_registry.register_group_like_cpp(second_group_guid, second_group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    set_group_guid_for_test_like_cpp(&mut session, Some(first_group_guid));
    assert!(session.group_reset_update_sequence_for_test());
    assert_eq!(
        session.group_next_update_sequence_for_test(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(1)
    );
    assert_eq!(
        session.group_next_update_sequence_for_test(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(2)
    );

    set_group_guid_for_test_like_cpp(&mut session, Some(second_group_guid));
    assert!(session.group_reset_update_sequence_for_test());
    assert_eq!(
        session.group_next_update_sequence_for_test(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(1)
    );
}


#[test]
fn reset_group_update_sequence_without_group_is_noop_like_cpp() {
    let (mut session, _, _) = make_session();

    assert!(!session.group_reset_update_sequence_for_test());
    assert_eq!(
        session.group_next_update_sequence_for_test(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
        ),
        Some(0)
    );
    assert_eq!(session.group_next_update_sequence_for_test(99), None);
}

