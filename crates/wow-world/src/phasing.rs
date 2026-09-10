// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `PhasingHandler` façade slices that are independent of runtime unit graphs.

use std::{collections::HashSet, error::Error, fmt};

use std::fmt::Write as _;

use wow_constants::{PhaseFlags, PhaseShiftFlags, TypeId};
use wow_core::ObjectGuid;
use wow_data::{AreaTableStore, PhaseGroupStore, PhaseInfoStore, PhaseStore, TerrainSwapStore};
use wow_entities::{PhaseShift, Unit, WorldObject};
use wow_packet::packets::misc::{PhaseShiftChange, PhaseShiftDataPhase};
use wow_packet::packets::party::{PartyMemberPhase, PartyMemberPhaseStates};

#[path = "phasing/personal.rs"]
pub mod personal;

pub const PHASE_USE_FLAGS_ALWAYS_VISIBLE: u8 = 0x01;
pub const PHASE_USE_FLAGS_INVERSE: u8 = 0x02;
const DEFAULT_PHASE: u32 = 169;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseVisibilityUpdate {
    pub update_visibility: bool,
    pub changed: bool,
}

impl PhaseVisibilityUpdate {
    pub const fn new(update_visibility: bool, changed: bool) -> Self {
        Self {
            update_visibility,
            changed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseShiftPacketBuildError {
    PhaseIdOutOfRange(u32),
    VisibleMapIdOutOfRange(u32),
    UiMapPhaseIdOutOfRange(u32),
}

impl fmt::Display for PhaseShiftPacketBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhaseIdOutOfRange(id) => {
                write!(f, "phase id {id} does not fit SMSG_PHASE_SHIFT_CHANGE")
            }
            Self::VisibleMapIdOutOfRange(id) => {
                write!(
                    f,
                    "visible map id {id} does not fit SMSG_PHASE_SHIFT_CHANGE"
                )
            }
            Self::UiMapPhaseIdOutOfRange(id) => {
                write!(
                    f,
                    "UI map phase id {id} does not fit SMSG_PHASE_SHIFT_CHANGE"
                )
            }
        }
    }
}

impl Error for PhaseShiftPacketBuildError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlledUnitInfo {
    pub guid: ObjectGuid,
    pub type_id: TypeId,
    pub has_vehicle: bool,
}

impl ControlledUnitInfo {
    pub const fn new(guid: ObjectGuid, type_id: TypeId, has_vehicle: bool) -> Self {
        Self {
            guid,
            type_id,
            has_vehicle,
        }
    }
}

/// C++ `PhasingHandler::ControlledUnitVisitor` visited-set and selection rules.
pub struct ControlledUnitVisitor {
    visited: HashSet<ObjectGuid>,
}

impl ControlledUnitVisitor {
    pub fn new(owner_guid: ObjectGuid) -> Self {
        let mut visited = HashSet::new();
        visited.insert(owner_guid);
        Self { visited }
    }

    pub fn was_visited(&self, guid: ObjectGuid) -> bool {
        self.visited.contains(&guid)
    }

    pub fn visit_controlled_of_like_cpp<ResolveControlled, SummonExists, VehiclePassengers, Visit>(
        &mut self,
        unit: &Unit,
        mut resolve_controlled: ResolveControlled,
        mut summon_exists: SummonExists,
        vehicle_passengers: VehiclePassengers,
        mut visit: Visit,
    ) where
        ResolveControlled: FnMut(ObjectGuid) -> Option<ControlledUnitInfo>,
        SummonExists: FnMut(ObjectGuid) -> bool,
        VehiclePassengers: IntoIterator<Item = ObjectGuid>,
        Visit: FnMut(ObjectGuid),
    {
        for controlled_guid in &unit.subsystems().control.controlled_guids {
            let Some(controlled) = resolve_controlled(*controlled_guid) else {
                continue;
            };
            if controlled.type_id != TypeId::Player
                && !controlled.has_vehicle
                && self.visited.insert(controlled.guid)
            {
                visit(controlled.guid);
            }
        }

        for summon_guid in unit.subsystems().control.summon_slots {
            if !summon_guid.is_empty()
                && summon_exists(summon_guid)
                && self.visited.insert(summon_guid)
            {
                visit(summon_guid);
            }
        }

        for passenger_guid in vehicle_passengers {
            if !passenger_guid.is_empty()
                && passenger_guid != unit.world().guid()
                && self.visited.insert(passenger_guid)
            {
                visit(passenger_guid);
            }
        }
    }
}

