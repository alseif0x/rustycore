//! Represented spell cooldown and recovery state.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The login cooldown wrappers
//! went to the test shim file with #1363 slice 8e (their only production
//! caller, the login spell-history load, moved to `wow-world-character`).

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/cooldown/f3_shims.rs"]
mod f3_shims;
