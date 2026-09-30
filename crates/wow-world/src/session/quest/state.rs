//! Represented quest log state at the Session boundary.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

mod owner;
mod fixtures;
mod catalog;
mod valuation;
mod compatibility_requests;
mod sharing;
