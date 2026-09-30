//! Immutable target observations for the represented visibility rules.
//!
//! The caller captures the target before releasing its owner guard, then reads
//! the seer later. This preserves that observation window without copying a
//! Unit, actor, motion master, or detection state belonging to the seer.
//! C++: Object.cpp:1488-1742, Unit.cpp:8248/8268, PhaseShift.cpp:108,
//! SmoothPhasing.cpp:45 (a5f8da2e). Existing represented gaps are unchanged;
//! this does not add corpse/viewpoint/vehicle or controller resolution.

use super::*;
use crate::{AppliedAuraRef, PhaseShift, SmoothPhasingLikeCpp};
use wow_core::Position;

/// Target values from one observation, independent of subsequent target writes.
/// This is not a proof of current actor identity or continued map residence.
#[derive(Debug)]
pub struct UnitVisibilityTargetFacts {
    scalar: TargetScalars,
    phase_shift: PhaseShift,
    smooth_phasing: Option<SmoothPhasingLikeCpp>,
    invisibility: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    stealth: [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    stalked_casters: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, Copy)]
struct TargetScalars {
    guid: ObjectGuid,
    is_in_world: bool,
    has_current_map: bool,
    map_id: u32,
    instance_id: u32,
    position: Position,
    never_visible_for_seer: bool,
    always_visible_for_seer: bool,
    charmer_or_owner_guid: Option<ObjectGuid>,
    target_owner_group_visible_for_seer: bool,
    private_object_owner: ObjectGuid,
    object_id_visibility_conditions_met: bool,
    server_side_visibility_gm: u32,
    server_side_visibility_ghost: u32,
    ghost_visible_to_seer_by_group: bool,
    invisible_due_to_despawn: bool,
    always_detectable_for_seer: bool,
    invisibility_flags: u64,
    stealth_flags: u64,
}

enum StalkedCasters<'a> {
    Applied(&'a [AppliedAuraRef]),
    Captured(&'a [ObjectGuid]),
}

impl StalkedCasters<'_> {
    fn contains(&self, guid: ObjectGuid) -> bool {
        match self {
            Self::Applied(auras) => auras.iter().any(|aura| aura.caster_guid == guid),
            Self::Captured(casters) => casters.contains(&guid),
        }
    }

    fn capture(&self) -> Vec<ObjectGuid> {
        match self {
            Self::Applied(auras) => auras.iter().map(|aura| aura.caster_guid).collect(),
            Self::Captured(casters) => casters.to_vec(),
        }
    }
}

/// Both legacy borrowed targets and owned observations use this same kernel.
struct TargetView<'a> {
    scalar: TargetScalars,
    phase_shift: &'a PhaseShift,
    smooth_phasing: Option<&'a SmoothPhasingLikeCpp>,
    invisibility: &'a [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    stealth: &'a [i32; MAX_VISIBILITY_AURA_TYPES_LIKE_CPP],
    stalked_casters: StalkedCasters<'a>,
}

impl<'a> TargetView<'a> {
    fn borrowed(target: &'a Unit) -> Self {
        let world = &target.world;
        let detection = &target.visibility_detection;
        Self {
            scalar: TargetScalars {
                guid: world.object().guid(),
                is_in_world: world.object().is_in_world(),
                has_current_map: world.has_current_map(),
                map_id: world.map_id(),
                instance_id: world.instance_id(),
                position: world.position(),
                never_visible_for_seer: detection.never_visible_for_seer,
                always_visible_for_seer: detection.always_visible_for_seer,
                charmer_or_owner_guid: target.subsystems.control.charmer_or_owner_guid(),
                target_owner_group_visible_for_seer: detection.target_owner_group_visible_for_seer,
                private_object_owner: detection.private_object_owner,
                object_id_visibility_conditions_met: detection.object_id_visibility_conditions_met,
                server_side_visibility_gm: detection.server_side_visibility_gm,
                server_side_visibility_ghost: detection.server_side_visibility_ghost,
                ghost_visible_to_seer_by_group: detection.ghost_visible_to_seer_by_group,
                invisible_due_to_despawn: detection.invisible_due_to_despawn,
                always_detectable_for_seer: detection.always_detectable_for_seer,
                invisibility_flags: detection.invisibility_flags,
                stealth_flags: detection.stealth_flags,
            },
            phase_shift: world.phase_shift(),
            smooth_phasing: world.smooth_phasing_like_cpp(),
            invisibility: &detection.invisibility,
            stealth: &detection.stealth,
            stalked_casters: StalkedCasters::Applied(
                target.subsystems.auras.applied_aura_types
                    .get(&SPELL_AURA_MOD_STALKED_LIKE_CPP)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            ),
        }
    }

