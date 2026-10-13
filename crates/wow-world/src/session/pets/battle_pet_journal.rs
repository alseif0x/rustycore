//! Represented battle-pet journal entries and their attributes.
//!
//! Moved out of the Session root under #607. The durable level grant and the
//! journal helpers moved to `wow_world_inventory::battle_pet_session` (#1363
//! slice 7b); the represented test bodies live in
//! `handlers/battle_pet_test_shims.rs`.

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/battle_pet_journal/f3_shims.rs"]
mod f3_shims;
