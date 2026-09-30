//! Represented quest completion, turn-in and reward choice.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

mod event_completion;
mod currency;
mod authority;
mod spells;
mod valuation;
mod admission;
mod publication;
