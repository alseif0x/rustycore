// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Cinematic adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

impl WorldSession {
    pub(crate) fn opening_cinematic_like_cpp(&mut self) -> Option<u32> {
        if crate::session::hub_ref(self).resolved_player_xp_like_cpp()? != 0 {
            return None;
        }

        let class_store = self.catalogs.chr.classes_store.as_ref()?;
        let class_entry = class_store.get(u32::from(
            crate::session::hub_ref(self).player_class_like_cpp(),
        ))?;
        let cinematic_id = if class_entry.cinematic_sequence_id != 0 {
            u32::from(class_entry.cinematic_sequence_id)
        } else {
            let race_store = self.catalogs.chr.races_store.as_ref()?;
            race_store
                .get(u32::from(
                    crate::session::hub_ref(self).player_race_like_cpp(),
                ))
                .map(|race_entry| race_entry.cinematic_sequence_id as u32)?
        };

        self.send_represented_cinematic_start_like_cpp(cinematic_id);
        Some(cinematic_id)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/cinematic_adapter/f3_shims.rs"]
mod f3_shims;
