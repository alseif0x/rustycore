//! Original Group Session application cases.

use super::*;

#[tokio::test]
async fn failed_party_invite_social_lookup_retains_the_existing_fail_open_result() {
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let port = PartyInviteSocialPortLikeCpp::new(
        SocialPartyInviteLookupOutcomeLikeCpp::Failed {
            reason: "database unavailable".to_owned(),
        },
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(false),
    );

    assert!(
        !group_target_ignores_inviter_for_test(
            Some(port.clone()),
            target,
            inviter,
            1,
        )
        .await
    );
    assert_eq!(port.calls(), vec!["ignore:77:42:1"]);
}

