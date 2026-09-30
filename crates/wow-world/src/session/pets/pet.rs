//! Represented pet state at the Session boundary.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;
mod canonical_state;
mod lifecycle;
mod action_bar;
mod kill_fixture;
mod movement;
