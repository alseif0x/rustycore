//! Original mailbox identity cases; no session fixture state is constructed.
use super::support::*;
use wow_world::session::mailbox::{LootRollCommandIdentityLikeCpp, LootRollVoteCommand};
use wow_loot::{OwnedLootAuthority, ROLL_VOTE_GREED_LIKE_CPP};

#[test]
fn loot_roll_vote_command_accepts_exact_enqueued_roll_identity_like_cpp() {
    let loot_object = represented_loot_object_guid_like_cpp(test_creature_guid(19_062));
    let authority = OwnedLootAuthority::new();
    let roll_identity = LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority, 7);
    let command = LootRollVoteCommand {
        voter_guid: ObjectGuid::create_player(1, 77),
        loot_obj: loot_object,
        loot_list_id: 0,
        roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        pass_on_group_loot: false,
        roll_identity: roll_identity.clone(),
    };

    assert!(command.targets_identity_like_cpp(&roll_identity));
}

#[test]
fn queued_loot_roll_vote_rejects_replacement_with_same_key_and_generation_like_cpp() {
    let loot_object = represented_loot_object_guid_like_cpp(test_creature_guid(19_063));
    let authority = OwnedLootAuthority::new();
    let stale_identity =
        LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority.clone(), 7);
    let replacement_identity =
        LootRollCommandIdentityLikeCpp::new_like_cpp(loot_object, 0, authority, 7);
    let stale_command = LootRollVoteCommand {
        voter_guid: ObjectGuid::create_player(1, 77),
        loot_obj: loot_object,
        loot_list_id: 0,
        roll_type: ROLL_VOTE_GREED_LIKE_CPP,
        pass_on_group_loot: false,
        roll_identity: stale_identity,
    };

    assert!(
        !stale_command.targets_identity_like_cpp(&replacement_identity),
        "a command queued for the destroyed C++ LootRoll* must not vote on its replacement"
    );
}
