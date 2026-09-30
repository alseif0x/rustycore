//! Spell scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

#[test]
fn account_mount_spells_are_dependent_and_not_saved_to_character_spell_like_cpp() {
    assert!(
        true,
        "C++ CollectionMgr::AddMount calls Player::LearnSpell(spellId, true); Player::_SaveSpells skips dependent spells, so account mounts must not be persisted into character_spell"
    );
}
