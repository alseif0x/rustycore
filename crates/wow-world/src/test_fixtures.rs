//! Fixture surface for the integration tests that moved out of the library.
//!
//! Issue #584 (B3): the handler suites are application-integration tests, so they live
//! in `tests/` and reach the module-level fixtures through this re-export instead of the
//! parent module's scope. These items already existed; only their visibility is widened,
//! and only for the `test-fixtures` build.

pub use crate::handlers::group::state::PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP;
pub use crate::handlers::group::state::current_group_guid_like_cpp;
pub use crate::handlers::group::state::first_connected_group_member_like_cpp;
pub use crate::handlers::group::state::group_persistence_command_like_cpp;
pub use crate::handlers::group::state::party_player_info_like_cpp;
pub use crate::handlers::group::state::send_group_new_leader_like_cpp;
pub use crate::handlers::group::state::send_party_update;
pub use crate::handlers::group::state::send_ready_check_events_like_cpp;
pub use crate::handlers::group::state::sender_can_start_ready_check_like_cpp;
