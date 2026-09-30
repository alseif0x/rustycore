//! Existing Unit visibility state setters and borrowed kernel delegates.

use super::*;

impl Unit {
    pub const fn visibility_detection_like_cpp(&self) -> &UnitVisibilityDetectionStateLikeCpp {
        &self.visibility_detection
    }
    pub fn replace_visibility_detection_like_cpp(
        &mut self,
        state: UnitVisibilityDetectionStateLikeCpp,
    ) {
        self.visibility_detection = state;
    }
    pub fn set_never_visible_for_seer_like_cpp(&mut self, never_visible: bool) {
        self.visibility_detection.never_visible_for_seer = never_visible;
    }
    pub fn set_seer_can_never_see_target_like_cpp(&mut self, can_never_see: bool) {
        self.visibility_detection.seer_can_never_see_target = can_never_see;
    }
    pub fn set_always_visible_for_seer_like_cpp(&mut self, always_visible: bool) {
        self.visibility_detection.always_visible_for_seer = always_visible;
    }
    pub fn set_seer_can_always_see_target_like_cpp(&mut self, can_always_see: bool) {
        self.visibility_detection.seer_can_always_see_target = can_always_see;
    }
    pub fn set_target_owner_group_visible_for_seer_like_cpp(&mut self, visible: bool) {
        self.visibility_detection
            .target_owner_group_visible_for_seer = visible;
    }
    pub fn set_seer_can_always_see_target_guid_like_cpp(&mut self, guid: ObjectGuid) {
        self.visibility_detection.seer_can_always_see_target_guid = guid;
    }
    pub fn set_always_detectable_for_seer_like_cpp(&mut self, always_detectable: bool) {
        self.visibility_detection.always_detectable_for_seer = always_detectable;
    }
    pub fn set_invisible_due_to_despawn_like_cpp(&mut self, invisible_due_to_despawn: bool) {
        self.visibility_detection.invisible_due_to_despawn = invisible_due_to_despawn;
    }
    pub fn set_private_object_owner_like_cpp(&mut self, owner: ObjectGuid) {
        self.visibility_detection.private_object_owner = owner;
    }
    pub const fn private_object_owner_like_cpp(&self) -> ObjectGuid {
        self.visibility_detection.private_object_owner
    }
    pub fn set_seer_private_object_owner_like_cpp(&mut self, owner: ObjectGuid) {
        self.visibility_detection.seer_private_object_owner = owner;
    }
    pub fn set_seer_group_visible_for_private_owner_like_cpp(&mut self, visible: bool) {
        self.visibility_detection
            .seer_group_visible_for_private_owner = visible;
    }
    pub fn set_object_id_visibility_conditions_met_like_cpp(&mut self, met: bool) {
        self.visibility_detection
            .object_id_visibility_conditions_met = met;
    }
    pub fn set_server_side_gm_visibility_like_cpp(&mut self, visibility: u32) {
        self.visibility_detection.server_side_visibility_gm = visibility;
    }
    pub fn set_server_side_gm_visibility_detect_like_cpp(&mut self, detect: u32) {
        self.visibility_detection.server_side_visibility_detect_gm = detect;
    }
    pub fn set_server_side_ghost_visibility_like_cpp(&mut self, visibility: u32) {
        self.visibility_detection.server_side_visibility_ghost =
            visibility & (GHOST_VISIBILITY_ALIVE_LIKE_CPP | 0x2);
    }
    pub fn set_server_side_ghost_visibility_detect_like_cpp(&mut self, detect: u32) {
        self.visibility_detection
            .server_side_visibility_detect_ghost = detect & (GHOST_VISIBILITY_ALIVE_LIKE_CPP | 0x2);
    }
    pub fn set_ghost_visible_to_seer_by_group_like_cpp(&mut self, visible: bool) {
        self.visibility_detection.ghost_visible_to_seer_by_group = visible;
    }
    pub fn set_invisibility_like_cpp(&mut self, aura_type: usize, value: i32) {
        if aura_type >= MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            return;
        }
        let flag = 1_u64 << aura_type;
        self.visibility_detection.invisibility[aura_type] = value;
        if value > 0 {
            self.visibility_detection.invisibility_flags |= flag;
        } else {
            self.visibility_detection.invisibility_flags &= !flag;
        }
    }
    pub fn set_invisibility_detect_like_cpp(&mut self, aura_type: usize, value: i32) {
        if aura_type >= MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            return;
        }
        let flag = 1_u64 << aura_type;
        self.visibility_detection.invisibility_detect[aura_type] = value;
        if value > 0 {
            self.visibility_detection.invisibility_detect_flags |= flag;
        } else {
            self.visibility_detection.invisibility_detect_flags &= !flag;
        }
    }
    pub fn set_stealth_like_cpp(&mut self, aura_type: usize, value: i32) {
        if aura_type >= MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            return;
        }
        let flag = 1_u64 << aura_type;
        self.visibility_detection.stealth[aura_type] = value;
        if value > 0 {
            self.visibility_detection.stealth_flags |= flag;
        } else {
            self.visibility_detection.stealth_flags &= !flag;
        }
    }
    pub fn set_stealth_detect_like_cpp(&mut self, aura_type: usize, value: i32) {
        if aura_type < MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
            self.visibility_detection.stealth_detect[aura_type] = value;
        }
    }
    pub const fn has_stealth_aura_like_cpp(&self) -> bool {
        self.visibility_detection.stealth_flags != 0
    }
    pub fn can_detect_invisibility_of_like_cpp(&self, target: &Self) -> bool {
        super::visibility::can_detect_invisibility_of(self, target)
    }
    pub fn can_detect_stealth_of_like_cpp(
        &self,
        target: &Self,
        seer_is_player: bool,
        check_alert: bool,
    ) -> bool {
        super::visibility::can_detect_stealth_of(self, target, seer_is_player, check_alert)
    }
    pub fn can_see_or_detect_unit_like_cpp(
        &self,
        target: &Self,
        implicit_detect: bool,
        seer_is_player: bool,
        check_alert: bool,
    ) -> bool {
        super::visibility::can_see_or_detect_unit(
            self,
            target,
            implicit_detect,
            seer_is_player,
            check_alert,
        )
    }
}
