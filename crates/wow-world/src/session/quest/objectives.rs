//! Represented quest objective progress and credit.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

mod credit;
mod thresholds;
mod queue;
