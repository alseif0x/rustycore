//! Represented spell acquisition runtime wiring and its fixtures.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Install the process-wide audited static authority consumed by the
    /// immutable acquisition planner. Absence remains fail-closed.
    pub fn set_spell_acquisition_static_authority_like_cpp(
        &mut self,
        safe_cast_spell_ids: impl IntoIterator<Item = u32>,
        valid_craft_spell_ids: impl IntoIterator<Item = u32>,
    ) {
        self.spell_state.set_spell_acquisition_cast_authority_like_cpp(Arc::new(
            crate::spell_acquisition::SpellAcquisitionCastAuthorityLikeCpp::from_audited_rows_like_cpp(
                safe_cast_spell_ids,
                std::iter::empty(),
            ),
        ));
        self.spell_state.set_spell_acquisition_craft_authority_like_cpp(Arc::new(
            crate::spell_acquisition::SpellAcquisitionCraftValidityAuthorityLikeCpp::from_audited_rows_like_cpp(
                valid_craft_spell_ids,
                std::iter::empty(),
            ),
        ));
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/acquisition/f3_shims.rs"]
mod f3_shims;
