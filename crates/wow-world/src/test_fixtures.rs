//! Fixture surface prepared for the crate's integration tests.
//!
//! Issue #584 (B3): these re-exports and wrappers let external integration-test targets
//! reach crate-level fixtures. The suite migration is still pending; this module is
//! available only in the `test-fixtures` build.

pub use crate::handlers::group::state::PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP;
pub use crate::handlers::group::state::current_group_guid_like_cpp;
pub use crate::handlers::group::state::first_connected_group_member_like_cpp;
pub use crate::handlers::group::state::group_persistence_command_like_cpp;
pub use crate::handlers::group::state::party_player_info_like_cpp;
pub use crate::handlers::group::state::send_group_new_leader_like_cpp;
pub use crate::handlers::group::state::send_party_update;
pub use crate::handlers::group::state::send_ready_check_events_like_cpp;
pub use crate::handlers::group::state::sender_can_start_ready_check_like_cpp;
pub use crate::handlers::group::test_support::PartyInviteSocialPortLikeCpp;

pub fn with_canonical_player_at_mut_like_cpp<R>(
    manager: &crate::session::SharedCanonicalMapManager,
    guid: wow_core::ObjectGuid,
    map_id: u32,
    instance_id: u32,
    mutate: impl FnOnce(&mut wow_entities::Player) -> R,
) -> Option<R> {
    crate::canonical_player_access::with_canonical_player_at_mut_like_cpp(
        manager,
        guid,
        map_id,
        instance_id,
        mutate,
    )
}

pub fn set_loaded_player_identity_like_cpp(
    session: &mut crate::session::WorldSession,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.set_loaded_player_identity_like_cpp(map_id, race, class, level, gender);
}

pub const PLAYER_FLAGS_GHOST_LIKE_CPP: u32 = crate::session::PLAYER_FLAGS_GHOST_LIKE_CPP;
pub const PLAYER_FLAGS_AFK_LIKE_CPP: u32 = crate::session::PLAYER_FLAGS_AFK_LIKE_CPP;
pub const PLAYER_FLAGS_DND_LIKE_CPP: u32 = crate::session::PLAYER_FLAGS_DND_LIKE_CPP;
