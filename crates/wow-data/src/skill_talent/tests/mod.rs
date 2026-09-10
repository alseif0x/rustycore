//! Skill and talent store regressions.
//!
//! Separated from skill_talent.rs under #685.

use super::*;

fn skill_line(
    id: u32,
    category_id: i8,
    parent_skill_line_id: u32,
    parent_tier_index: i32,
) -> SkillLineEntry {
    SkillLineEntry {
        id,
        display_name: String::new(),
        alternate_verb: String::new(),
        description: String::new(),
        horde_display_name: String::new(),
        override_source_info_display_name: String::new(),
        category_id,
        spell_icon_file_id: 0,
        can_link: 0,
        parent_skill_line_id,
        parent_tier_index,
        flags: 0,
        spell_book_spell_id: 0,
    }
}

mod scenarios;
