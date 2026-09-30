use super::*;

#[test]
fn has_item_appearance_reports_permanent_before_temporary_like_cpp() {
    let (mut session, _, _) = make_session();

    assert_eq!(session.has_item_appearance_like_cpp(65), (false, false));

    session
        .represented_temporary_item_appearances_like_cpp
        .insert(65, HashSet::from([ObjectGuid::create_item(1, 900)]));
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, true));

    session.represented_item_appearances_like_cpp.insert(65);
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, false));
}
#[test]
fn account_transmog_update_is_not_sent_while_opcode_is_unresolved_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session
        .represented_favorite_item_appearances_like_cpp
        .insert(65, FavoriteAppearanceStateLikeCpp::Unchanged);

    session.send_favorite_appearances_like_cpp();

    assert_eq!(
        send_rx.try_recv(),
        Err(flume::TryRecvError::Empty),
        "do not send SMSG_ACCOUNT_TRANSMOG_UPDATE while legacy C++ keeps it at NULL_OPCODE/0xBADD"
    );
}