/// C++ local `PhasingHandler.cpp::GetPhaseFlags`.
pub fn phase_flags_for_id_like_cpp(phase_store: &PhaseStore, phase_id: u32) -> PhaseFlags {
    if phase_store.is_cosmetic_phase(phase_id) {
        return PhaseFlags::COSMETIC;
    }

    if phase_store.is_personal_phase(phase_id) {
        return PhaseFlags::PERSONAL;
    }

    PhaseFlags::NONE
}

/// C++ `PhasingHandler::InitDbPhaseShift`.
pub fn init_db_phase_shift_like_cpp(
    phase_shift: &mut PhaseShift,
    phase_store: &PhaseStore,
    phase_group_store: &PhaseGroupStore,
    phase_use_flags: u8,
    phase_id: u16,
    phase_group_id: u32,
) {
    phase_shift.clear_phases_like_cpp();
    phase_shift.set_db_phase_shift_like_cpp(true);

    let mut flags = PhaseShiftFlags::NONE;
    if phase_use_flags & PHASE_USE_FLAGS_ALWAYS_VISIBLE != 0 {
        flags |= PhaseShiftFlags::ALWAYS_VISIBLE | PhaseShiftFlags::UNPHASED;
    }
    if phase_use_flags & PHASE_USE_FLAGS_INVERSE != 0 {
        flags |= PhaseShiftFlags::INVERSE;
    }

    if phase_id != 0 {
        let phase_id = u32::from(phase_id);
        phase_shift.add_phase_like_cpp(
            phase_id,
            phase_flags_for_id_like_cpp(phase_store, phase_id),
            1,
        );
    } else if phase_group_id != 0
        && let Some(phases_in_group) = phase_group_store.phases_for_group(phase_group_id)
    {
        for phase_in_group in phases_in_group {
            phase_shift.add_phase_like_cpp(
                *phase_in_group,
                phase_flags_for_id_like_cpp(phase_store, *phase_in_group),
                1,
            );
        }
    }

    if phase_shift.phase_count_like_cpp() == 0 || phase_shift.has_phase_like_cpp(DEFAULT_PHASE) {
        if flags.contains(PhaseShiftFlags::INVERSE) {
            flags |= PhaseShiftFlags::INVERSE_UNPHASED;
        } else {
            flags |= PhaseShiftFlags::UNPHASED;
        }
    }

    phase_shift.set_flags_like_cpp(flags);
}

/// C++ `PhasingHandler::InitDbPersonalOwnership`.
pub fn init_db_personal_ownership_like_cpp(
    phase_shift: &mut PhaseShift,
    personal_guid: ObjectGuid,
) {
    assert!(phase_shift.is_db_phase_shift_like_cpp());
    assert!(phase_shift.has_personal_phase_like_cpp());
    phase_shift.set_personal_guid_like_cpp(personal_guid);
}

/// C++ `PhasingHandler::InitDbVisibleMapId`.
pub fn init_db_visible_map_id_like_cpp(
    phase_shift: &mut PhaseShift,
    terrain_swap_store: &TerrainSwapStore,
    visible_map_id: i32,
) {
    phase_shift.clear_visible_map_ids_like_cpp();
    if let Ok(visible_map_id) = u32::try_from(visible_map_id)
        && terrain_swap_store
            .terrain_swap_info(visible_map_id)
            .is_some()
    {
        phase_shift.add_visible_map_id_like_cpp(visible_map_id, 1);
    }
}