    fn captured(target: &'a UnitVisibilityTargetFacts) -> Self {
        Self {
            scalar: target.scalar,
            phase_shift: &target.phase_shift,
            smooth_phasing: target.smooth_phasing.as_ref(),
            invisibility: &target.invisibility,
            stealth: &target.stealth,
            stalked_casters: StalkedCasters::Captured(&target.stalked_casters),
        }
    }
}

impl Unit {
    pub fn capture_visibility_target(&self) -> UnitVisibilityTargetFacts {
        let target = TargetView::borrowed(self);
        UnitVisibilityTargetFacts {
            scalar: target.scalar,
            phase_shift: target.phase_shift.clone(),
            smooth_phasing: target.smooth_phasing.cloned(),
            invisibility: *target.invisibility,
            stealth: *target.stealth,
            stalked_casters: target.stalked_casters.capture(),
        }
    }

    pub fn can_see_or_detect_target(
        &self,
        target: &UnitVisibilityTargetFacts,
        implicit_detect: bool,
        seer_is_player: bool,
        check_alert: bool,
    ) -> bool {
        can_see(self, &TargetView::captured(target), false,
            implicit_detect, seer_is_player, check_alert)
    }
}

pub(super) fn can_detect_invisibility_of(seer: &Unit, target: &Unit) -> bool {
    can_detect_invisibility(seer, &TargetView::borrowed(target))
}

pub(super) fn can_detect_stealth_of(
    seer: &Unit, target: &Unit, seer_is_player: bool, check_alert: bool,
) -> bool {
    can_detect_stealth(seer, &TargetView::borrowed(target),
        std::ptr::eq(&seer.world, &target.world), seer_is_player, check_alert)
}

pub(super) fn can_see_or_detect_unit(
    seer: &Unit, target: &Unit, implicit_detect: bool, seer_is_player: bool, check_alert: bool,
) -> bool {
    can_see(seer, &TargetView::borrowed(target),
        std::ptr::eq(&seer.world, &target.world), implicit_detect, seer_is_player, check_alert)
}

fn can_detect_invisibility(seer: &Unit, target: &TargetView<'_>) -> bool {
    let target_flags = target.scalar.invisibility_flags;
    if target_flags == 0 {
        return true;
    }
    if target_flags & seer.visibility_detection.invisibility_detect_flags != target_flags {
        return false;
    }
    for aura_type in 0..MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
        let flag = 1_u64 << aura_type;
        if target_flags & flag == 0 {
            continue;
        }
        if seer.visibility_detection.invisibility_detect[aura_type] < target.invisibility[aura_type] {
            return false;
        }
    }
    true
}

