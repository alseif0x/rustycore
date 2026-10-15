//! Represented talents and glyph slots, and their published state.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The talent-point refresh moved
//! to `wow_world_quest` (#1363 slice 12a); the catalog setters stay here.

use super::*;

impl WorldSession {
    pub fn set_talent_store(&mut self, store: Arc<TalentStore>) {
        self.catalogs.talent_store = Some(store);
    }
    pub fn set_num_talents_at_level_store(&mut self, store: Arc<NumTalentsAtLevelStore>) {
        self.catalogs.num_talents_at_level_store = Some(store);
        wow_world_quest::refresh_represented_talent_points_like_cpp(self);
    }
}
