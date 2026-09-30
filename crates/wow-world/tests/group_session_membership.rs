//! Group Session membership, mailbox and canonical publication contracts.

use std::sync::{Arc, Mutex};
use wow_constants::ServerOpcodes;
use wow_core::{ObjectGuid, Position};
use wow_entities::Player;
use wow_packet::WorldPacket;
use wow_persistence::SocialPartyInviteLookupOutcomeLikeCpp;
use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};
use wow_world::SharedCanonicalMapManager;
use wow_world::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
use wow_world::session::mailbox::{
    ApplyGroupRemovalLikeCppCommand, ApplyGroupSubgroupLikeCppCommand,
    SendPartyUpdateLikeCppCommand, SessionCommand,
};
use wow_world::session::{SessionState, WorldSession};
use wow_world::test_fixtures::{
    PartyInviteSocialPortLikeCpp, group_guid_for_test_like_cpp,
    group_target_ignores_inviter_for_test,
    insert_character_fixture_player_into_canonical_map as insert_session_player_into_canonical_map_like_cpp,
    install_realm_send_channel_for_test, mutate_canonical_player_for_test,
    process_represented_session_commands_like_cpp, register_in_player_registry_production_for_test,
    represented_subgroup_like_cpp, resolved_group_guid_like_cpp, set_group_guid_for_test_like_cpp,
    set_loaded_player_name_like_cpp, set_owned_player_group_like_cpp,
    sync_player_registry_state_production_for_test,
};

// Same private Player flag from the original Session test root.
const PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP: u32 = 0x0000_0001;
const PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP: u32 =
    WorldSession::CHARACTER_PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_FOR_TEST;

#[path = "group_session_membership/support.rs"]
mod support;
use support::*;
#[path = "group_session_membership/packets.rs"]
mod packets;
use packets::*;
#[path = "group_session_membership/canonical_membership.rs"]
mod canonical_membership;
#[path = "group_session_membership/fixture_rail.rs"]
mod fixture_rail;
#[path = "group_session_membership/mailbox.rs"]
mod mailbox;
#[path = "group_session_membership/sequence.rs"]
mod sequence;
#[path = "group_session_membership/social_lookup.rs"]
mod social_lookup;