fn can_detect_stealth(
    seer: &Unit, target: &TargetView<'_>, same_world_reference: bool,
    seer_is_player: bool, check_alert: bool,
) -> bool {
    let target_flags = target.scalar.stealth_flags;
    if target_flags == 0 {
        return true;
    }
    let distance = seer.world.position().distance(&target.scalar.position);
    let combat_reach = seer.data.combat_reach.max(0.0);
    if distance < combat_reach {
        return true;
    }
    // Legacy has_in_arc returns true for the same WorldObject reference.
    // Captured targets are distinct; the position helper has the same angle arithmetic.
    if !same_world_reference
        && !seer.world.has_position_in_arc(std::f32::consts::PI, target.scalar.position, 2.0)
    {
        return false;
    }
    for aura_type in 0..MAX_VISIBILITY_AURA_TYPES_LIKE_CPP {
        let flag = 1_u64 << aura_type;
        if target_flags & flag == 0 {
            continue;
        }
        let level = seer.data.level.max(1);
        let detection_value =
            30 + (level - 1) * 5 + seer.visibility_detection.stealth_detect[aura_type]
                - target.stealth[aura_type];
        let mut visibility_range = detection_value as f32 * 0.3 + combat_reach;
        if seer_is_player {
            visibility_range = visibility_range.min(MAX_PLAYER_STEALTH_DETECT_RANGE_LIKE_CPP);
        }
        if check_alert {
            visibility_range += visibility_range * 0.08 + 1.5;
        }
        if distance > visibility_range {
            return false;
        }
    }
    true
}

fn can_see(
    seer: &Unit, target: &TargetView<'_>, same_world_reference: bool,
    implicit_detect: bool, seer_is_player: bool, check_alert: bool,
) -> bool {
    let seer_guid = seer.world.object().guid();
    let target_scalar = &target.scalar;
    if !seer_guid.is_empty() && seer_guid == target_scalar.guid {
        return true;
    }
    if target_scalar.never_visible_for_seer
        || seer.visibility_detection.seer_can_never_see_target
        || (seer.world.has_current_map()
            && target_scalar.has_current_map
            && !(seer.world.object().is_in_world()
                && target_scalar.is_in_world
                && seer.world.map_id() == target_scalar.map_id
                && seer.world.instance_id() == target_scalar.instance_id))
        || !seer.world.phase_shift().can_see(target.phase_shift)
    {
        return false;
    }
    if target_scalar.always_visible_for_seer
        || seer.visibility_detection.seer_can_always_see_target
        || target_scalar.charmer_or_owner_guid.is_some_and(|owner_guid| owner_guid == seer_guid)
        || target_scalar.target_owner_group_visible_for_seer
        || (!seer.visibility_detection.seer_can_always_see_target_guid.is_empty()
            && seer.visibility_detection.seer_can_always_see_target_guid == target_scalar.guid)
    {
        return true;
    }
    let private_owner = target_scalar.private_object_owner;
    if !private_owner.is_empty()
        && private_owner != seer.world.object().guid()
        && private_owner != seer.visibility_detection.seer_private_object_owner
        && !seer.visibility_detection.seer_group_visible_for_private_owner
    {
        return false;
    }
    if target.smooth_phasing.is_some_and(|smooth_phasing| {
        smooth_phasing.is_being_replaced_for_seer_like_cpp(seer_guid)
    }) {
        return false;
    }
    if private_owner.is_empty() && !target_scalar.object_id_visibility_conditions_met {
        return false;
    }
    let gm_visibility = target_scalar.server_side_visibility_gm;
    if gm_visibility == 0 {
        if seer.visibility_detection.server_side_visibility_detect_gm != 0 {
            return true;
        }
    } else {
        return seer.visibility_detection.server_side_visibility_detect_gm >= gm_visibility;
    }
    if target_scalar.server_side_visibility_ghost
        & seer.visibility_detection.server_side_visibility_detect_ghost == 0
        && !(seer_is_player && target_scalar.ghost_visible_to_seer_by_group)
    {
        return false;
    }
    if target_scalar.invisible_due_to_despawn {
        return false;
    }
    if target_scalar.always_detectable_for_seer || target.stalked_casters.contains(seer_guid) {
        return true;
    }
    if !implicit_detect && !can_detect_invisibility(seer, target) {
        return false;
    }
    if !implicit_detect
        && !can_detect_stealth(seer, target, same_world_reference, seer_is_player, check_alert)
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests;
