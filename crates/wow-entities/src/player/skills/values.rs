//! 02245dcd UpdateFields.h:679-692: SkillInfo's 2101 field bits and two profession bits.
use super::{PLAYER_MAX_SKILLS, PlayerSkills, SkillFields};

const SKILL_INFO_FIELD_WORDS: usize = 66;
const SKILL_INFO_PARENT_BIT: usize = 0;
const LINE_FIRST_BIT: usize = 1;
const STEP_FIRST_BIT: usize = 301;
const RANK_FIRST_BIT: usize = 601;
const STARTING_RANK_FIRST_BIT: usize = 901;
const MAXIMUM_FIRST_BIT: usize = 1201;
const TEMPORARY_BONUS_FIRST_BIT: usize = 1501;
const PERMANENT_BONUS_FIRST_BIT: usize = 1801;

fn set_bit(words: &mut [u32; SKILL_INFO_FIELD_WORDS], bit: usize) {
    words[bit / 32] |= 1 << (bit % 32);
}

fn write_skill_field<T: PartialEq>(
    changes: &mut [u32; SKILL_INFO_FIELD_WORDS],
    current: &mut T,
    value: T,
    bit: usize,
    mut request: impl FnMut(),
) {
    set_bit(changes, SKILL_INFO_PARENT_BIT);
    set_bit(changes, bit);
    if *current != value {
        *current = value;
        request();
    }
}

impl PlayerSkills {
    pub fn field_changes(&self) -> &[u32; 66] {
        &self.field_changes
    }

    pub fn profession_changes(&self) -> u8 {
        self.profession_changes
    }

    /// Clear only disposable publication history; persistence status remains independent.
    pub(in crate::player) fn clear_changes(&mut self) {
        self.field_changes = [0; SKILL_INFO_FIELD_WORDS];
        self.profession_changes = 0;
    }

    /// Freeze the owned field facts and raw masks for a later publication adapter.
    pub fn values_snapshot(&self) -> SkillValuesSnapshot {
        SkillValuesSnapshot {
            fields: self.fields.clone(),
            field_changes: self.field_changes,
            professions: self.professions,
            profession_changes: self.profession_changes,
        }
    }

    pub(super) fn set_line(&mut self, slot: usize, value: u16, request: impl FnMut()) {
        write_skill_field(
            &mut self.field_changes,
            &mut self.fields[slot].line,
            value,
            LINE_FIRST_BIT + slot,
            request,
        );
    }

    pub(super) fn set_step(&mut self, slot: usize, value: u16, request: impl FnMut()) {
        write_skill_field(
            &mut self.field_changes,
            &mut self.fields[slot].step,
            value,
            STEP_FIRST_BIT + slot,
            request,
        );
    }

    pub(super) fn set_rank(&mut self, slot: usize, value: u16, request: impl FnMut()) {
        write_skill_field(
            &mut self.field_changes,
            &mut self.fields[slot].rank,
            value,
            RANK_FIRST_BIT + slot,
            request,
        );
    }

    pub(super) fn set_starting_rank(&mut self, slot: usize, value: u16, request: impl FnMut()) {
        write_skill_field(
            &mut self.field_changes,
            &mut self.fields[slot].starting_rank,
            value,
            STARTING_RANK_FIRST_BIT + slot,
            request,
        );
    }

    pub(super) fn set_maximum(&mut self, slot: usize, value: u16, request: impl FnMut()) {
        write_skill_field(
            &mut self.field_changes,
            &mut self.fields[slot].maximum,
            value,
            MAXIMUM_FIRST_BIT + slot,
            request,
        );
    }

    pub(super) fn set_temporary_bonus(&mut self, slot: usize, value: i16, request: impl FnMut()) {
        write_skill_field(
            &mut self.field_changes,
            &mut self.fields[slot].temporary_bonus,
            value,
            TEMPORARY_BONUS_FIRST_BIT + slot,
            request,
        );
    }

    pub(super) fn set_permanent_bonus(&mut self, slot: usize, value: u16, request: impl FnMut()) {
        write_skill_field(
            &mut self.field_changes,
            &mut self.fields[slot].permanent_bonus,
            value,
            PERMANENT_BONUS_FIRST_BIT + slot,
            request,
        );
    }

    pub(super) fn set_profession_line(
        &mut self,
        slot: usize,
        value: i32,
        mut request: impl FnMut(),
    ) {
        self.profession_changes |= 1 << slot;
        if self.professions[slot] != value {
            self.professions[slot] = value;
            request();
        }
    }
}

/// Immutable copy of the native skill values and their raw dirty masks.
pub struct SkillValuesSnapshot {
    fields: Box<[SkillFields; PLAYER_MAX_SKILLS]>,
    field_changes: [u32; SKILL_INFO_FIELD_WORDS],
    professions: [i32; 2],
    profession_changes: u8,
}

impl SkillValuesSnapshot {
    pub fn fields(&self) -> &[SkillFields; PLAYER_MAX_SKILLS] {
        &self.fields
    }

    pub fn field_changes(&self) -> &[u32; 66] {
        &self.field_changes
    }

    pub fn profession_lines(&self) -> &[i32; 2] {
        &self.professions
    }

    pub fn profession_changes(&self) -> u8 {
        self.profession_changes
    }

    pub(crate) fn clear_changes(&mut self) {
        self.field_changes = [0; SKILL_INFO_FIELD_WORDS];
        self.profession_changes = 0;
    }
}
