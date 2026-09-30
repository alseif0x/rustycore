//! Represented battle-pet state at the Session boundary.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

mod account_catalog;
mod mutations;
mod experience_criteria;
mod summon_queries;
