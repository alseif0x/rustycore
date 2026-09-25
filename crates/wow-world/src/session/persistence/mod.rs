//! Represented persistence responsibility, separated from the
//! Session root under #609. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod commit;
mod load;
mod load_authority;
mod plans;
#[cfg(test)]
pub(crate) mod test_fixtures;
pub(crate) use plans::player_homebind_update_request_like_cpp;
mod save;
