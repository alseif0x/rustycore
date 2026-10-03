//! Player.cpp:25729-25759; Player.h:754-769; UpdateFields.h:1488.
use super::PlayerSkills;
use wow_data::forever_birth::BirthCatalog;

impl PlayerSkills {
    pub fn profession_lines(&self) -> &[i32; 2] {
        &self.professions
    }
    pub fn profession_slot(&self, skill: u32) -> Option<usize> {
        // Source std::find includes zero and preserves int32 identity bits.
        self.professions
            .iter()
            .position(|&line| line == skill as i32)
    }
    fn free_profession_slot(&self, birth: &BirthCatalog, skill: u32) -> Option<usize> {
        let line = birth.skill_line(skill)?;
        if line.parent_skill != 0 || line.category != 11 {
            return None;
        }
        // Despite its source comment, this does NOT first deduplicate skill ID.
        self.professions.iter().position(|&line| line == 0)
    }
    pub(super) fn assign_profession(&mut self, birth: &BirthCatalog, skill: u32) {
        if let Some(slot) = self.free_profession_slot(birth, skill) {
            self.professions[slot] = skill as i32;
        }
    }
}
