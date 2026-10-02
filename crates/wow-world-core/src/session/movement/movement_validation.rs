// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical attacker removal shared with Session combat adapters.

use wow_core::ObjectGuid;

impl crate::session::HubMut<'_> {
    pub fn remove_canonical_attacker_like_cpp(
        &mut self,
        victim: ObjectGuid,
        attacker: ObjectGuid,
    ) {
        if self
            .core
            .mutate_canonical_player_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().remove_attacker_like_cpp(attacker)
            })
            .is_some()
        {
            return;
        }
        let _ = self
            .core
            .mutate_canonical_creature_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().remove_attacker_like_cpp(attacker)
            });
    }
}
