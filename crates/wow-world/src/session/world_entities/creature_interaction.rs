//! Viewer-dependent NPC flags and represented creature interaction gates.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_viewer_dependent_creature_npc_flags_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        npc_flags: u64,
    ) -> u64 {
        if (npc_flags & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP) == 0 {
            return npc_flags;
        }

        match self.represented_can_see_spell_click_on_creature_like_cpp(creature_guid) {
            RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden => {
                npc_flags & !UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP
            }
            RepresentedCanSeeSpellClickOutcomeLikeCpp::Visible
            | RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented => npc_flags,
        }
    }
    pub(crate) fn pause_interacted_creature_movement_like_cpp(&mut self, guid: ObjectGuid) -> bool {
        self.mutate_world_creature(guid, |creature| {
            creature.pause_interaction_movement_like_cpp()
        })
        .unwrap_or(false)
    }
}
