// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player identity: GUID, name, race/class, appearance and account.

use super::*;

impl Player {
    pub const fn guid(&self) -> ObjectGuid {
        self.unit.world().object().guid()
    }

    pub fn set_race_class_gender(&mut self, race: u8, class_id: u8, gender: Gender) {
        self.unit.set_race(race);
        self.unit.set_class(class_id);
        self.unit.set_player_class(class_id);
        self.unit.set_gender(gender);
        self.set_native_gender(gender);
    }

    pub fn set_native_gender(&mut self, gender: Gender) {
        self.set_player_u8(PLAYER_DATA_NATIVE_SEX_BIT, gender as u8, |data| {
            &mut data.native_sex
        });
    }

    pub fn replace_all_player_flags(&mut self, flags: u32) {
        self.set_player_u32(PLAYER_DATA_FLAGS_BIT, flags, |data| &mut data.player_flags);
    }

    pub fn replace_all_player_flags_ex(&mut self, flags: u32) {
        self.set_player_u32(PLAYER_DATA_FLAGS_EX_BIT, flags, |data| {
            &mut data.player_flags_ex
        });
    }

    pub fn set_loot_guid(&mut self, guid: ObjectGuid) {
        self.set_player_guid(PLAYER_DATA_LOOT_TARGET_GUID_BIT, guid, |data| {
            &mut data.loot_target_guid
        });
    }

    pub fn set_chosen_title_like_cpp(&mut self, title_id: i32) {
        self.set_player_i32(PLAYER_DATA_PLAYER_TITLE_BIT, title_id, |data| {
            &mut data.player_title
        });
    }

    pub fn learn_title_like_cpp(&mut self, title_id: u32) {
        self.gameplay_state.known_title_ids.insert(title_id);
    }

    pub fn replace_known_titles_like_cpp(&mut self, title_ids: BTreeSet<u32>) {
        self.gameplay_state.known_title_ids = title_ids;
    }

    pub fn has_title_like_cpp(&self, title_id: u32) -> bool {
        self.gameplay_state.known_title_ids.contains(&title_id)
    }

    fn set_player_guid(
        &mut self,
        bit: usize,
        value: ObjectGuid,
        field: impl FnOnce(&mut PlayerDataValues) -> &mut ObjectGuid,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_player_data(bit);
        }
    }

    pub(crate) fn set_active_guid(
        &mut self,
        bit: usize,
        value: ObjectGuid,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut ObjectGuid,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data(bit);
        }
    }
}