/// C++ `PhasingHandler::ResetPhaseShift`.
pub fn reset_phase_shift_like_cpp(object: &mut WorldObject) {
    object.phase_shift_mut().clear();
    object.suppressed_phase_shift_mut().clear();
}

/// C++ `PhasingHandler::InheritPhaseShift`.
pub fn inherit_phase_shift_like_cpp(target: &mut WorldObject, source: &WorldObject) {
    *target.phase_shift_mut() = source.phase_shift().clone();
    *target.suppressed_phase_shift_mut() = source.suppressed_phase_shift().clone();
}

/// C++ `PhasingHandler::SetAlwaysVisible`.
pub fn set_always_visible_like_cpp(
    object: &mut WorldObject,
    apply: bool,
    update_visibility: bool,
) -> PhaseVisibilityUpdate {
    object.phase_shift_mut().set_always_visible_like_cpp(apply);
    PhaseVisibilityUpdate::new(update_visibility, true)
}

/// C++ `PhasingHandler::SetInversed`.
pub fn set_inversed_like_cpp(
    object: &mut WorldObject,
    apply: bool,
    update_visibility: bool,
) -> PhaseVisibilityUpdate {
    object.phase_shift_mut().set_inversed_like_cpp(apply);
    PhaseVisibilityUpdate::new(update_visibility, true)
}

/// C++ `PhasingHandler::AddPhase` core mutation, excluding runtime controlled-unit traversal.
pub fn add_phase_like_cpp(
    object: &mut WorldObject,
    phase_store: &PhaseStore,
    phase_id: u32,
    personal_guid: ObjectGuid,
    update_visibility: bool,
) -> PhaseVisibilityUpdate {
    let flags = phase_flags_for_id_like_cpp(phase_store, phase_id);
    let changed = object
        .phase_shift_mut()
        .add_phase_like_cpp(phase_id, flags, 1);

    if object.phase_shift().has_personal_phase_like_cpp() {
        object
            .phase_shift_mut()
            .set_personal_guid_like_cpp(personal_guid);
    }

    PhaseVisibilityUpdate::new(update_visibility, changed)
}

/// Public C++ `PhasingHandler::AddPhase` entry shape for a single object.
pub fn add_object_phase_like_cpp(
    object: &mut WorldObject,
    phase_store: &PhaseStore,
    phase_id: u32,
    update_visibility: bool,
) -> PhaseVisibilityUpdate {
    add_phase_like_cpp(
        object,
        phase_store,
        phase_id,
        object.guid(),
        update_visibility,
    )
}

/// C++ `PhasingHandler::RemovePhase` core mutation, excluding runtime controlled-unit traversal.
pub fn remove_phase_like_cpp(
    object: &mut WorldObject,
    phase_id: u32,
    update_visibility: bool,
) -> PhaseVisibilityUpdate {
    let changed = object.phase_shift_mut().remove_phase_like_cpp(phase_id);
    PhaseVisibilityUpdate::new(update_visibility, changed)
}

/// C++ `PhasingHandler::AddPhaseGroup` core mutation, excluding runtime controlled-unit traversal.
pub fn add_phase_group_like_cpp(
    object: &mut WorldObject,
    phase_store: &PhaseStore,
    phase_group_store: &PhaseGroupStore,
    phase_group_id: u32,
    personal_guid: ObjectGuid,
    update_visibility: bool,
) -> Option<PhaseVisibilityUpdate> {
    let phases = phase_group_store.phases_for_group(phase_group_id)?;
    let mut changed = false;
    for phase_id in phases {
        let flags = phase_flags_for_id_like_cpp(phase_store, *phase_id);
        changed = object
            .phase_shift_mut()
            .add_phase_like_cpp(*phase_id, flags, 1)
            || changed;
    }

    if object.phase_shift().has_personal_phase_like_cpp() {
        object
            .phase_shift_mut()
            .set_personal_guid_like_cpp(personal_guid);
    }

    Some(PhaseVisibilityUpdate::new(update_visibility, changed))
}

