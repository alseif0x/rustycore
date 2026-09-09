//! Unit values, visibility and health-revision state operations, part 3 of 3.
//!
//! The inherent `Unit` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl Unit {
    pub(super) fn set_f32_field(
        &mut self,
        bit: usize,
        value: f32,
        field: impl FnOnce(&mut UnitDataValues) -> &mut f32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_unit_data(bit);
        }
    }
    pub(super) fn mark_unit_data(&mut self, bit: usize) {
        self.unit_data_changes.set(UNIT_DATA_PARENT_BIT);
        self.unit_data_changes.set(bit);
    }
    pub(super) fn mark_unit_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) {
        self.unit_data_changes.set(parent_bit);
        self.unit_data_changes.set(first_element_bit + index);
    }
    pub(super) fn mark_unit_data_nested(&mut self, parent_bit: usize, bit: usize) {
        self.unit_data_changes.set(parent_bit);
        self.unit_data_changes.set(bit);
    }
}
