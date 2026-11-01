//! Aura slot updates and packets published to the client.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The zone-script and area-PvP
//! source queries moved to `wow_world_spell` (#1363 slice 12b3); the war-mode
//! zone-aura source to `wow_world_application` (#1363 final cleanup).

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura_publication/f3_shims.rs"]
mod f3_shims;