/// Public C++ `PhasingHandler::AddPhaseGroup` entry shape for a single object.
pub fn add_object_phase_group_like_cpp(
    object: &mut WorldObject,
    phase_store: &PhaseStore,
    phase_group_store: &PhaseGroupStore,
    phase_group_id: u32,
    update_visibility: bool,
) -> Option<PhaseVisibilityUpdate> {
    add_phase_group_like_cpp(
        object,
        phase_store,
        phase_group_store,
        phase_group_id,
        object.guid(),
        update_visibility,
    )
}

/// C++ `PhasingHandler::RemovePhaseGroup` core mutation, excluding runtime controlled-unit traversal.
pub fn remove_phase_group_like_cpp(
    object: &mut WorldObject,
    phase_group_store: &PhaseGroupStore,
    phase_group_id: u32,
    update_visibility: bool,
) -> Option<PhaseVisibilityUpdate> {
    let phases = phase_group_store.phases_for_group(phase_group_id)?;
    let mut changed = false;
    for phase_id in phases {
        changed = object.phase_shift_mut().remove_phase_like_cpp(*phase_id) || changed;
    }

    Some(PhaseVisibilityUpdate::new(update_visibility, changed))
}

/// C++ `PhasingHandler::AddVisibleMapId` core mutation, excluding runtime controlled-unit traversal.
pub fn add_visible_map_id_like_cpp(
    object: &mut WorldObject,
    terrain_swap_store: &TerrainSwapStore,
    visible_map_id: u32,
) -> Option<PhaseVisibilityUpdate> {
    let terrain_swap_info = terrain_swap_store.terrain_swap_info(visible_map_id)?;
    let mut changed = object
        .phase_shift_mut()
        .add_visible_map_id_like_cpp(visible_map_id, 1);

    for ui_map_phase_id in &terrain_swap_info.ui_map_phase_ids {
        changed = object
            .phase_shift_mut()
            .add_ui_map_phase_id_like_cpp(*ui_map_phase_id, 1)
            || changed;
    }

    Some(PhaseVisibilityUpdate::new(false, changed))
}

/// C++ `PhasingHandler::RemoveVisibleMapId` core mutation, excluding runtime controlled-unit traversal.
pub fn remove_visible_map_id_like_cpp(
    object: &mut WorldObject,
    terrain_swap_store: &TerrainSwapStore,
    visible_map_id: u32,
) -> Option<PhaseVisibilityUpdate> {
    let terrain_swap_info = terrain_swap_store.terrain_swap_info(visible_map_id)?;
    let mut changed = object
        .phase_shift_mut()
        .remove_visible_map_id_like_cpp(visible_map_id);

    for ui_map_phase_id in &terrain_swap_info.ui_map_phase_ids {
        changed = object
            .phase_shift_mut()
            .remove_ui_map_phase_id_like_cpp(*ui_map_phase_id)
            || changed;
    }

    Some(PhaseVisibilityUpdate::new(false, changed))
}

