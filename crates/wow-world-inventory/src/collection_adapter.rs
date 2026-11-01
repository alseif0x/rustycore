// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Transmog criteria evidence retained by Session inventory.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedTransmogCriteriaEvent {
    LearnAnyTransmogInSlot {
        equipment_slot: u32,
        item_modified_appearance_id: u32,
    },
    CollectTransmogSetFromGroup {
        transmog_set_group_id: u32,
    },
}
