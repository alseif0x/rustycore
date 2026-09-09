//! Represented persistence responsibility, separated from the
//! Session root under #609. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod commit;
mod load;
mod load_authority;
mod plans;
mod save;