/// C++ `PhasingHandler::OnMapChange` core terrain-swap pass.
///
/// The condition predicate represents
/// `sConditionMgr->IsObjectMeetingNotGroupedConditions(CONDITION_SOURCE_TYPE_TERRAIN_SWAP, id, srcInfo)`.
pub fn on_map_change_like_cpp(
    object: &mut WorldObject,
    terrain_swap_store: &TerrainSwapStore,
    mut terrain_swap_conditions_pass: impl FnMut(u32, &WorldObject) -> bool,
) -> PhaseVisibilityUpdate {
    object.phase_shift_mut().clear_visible_map_ids_like_cpp();
    object.phase_shift_mut().clear_ui_map_phase_ids_like_cpp();
    object
        .suppressed_phase_shift_mut()
        .clear_visible_map_ids_like_cpp();

    let object_map_id = object.map_id();
    for (map_id, terrain_swap_ids) in terrain_swap_store.terrain_swaps_by_map_like_cpp() {
        for terrain_swap_id in terrain_swap_ids {
            let Some(terrain_swap_info) = terrain_swap_store.terrain_swap_info(*terrain_swap_id)
            else {
                continue;
            };

            if terrain_swap_conditions_pass(terrain_swap_info.id, object) {
                if map_id == object_map_id {
                    object
                        .phase_shift_mut()
                        .add_visible_map_id_like_cpp(terrain_swap_info.id, 1);
                }

                for ui_map_phase_id in &terrain_swap_info.ui_map_phase_ids {
                    object
                        .phase_shift_mut()
                        .add_ui_map_phase_id_like_cpp(*ui_map_phase_id, 1);
                }
            } else if map_id == object_map_id {
                object
                    .suppressed_phase_shift_mut()
                    .add_visible_map_id_like_cpp(terrain_swap_info.id, 1);
            }
        }
    }

    PhaseVisibilityUpdate::new(false, true)
}

/// C++ `PhasingHandler::OnAreaChange` core area-phase pass.
///
/// `phase_area_conditions_pass` represents
/// `sConditionMgr->IsObjectMeetToConditions(srcInfo, phaseArea.Conditions)`.
/// `aura_phase_ids` and `aura_phase_group_ids` represent active
/// `SPELL_AURA_PHASE` / `SPELL_AURA_PHASE_GROUP` effects already filtered by the caller.
pub fn on_area_change_like_cpp(
    object: &mut WorldObject,
    area_store: &AreaTableStore,
    phase_store: &PhaseStore,
    phase_group_store: &PhaseGroupStore,
    phase_info_store: &PhaseInfoStore,
    mut phase_area_conditions_pass: impl FnMut(u32, &WorldObject) -> bool,
    aura_phase_ids: impl IntoIterator<Item = u32>,
    aura_phase_group_ids: impl IntoIterator<Item = u32>,
) -> PhaseVisibilityUpdate {
    let old_phases = object.phase_shift().phase_snapshot_like_cpp();

    object.phase_shift_mut().clear_phases_like_cpp();
    object.suppressed_phase_shift_mut().clear_phases_like_cpp();

    let original_area_id = object.area_id();
    let mut area_id = original_area_id;
    while let Some(area_entry) = area_store.get(area_id) {
        if let Some(area_phases) = phase_info_store.phases_for_area(area_entry.id) {
            for phase_area in area_phases {
                if phase_area.sub_area_exclusions.contains(&original_area_id) {
                    continue;
                }

                let phase_id = phase_area.phase_id;
                let phase_flags = phase_flags_for_id_like_cpp(phase_store, phase_id);
                if phase_area_conditions_pass(phase_id, object) {
                    object
                        .phase_shift_mut()
                        .add_phase_like_cpp(phase_id, phase_flags, 1);
                } else {
                    object.suppressed_phase_shift_mut().add_phase_like_cpp(
                        phase_id,
                        phase_flags,
                        1,
                    );
                }
            }
        }

        area_id = u32::from(area_entry.parent_area_id);
        if area_id == 0 {
            break;
        }
    }

    let mut changed = object.phase_shift().phase_snapshot_like_cpp() != old_phases;

    for phase_id in aura_phase_ids {
        let flags = phase_flags_for_id_like_cpp(phase_store, phase_id);
        changed = object
            .phase_shift_mut()
            .add_phase_like_cpp(phase_id, flags, 1)
            || changed;
    }

    for phase_group_id in aura_phase_group_ids {
        let Some(phases) = phase_group_store.phases_for_group(phase_group_id) else {
            continue;
        };
        for phase_id in phases {
            let flags = phase_flags_for_id_like_cpp(phase_store, *phase_id);
            changed = object
                .phase_shift_mut()
                .add_phase_like_cpp(*phase_id, flags, 1)
                || changed;
        }
    }

    if object.phase_shift().has_personal_phase_like_cpp() {
        let personal_guid = object.guid();
        object
            .phase_shift_mut()
            .set_personal_guid_like_cpp(personal_guid);
    }

    PhaseVisibilityUpdate::new(true, changed)
}

/// C++ `PhasingHandler::OnConditionChange` core mutation, excluding runtime unit side effects.
///
/// `active_phase_condition_pass` represents the nullable `PhaseRef::AreaConditions` check:
/// returning `None` means the active phase has no area condition pointer and must not be
/// suppressed by this pass. `suppressed_phase_condition_pass` represents the C++ asserted
/// non-null condition pointer on suppressed phases. Terrain-swap conditions use
/// `CONDITION_SOURCE_TYPE_TERRAIN_SWAP`.
pub fn on_condition_change_like_cpp(
    object: &mut WorldObject,
    phase_store: &PhaseStore,
    phase_group_store: &PhaseGroupStore,
    terrain_swap_store: &TerrainSwapStore,
    update_visibility: bool,
    mut active_phase_condition_pass: impl FnMut(u32, &WorldObject) -> Option<bool>,
    mut suppressed_phase_condition_pass: impl FnMut(u32, &WorldObject) -> bool,
    mut terrain_swap_conditions_pass: impl FnMut(u32, &WorldObject) -> bool,
    aura_phase_ids: impl IntoIterator<Item = u32>,
    aura_phase_group_ids: impl IntoIterator<Item = u32>,
) -> PhaseVisibilityUpdate {
    let mut new_suppressions = PhaseShift::default();
    let mut changed = false;

    let active_phases = object.phase_shift().phase_snapshot_like_cpp();
    for phase_ref in active_phases {
        if active_phase_condition_pass(phase_ref.id(), object) == Some(false)
            && let Some(removed) = object
                .phase_shift_mut()
                .remove_phase_all_references_like_cpp(phase_ref.id())
        {
            new_suppressions.add_phase_like_cpp(
                removed.id(),
                removed.flags(),
                removed.references(),
            );
        }
    }

    let suppressed_phases = object.suppressed_phase_shift().phase_snapshot_like_cpp();
    for phase_ref in suppressed_phases {
        if suppressed_phase_condition_pass(phase_ref.id(), object)
            && let Some(removed) = object
                .suppressed_phase_shift_mut()
                .remove_phase_all_references_like_cpp(phase_ref.id())
        {
            changed = object.phase_shift_mut().add_phase_like_cpp(
                removed.id(),
                removed.flags(),
                removed.references(),
            ) || changed;
        }
    }

    let active_visible_maps = object.phase_shift().visible_map_id_snapshot_like_cpp();
    for (visible_map_id, _visible_map_ref) in active_visible_maps {
        if !terrain_swap_conditions_pass(visible_map_id, object)
            && let Some(removed) = object
                .phase_shift_mut()
                .remove_visible_map_id_all_references_like_cpp(visible_map_id)
        {
            new_suppressions.add_visible_map_id_like_cpp(visible_map_id, removed.references());

            if let Some(terrain_swap_info) = terrain_swap_store.terrain_swap_info(visible_map_id) {
                for ui_map_phase_id in &terrain_swap_info.ui_map_phase_ids {
                    changed = object
                        .phase_shift_mut()
                        .remove_ui_map_phase_id_like_cpp(*ui_map_phase_id)
                        || changed;
                }
            }
        }
    }

    let suppressed_visible_maps = object
        .suppressed_phase_shift()
        .visible_map_id_snapshot_like_cpp();
    for (visible_map_id, visible_map_ref) in suppressed_visible_maps {
        if terrain_swap_conditions_pass(visible_map_id, object)
            && object
                .suppressed_phase_shift_mut()
                .remove_visible_map_id_all_references_like_cpp(visible_map_id)
                .is_some()
        {
            changed = object
                .phase_shift_mut()
                .add_visible_map_id_like_cpp(visible_map_id, visible_map_ref.references())
                || changed;

            if let Some(terrain_swap_info) = terrain_swap_store.terrain_swap_info(visible_map_id) {
                for ui_map_phase_id in &terrain_swap_info.ui_map_phase_ids {
                    changed = object
                        .phase_shift_mut()
                        .add_ui_map_phase_id_like_cpp(*ui_map_phase_id, 1)
                        || changed;
                }
            }
        }
    }

    for phase_id in aura_phase_ids {
        if new_suppressions.has_phase_like_cpp(phase_id) {
            new_suppressions.remove_phase_like_cpp(phase_id);
            let flags = phase_flags_for_id_like_cpp(phase_store, phase_id);
            object
                .phase_shift_mut()
                .add_phase_like_cpp(phase_id, flags, 1);
        }
    }

    for phase_group_id in aura_phase_group_ids {
        let Some(phases) = phase_group_store.phases_for_group(phase_group_id) else {
            continue;
        };
        for phase_id in phases {
            if new_suppressions.has_phase_like_cpp(*phase_id) {
                new_suppressions.remove_phase_like_cpp(*phase_id);
                let flags = phase_flags_for_id_like_cpp(phase_store, *phase_id);
                object
                    .phase_shift_mut()
                    .add_phase_like_cpp(*phase_id, flags, 1);
            }
        }
    }

    if object.phase_shift().has_personal_phase_like_cpp() {
        let personal_guid = object.guid();
        object
            .phase_shift_mut()
            .set_personal_guid_like_cpp(personal_guid);
    }

    changed = changed
        || new_suppressions.phase_count_like_cpp() != 0
        || new_suppressions.visible_map_id_count_like_cpp() != 0;

    for phase_ref in new_suppressions.phase_snapshot_like_cpp() {
        object.suppressed_phase_shift_mut().add_phase_like_cpp(
            phase_ref.id(),
            phase_ref.flags(),
            phase_ref.references(),
        );
    }

    for (visible_map_id, visible_map_ref) in new_suppressions.visible_map_id_snapshot_like_cpp() {
        object
            .suppressed_phase_shift_mut()
            .add_visible_map_id_like_cpp(visible_map_id, visible_map_ref.references());
    }

    PhaseVisibilityUpdate::new(update_visibility, changed)
}

/// C++ `PhasingHandler::SendToPlayer(Player const*, PhaseShift const&)` packet build step.
pub fn phase_shift_change_for_player_like_cpp(
    player_guid: ObjectGuid,
    phase_shift: &PhaseShift,
) -> Result<PhaseShiftChange, PhaseShiftPacketBuildError> {
    let phases = phase_shift
        .phases_like_cpp()
        .map(|phase| {
            Ok(PhaseShiftDataPhase {
                phase_flags: phase.flags().bits(),
                id: u16::try_from(phase.id())
                    .map_err(|_| PhaseShiftPacketBuildError::PhaseIdOutOfRange(phase.id()))?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let visible_map_ids = phase_shift
        .visible_map_ids_like_cpp()
        .map(|visible_map_id| {
            u16::try_from(visible_map_id)
                .map_err(|_| PhaseShiftPacketBuildError::VisibleMapIdOutOfRange(visible_map_id))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let ui_map_phase_ids = phase_shift
        .ui_map_phase_ids_like_cpp()
        .map(|ui_map_phase_id| {
            u16::try_from(ui_map_phase_id)
                .map_err(|_| PhaseShiftPacketBuildError::UiMapPhaseIdOutOfRange(ui_map_phase_id))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PhaseShiftChange {
        player_guid,
        phase_shift_flags: phase_shift.flags_like_cpp().bits(),
        phases,
        personal_guid: phase_shift.personal_guid_like_cpp(),
        visible_map_ids,
        preload_map_ids: Vec::new(),
        ui_map_phase_ids,
    })
}

/// C++ `PhasingHandler::FillPartyMemberPhase`.
pub fn party_member_phase_states_like_cpp(
    phase_shift: &PhaseShift,
) -> Result<PartyMemberPhaseStates, PhaseShiftPacketBuildError> {
    let phases = phase_shift
        .phases_like_cpp()
        .map(|phase| {
            Ok(PartyMemberPhase {
                flags: u32::from(phase.flags().bits()),
                id: u16::try_from(phase.id())
                    .map_err(|_| PhaseShiftPacketBuildError::PhaseIdOutOfRange(phase.id()))?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PartyMemberPhaseStates {
        phase_shift_flags: phase_shift.flags_like_cpp().bits(),
        personal_guid: phase_shift.personal_guid_like_cpp(),
        phases,
    })
}

/// Arguments produced by C++ `PhasingHandler::PrintToChat` before localization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseShiftChatSnapshot {
    pub flags: u32,
    pub personal_guid: ObjectGuid,
    pub personal_owner_name: String,
    pub phases: Option<String>,
    pub visible_map_ids: Option<String>,
    pub ui_map_phase_ids: Option<String>,
}

/// C++ `PhasingHandler::FormatPhases`.
pub fn format_phases_like_cpp(phase_shift: &PhaseShift) -> String {
    let mut phases = String::new();
    for phase in phase_shift.phases_like_cpp() {
        let _ = write!(phases, "{},", phase.id());
    }
    phases
}

/// C++ `PhasingHandler::PrintToChat`, split from the concrete `ChatHandler`.
pub fn print_to_chat_snapshot_like_cpp(
    target: &WorldObject,
    mut resolve_personal_owner_name: impl FnMut(ObjectGuid) -> Option<String>,
    mut resolve_phase_name: impl FnMut(u32) -> Option<String>,
    cosmetic_label: &str,
    personal_label: &str,
) -> PhaseShiftChatSnapshot {
    let phase_shift = target.phase_shift();
    let mut personal_owner_name = String::from("N/A");

    if phase_shift.has_personal_phase_like_cpp()
        && let Some(name) = resolve_personal_owner_name(phase_shift.personal_guid_like_cpp())
    {
        personal_owner_name = name;
    }

    let phases = if phase_shift.phase_count_like_cpp() != 0 {
        let mut phases = String::new();
        for phase in phase_shift.phases_like_cpp() {
            phases.push_str("\r\n   ");
            let phase_name =
                resolve_phase_name(phase.id()).unwrap_or_else(|| String::from("Unknown Name"));
            let _ = write!(phases, "{} ({})", phase.id(), phase_name);
            if phase.flags().contains(PhaseFlags::COSMETIC) {
                let _ = write!(phases, " ({cosmetic_label})");
            }
            if phase.flags().contains(PhaseFlags::PERSONAL) {
                let _ = write!(phases, " ({personal_label})");
            }
        }
        Some(phases)
    } else {
        None
    };

    let visible_map_ids = if phase_shift.visible_map_id_count_like_cpp() != 0 {
        let mut visible_map_ids = String::new();
        for visible_map_id in phase_shift.visible_map_ids_like_cpp() {
            let _ = write!(visible_map_ids, "{visible_map_id}, ");
        }
        Some(visible_map_ids)
    } else {
        None
    };

    let mut ui_map_phase_ids = String::new();
    for ui_map_phase_id in phase_shift.ui_map_phase_ids_like_cpp() {
        let _ = write!(ui_map_phase_ids, "{ui_map_phase_id}, ");
    }

    PhaseShiftChatSnapshot {
        flags: phase_shift.flags_like_cpp().bits(),
        personal_guid: phase_shift.personal_guid_like_cpp(),
        personal_owner_name,
        phases,
        visible_map_ids,
        ui_map_phase_ids: (!ui_map_phase_ids.is_empty()).then_some(ui_map_phase_ids),
    }
}

#[cfg(test)]
#[path = "phasing/tests/mod.rs"]
mod tests;
