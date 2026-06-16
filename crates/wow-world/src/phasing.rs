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
mod tests {
    use super::*;
    use wow_constants::{PhaseFlags, PhaseShiftFlags, TypeId, TypeMask};
    use wow_core::{ObjectGuid, guid::HighGuid};
    use wow_data::{
        AreaTableEntry, MapEntry, MapStore, PhaseEntry, PhaseXPhaseGroupEntry,
        phase::{PHASE_ENTRY_FLAG_COSMETIC, PHASE_ENTRY_FLAG_PERSONAL},
    };

    fn world_object() -> WorldObject {
        let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        object.object_mut().create(ObjectGuid::create_world_object(
            HighGuid::Creature,
            0,
            0,
            571,
            0,
            1,
            1,
        ));
        object
    }

    fn unit(guid: ObjectGuid) -> Unit {
        let mut unit = Unit::new(true);
        unit.world_mut().object_mut().create(guid);
        unit
    }

    fn phase_store() -> PhaseStore {
        PhaseStore::from_entries([
            PhaseEntry { id: 10, flags: 0 },
            PhaseEntry {
                id: 20,
                flags: PHASE_ENTRY_FLAG_PERSONAL,
            },
            PhaseEntry {
                id: 30,
                flags: PHASE_ENTRY_FLAG_COSMETIC,
            },
        ])
    }

    fn phase_group_store(phase_store: &PhaseStore) -> PhaseGroupStore {
        PhaseGroupStore::from_entries(
            phase_store,
            [
                PhaseXPhaseGroupEntry {
                    id: 1,
                    phase_id: 10,
                    phase_group_id: 7,
                },
                PhaseXPhaseGroupEntry {
                    id: 2,
                    phase_id: 20,
                    phase_group_id: 7,
                },
                PhaseXPhaseGroupEntry {
                    id: 3,
                    phase_id: 99,
                    phase_group_id: 7,
                },
            ],
        )
    }

    fn area_store() -> AreaTableStore {
        AreaTableStore::from_entries([
            AreaTableEntry {
                id: 100,
                continent_id: 0,
                parent_area_id: 0,
                mount_flags: 0,
                flags: 0,
            },
            AreaTableEntry {
                id: 101,
                continent_id: 0,
                parent_area_id: 100,
                mount_flags: 0,
                flags: 0,
            },
            AreaTableEntry {
                id: 102,
                continent_id: 0,
                parent_area_id: 100,
                mount_flags: 0,
                flags: 0,
            },
        ])
    }

    fn phase_info_store(area_store: &AreaTableStore, phase_store: &PhaseStore) -> PhaseInfoStore {
        let mut store = PhaseInfoStore::from_phase_store_like_cpp(phase_store);
        store.load_area_phases_from_rows_like_cpp(
            area_store,
            phase_store,
            [(100, 10), (101, 10), (100, 20), (102, 30)],
        );
        store
    }

    fn map(id: u32, parent_map_id: i16) -> MapEntry {
        MapEntry {
            id,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id,
            cosmetic_parent_map_id: -1,
            flags1: 0,
        }
    }

    fn terrain_swap_store() -> TerrainSwapStore {
        let map_store =
            MapStore::from_entries([map(1, -1), map(571, -1), map(609, 571), map(700, 1)]);
        TerrainSwapStore::from_rows_like_cpp(
            &map_store,
            [(609, 42), (609, 43), (700, 70)],
            [(571, 609), (1, 700)],
            |_| true,
        )
    }

    #[test]
    fn reset_phase_shift_clears_active_and_suppressed_like_cpp() {
        let mut object = world_object();
        object
            .phase_shift_mut()
            .add_phase_like_cpp(10, PhaseFlags::NONE, 1);
        object
            .suppressed_phase_shift_mut()
            .add_phase_like_cpp(20, PhaseFlags::NONE, 1);

        reset_phase_shift_like_cpp(&mut object);

        assert!(
            object
                .phase_shift()
                .flags_like_cpp()
                .contains(PhaseShiftFlags::UNPHASED)
        );
        assert!(
            object
                .suppressed_phase_shift()
                .flags_like_cpp()
                .contains(PhaseShiftFlags::UNPHASED)
        );
        assert!(!object.phase_shift().has_phase_like_cpp(10));
        assert!(!object.suppressed_phase_shift().has_phase_like_cpp(20));
    }

    #[test]
    fn controlled_unit_visitor_matches_cpp_selection_rules() {
        let owner_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 1);
        let controlled_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 2);
        let controlled_player = ObjectGuid::create_player(1, 50);
        let nested_vehicle_passenger =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 3);
        let summon_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 4);
        let missing_summon =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 5);
        let vehicle_passenger =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 6);

        let mut unit = unit(owner_guid);
        unit.subsystems_mut()
            .control
            .add_controlled(controlled_guid);
        unit.subsystems_mut()
            .control
            .add_controlled(controlled_player);
        unit.subsystems_mut()
            .control
            .add_controlled(nested_vehicle_passenger);
        unit.subsystems_mut()
            .control
            .set_summon_slot(0, summon_guid);
        unit.subsystems_mut()
            .control
            .set_summon_slot(1, missing_summon);

        let mut visitor = ControlledUnitVisitor::new(owner_guid);
        let mut visited = Vec::new();
        visitor.visit_controlled_of_like_cpp(
            &unit,
            |guid| {
                if guid == controlled_guid {
                    Some(ControlledUnitInfo::new(guid, TypeId::Unit, false))
                } else if guid == controlled_player {
                    Some(ControlledUnitInfo::new(guid, TypeId::Player, false))
                } else if guid == nested_vehicle_passenger {
                    Some(ControlledUnitInfo::new(guid, TypeId::Unit, true))
                } else {
                    None
                }
            },
            |guid| guid == summon_guid,
            [vehicle_passenger, owner_guid, summon_guid],
            |guid| visited.push(guid),
        );

        visited.sort();
        assert_eq!(
            visited,
            vec![controlled_guid, summon_guid, vehicle_passenger]
        );
        assert!(visitor.was_visited(owner_guid));
    }

    #[test]
    fn add_and_remove_phase_core_match_cpp_mutation() {
        let phase_store = phase_store();
        let mut object = world_object();
        let personal_guid = object.guid();

        let update = add_object_phase_like_cpp(&mut object, &phase_store, 20, true);

        assert_eq!(update, PhaseVisibilityUpdate::new(true, true));
        assert!(object.phase_shift().has_phase_like_cpp(20));
        assert_eq!(
            object
                .phase_shift()
                .phase_ref_like_cpp(20)
                .map(|phase| phase.flags()),
            Some(PhaseFlags::PERSONAL)
        );
        assert_eq!(object.phase_shift().personal_guid_like_cpp(), personal_guid);

        let update = add_object_phase_like_cpp(&mut object, &phase_store, 20, true);
        assert_eq!(update, PhaseVisibilityUpdate::new(true, false));

        let update = remove_phase_like_cpp(&mut object, 20, false);
        assert_eq!(update, PhaseVisibilityUpdate::new(false, false));
        assert!(object.phase_shift().has_phase_like_cpp(20));

        let update = remove_phase_like_cpp(&mut object, 20, false);
        assert_eq!(update, PhaseVisibilityUpdate::new(false, true));
        assert!(!object.phase_shift().has_phase_like_cpp(20));
    }

    #[test]
    fn phase_group_core_uses_cpp_group_lookup_and_phase_flags() {
        let phase_store = phase_store();
        let phase_group_store = phase_group_store(&phase_store);
        let mut object = world_object();

        let update =
            add_object_phase_group_like_cpp(&mut object, &phase_store, &phase_group_store, 7, true);

        assert_eq!(update, Some(PhaseVisibilityUpdate::new(true, true)));
        assert!(object.phase_shift().has_phase_like_cpp(10));
        assert!(object.phase_shift().has_phase_like_cpp(20));
        assert!(!object.phase_shift().has_phase_like_cpp(99));

        let missing_group = add_object_phase_group_like_cpp(
            &mut object,
            &phase_store,
            &phase_group_store,
            99,
            true,
        );
        assert_eq!(missing_group, None);

        let update = remove_phase_group_like_cpp(&mut object, &phase_group_store, 7, false);
        assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, true)));
        assert!(!object.phase_shift().has_phase_like_cpp(10));
        assert!(!object.phase_shift().has_phase_like_cpp(20));
    }

    #[test]
    fn visible_map_id_core_updates_ui_map_phase_ids_like_cpp() {
        let terrain_swap_store = terrain_swap_store();
        let mut object = world_object();

        let update = add_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 609);

        assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, true)));
        assert!(object.phase_shift().has_visible_map_id_like_cpp(609));
        assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(42));
        assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(43));

        let update = add_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 609);
        assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, false)));

        let update = remove_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 609);
        assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, false)));
        assert!(object.phase_shift().has_visible_map_id_like_cpp(609));
        assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(42));

        let update = remove_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 609);
        assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, true)));
        assert!(!object.phase_shift().has_visible_map_id_like_cpp(609));
        assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(42));
        assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(43));
    }

    #[test]
    fn visible_map_id_core_ignores_missing_terrain_swap_info() {
        let terrain_swap_store = terrain_swap_store();
        let mut object = world_object();

        assert_eq!(
            add_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 999),
            None
        );
        assert!(!object.phase_shift().has_visible_map_id_like_cpp(999));
    }

    #[test]
    fn on_map_change_rebuilds_visible_ui_and_suppressed_swaps_like_cpp() {
        let terrain_swap_store = terrain_swap_store();
        let mut object = world_object();
        object.world_relocate(571, object.position());
        object.phase_shift_mut().add_visible_map_id_like_cpp(999, 1);
        object
            .phase_shift_mut()
            .add_ui_map_phase_id_like_cpp(999, 1);
        object
            .suppressed_phase_shift_mut()
            .add_visible_map_id_like_cpp(998, 1);

        let update = on_map_change_like_cpp(&mut object, &terrain_swap_store, |id, _| id != 609);

        assert_eq!(update, PhaseVisibilityUpdate::new(false, true));
        assert!(!object.phase_shift().has_visible_map_id_like_cpp(999));
        assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(999));
        assert!(
            !object
                .suppressed_phase_shift()
                .has_visible_map_id_like_cpp(998)
        );
        assert!(!object.phase_shift().has_visible_map_id_like_cpp(609));
        assert!(
            object
                .suppressed_phase_shift()
                .has_visible_map_id_like_cpp(609)
        );
        assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(42));
        assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(43));
        assert!(!object.phase_shift().has_visible_map_id_like_cpp(700));
        assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(70));
    }

    #[test]
    fn on_area_change_walks_parent_suppresses_and_reapplies_aura_phases_like_cpp() {
        let area_store = area_store();
        let phase_store = phase_store();
        let phase_group_store = phase_group_store(&phase_store);
        let phase_info_store = phase_info_store(&area_store, &phase_store);
        let mut object = world_object();
        let personal_guid = object.guid();
        object.set_zone_and_area(100, 101);
        object
            .phase_shift_mut()
            .add_phase_like_cpp(99, PhaseFlags::NONE, 1);
        object
            .suppressed_phase_shift_mut()
            .add_phase_like_cpp(98, PhaseFlags::NONE, 1);

        let update = on_area_change_like_cpp(
            &mut object,
            &area_store,
            &phase_store,
            &phase_group_store,
            &phase_info_store,
            |phase_id, _| phase_id != 20,
            [30],
            [7],
        );

        assert_eq!(update, PhaseVisibilityUpdate::new(true, true));
        assert!(!object.phase_shift().has_phase_like_cpp(99));
        assert!(!object.suppressed_phase_shift().has_phase_like_cpp(98));
        assert!(object.phase_shift().has_phase_like_cpp(10));
        assert!(object.suppressed_phase_shift().has_phase_like_cpp(20));
        assert!(object.phase_shift().has_phase_like_cpp(30));
        assert_eq!(
            object
                .phase_shift()
                .phase_ref_like_cpp(30)
                .map(|phase| phase.flags()),
            Some(PhaseFlags::COSMETIC)
        );
        assert_eq!(object.phase_shift().personal_guid_like_cpp(), personal_guid);
    }

    #[test]
    fn on_condition_change_moves_phases_between_active_and_suppressed_like_cpp() {
        let phase_store = phase_store();
        let phase_group_store = phase_group_store(&phase_store);
        let terrain_swap_store = terrain_swap_store();
        let mut object = world_object();
        let personal_guid = object.guid();
        object
            .phase_shift_mut()
            .add_phase_like_cpp(10, PhaseFlags::NONE, 2);
        object
            .phase_shift_mut()
            .add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
        object
            .suppressed_phase_shift_mut()
            .add_phase_like_cpp(30, PhaseFlags::COSMETIC, 1);

        let update = on_condition_change_like_cpp(
            &mut object,
            &phase_store,
            &phase_group_store,
            &terrain_swap_store,
            true,
            |phase_id, _| match phase_id {
                10 => Some(false),
                20 => None,
                _ => Some(true),
            },
            |phase_id, _| phase_id == 30,
            |_, _| true,
            [10],
            std::iter::empty(),
        );

        assert_eq!(update, PhaseVisibilityUpdate::new(true, true));
        assert_eq!(
            object
                .phase_shift()
                .phase_ref_like_cpp(10)
                .map(|phase| phase.references()),
            Some(1)
        );
        assert_eq!(
            object
                .suppressed_phase_shift()
                .phase_ref_like_cpp(10)
                .map(|phase| phase.references()),
            Some(1)
        );
        assert!(object.phase_shift().has_phase_like_cpp(20));
        assert_eq!(object.phase_shift().personal_guid_like_cpp(), personal_guid);
        assert!(object.phase_shift().has_phase_like_cpp(30));
        assert!(!object.suppressed_phase_shift().has_phase_like_cpp(30));
    }

    #[test]
    fn on_condition_change_moves_visible_maps_and_ui_phase_ids_like_cpp() {
        let phase_store = phase_store();
        let phase_group_store = phase_group_store(&phase_store);
        let terrain_swap_store = terrain_swap_store();
        let mut object = world_object();
        object.phase_shift_mut().add_visible_map_id_like_cpp(609, 1);
        object.phase_shift_mut().add_ui_map_phase_id_like_cpp(42, 1);
        object.phase_shift_mut().add_ui_map_phase_id_like_cpp(43, 1);
        object
            .suppressed_phase_shift_mut()
            .add_visible_map_id_like_cpp(700, 1);

        let update = on_condition_change_like_cpp(
            &mut object,
            &phase_store,
            &phase_group_store,
            &terrain_swap_store,
            false,
            |_, _| None,
            |_, _| false,
            |visible_map_id, _| visible_map_id == 700,
            std::iter::empty(),
            std::iter::empty(),
        );

        assert_eq!(update, PhaseVisibilityUpdate::new(false, true));
        assert!(!object.phase_shift().has_visible_map_id_like_cpp(609));
        assert!(
            object
                .suppressed_phase_shift()
                .has_visible_map_id_like_cpp(609)
        );
        assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(42));
        assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(43));
        assert!(object.phase_shift().has_visible_map_id_like_cpp(700));
        assert!(
            !object
                .suppressed_phase_shift()
                .has_visible_map_id_like_cpp(700)
        );
        assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(70));
    }

    #[test]
    fn on_area_change_honors_parent_sub_area_exclusions_like_cpp() {
        let area_store = area_store();
        let phase_store = phase_store();
        let phase_group_store = phase_group_store(&phase_store);
        let phase_info_store = phase_info_store(&area_store, &phase_store);
        let mut object = world_object();
        object.set_zone_and_area(100, 101);

        on_area_change_like_cpp(
            &mut object,
            &area_store,
            &phase_store,
            &phase_group_store,
            &phase_info_store,
            |_, _| true,
            [],
            [],
        );

        assert!(object.phase_shift().has_phase_like_cpp(10));
        assert!(object.phase_shift().has_phase_like_cpp(20));
        assert!(!object.phase_shift().has_phase_like_cpp(30));
    }

    #[test]
    fn inherit_phase_shift_copies_active_and_suppressed_like_cpp() {
        let mut source = world_object();
        let mut target = world_object();
        source
            .phase_shift_mut()
            .add_phase_like_cpp(10, PhaseFlags::NONE, 1);
        source
            .suppressed_phase_shift_mut()
            .add_phase_like_cpp(20, PhaseFlags::NONE, 1);

        inherit_phase_shift_like_cpp(&mut target, &source);

        assert!(target.phase_shift().has_phase_like_cpp(10));
        assert!(target.suppressed_phase_shift().has_phase_like_cpp(20));
    }

    #[test]
    fn set_visibility_flags_match_cpp_and_report_update_request() {
        let mut object = world_object();

        let update = set_always_visible_like_cpp(&mut object, true, true);
        assert_eq!(update, PhaseVisibilityUpdate::new(true, true));
        assert!(
            object
                .phase_shift()
                .flags_like_cpp()
                .contains(PhaseShiftFlags::ALWAYS_VISIBLE)
        );

        let update = set_inversed_like_cpp(&mut object, true, false);
        assert_eq!(update, PhaseVisibilityUpdate::new(false, true));
        assert!(
            object
                .phase_shift()
                .flags_like_cpp()
                .contains(PhaseShiftFlags::INVERSE)
        );
        assert!(
            object
                .phase_shift()
                .flags_like_cpp()
                .contains(PhaseShiftFlags::INVERSE_UNPHASED)
        );
        assert!(
            !object
                .phase_shift()
                .flags_like_cpp()
                .contains(PhaseShiftFlags::UNPHASED)
        );
    }

    #[test]
    fn init_db_phase_shift_uses_cpp_flags_phase_and_group_priority() {
        let phase_store = phase_store();
        let phase_group_store = phase_group_store(&phase_store);
        let mut phase_shift = PhaseShift::default();

        init_db_phase_shift_like_cpp(
            &mut phase_shift,
            &phase_store,
            &phase_group_store,
            PHASE_USE_FLAGS_ALWAYS_VISIBLE | PHASE_USE_FLAGS_INVERSE,
            20,
            7,
        );

        assert!(phase_shift.is_db_phase_shift_like_cpp());
        assert!(phase_shift.has_phase_like_cpp(20));
        assert!(!phase_shift.has_phase_like_cpp(10));
        assert_eq!(
            phase_shift.flags_like_cpp(),
            PhaseShiftFlags::ALWAYS_VISIBLE | PhaseShiftFlags::UNPHASED | PhaseShiftFlags::INVERSE
        );
    }

    #[test]
    fn init_db_phase_shift_uses_group_and_unphased_fallback_like_cpp() {
        let phase_store = phase_store();
        let phase_group_store = phase_group_store(&phase_store);
        let mut phase_shift = PhaseShift::default();

        init_db_phase_shift_like_cpp(
            &mut phase_shift,
            &phase_store,
            &phase_group_store,
            PHASE_USE_FLAGS_INVERSE,
            0,
            7,
        );

        assert!(phase_shift.has_phase_like_cpp(10));
        assert!(phase_shift.has_phase_like_cpp(20));
        assert!(!phase_shift.has_phase_like_cpp(99));
        assert_eq!(phase_shift.flags_like_cpp(), PhaseShiftFlags::INVERSE);

        init_db_phase_shift_like_cpp(
            &mut phase_shift,
            &phase_store,
            &phase_group_store,
            PHASE_USE_FLAGS_INVERSE,
            0,
            0,
        );

        assert_eq!(
            phase_shift.flags_like_cpp(),
            PhaseShiftFlags::INVERSE | PhaseShiftFlags::INVERSE_UNPHASED
        );
    }

    #[test]
    fn init_db_personal_ownership_stamps_personal_guid_like_cpp() {
        let phase_store = phase_store();
        let phase_group_store = phase_group_store(&phase_store);
        let personal_guid = ObjectGuid::create_player(1, 42);
        let mut phase_shift = PhaseShift::default();

        init_db_phase_shift_like_cpp(&mut phase_shift, &phase_store, &phase_group_store, 0, 20, 0);
        init_db_personal_ownership_like_cpp(&mut phase_shift, personal_guid);

        assert_eq!(phase_shift.personal_guid_like_cpp(), personal_guid);
    }

    #[test]
    fn init_db_visible_map_id_resets_visible_maps_only_like_cpp() {
        let terrain_swap_store = terrain_swap_store();
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_visible_map_id_like_cpp(700, 1);
        phase_shift.add_ui_map_phase_id_like_cpp(70, 1);

        init_db_visible_map_id_like_cpp(&mut phase_shift, &terrain_swap_store, 609);

        assert!(!phase_shift.has_visible_map_id_like_cpp(700));
        assert!(phase_shift.has_visible_map_id_like_cpp(609));
        assert!(phase_shift.has_ui_map_phase_id_like_cpp(70));

        init_db_visible_map_id_like_cpp(&mut phase_shift, &terrain_swap_store, -1);
        assert_eq!(phase_shift.visible_map_id_count_like_cpp(), 0);
        assert!(phase_shift.has_ui_map_phase_id_like_cpp(70));
    }

    #[test]
    fn phase_shift_change_packet_copies_phase_shift_like_cpp_send_to_player() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let personal_guid = ObjectGuid::create_player(1, 99);
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_phase_like_cpp(10, PhaseFlags::COSMETIC, 1);
        phase_shift.add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
        phase_shift.set_personal_guid_like_cpp(personal_guid);
        phase_shift.add_visible_map_id_like_cpp(609, 1);
        phase_shift.add_ui_map_phase_id_like_cpp(42, 1);

        let packet = phase_shift_change_for_player_like_cpp(player_guid, &phase_shift).unwrap();

        assert_eq!(packet.player_guid, player_guid);
        assert_eq!(
            packet.phase_shift_flags,
            phase_shift.flags_like_cpp().bits()
        );
        assert_eq!(packet.personal_guid, personal_guid);
        assert_eq!(
            packet.phases,
            vec![
                PhaseShiftDataPhase {
                    phase_flags: PhaseFlags::COSMETIC.bits(),
                    id: 10,
                },
                PhaseShiftDataPhase {
                    phase_flags: PhaseFlags::PERSONAL.bits(),
                    id: 20,
                },
            ]
        );
        assert_eq!(packet.visible_map_ids, vec![609]);
        assert!(packet.preload_map_ids.is_empty());
        assert_eq!(packet.ui_map_phase_ids, vec![42]);
    }

    #[test]
    fn party_member_phase_states_copy_phase_shift_like_cpp() {
        let personal_guid = ObjectGuid::create_player(1, 99);
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_phase_like_cpp(10, PhaseFlags::COSMETIC, 1);
        phase_shift.add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
        phase_shift.set_personal_guid_like_cpp(personal_guid);

        let states = party_member_phase_states_like_cpp(&phase_shift).unwrap();

        assert_eq!(
            states.phase_shift_flags,
            phase_shift.flags_like_cpp().bits()
        );
        assert_eq!(states.personal_guid, personal_guid);
        assert_eq!(
            states.phases,
            vec![
                PartyMemberPhase {
                    flags: u32::from(PhaseFlags::COSMETIC.bits()),
                    id: 10,
                },
                PartyMemberPhase {
                    flags: u32::from(PhaseFlags::PERSONAL.bits()),
                    id: 20,
                },
            ]
        );
    }

    #[test]
    fn format_phases_keeps_cpp_comma_suffix_and_order() {
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
        phase_shift.add_phase_like_cpp(10, PhaseFlags::COSMETIC, 1);

        assert_eq!(format_phases_like_cpp(&phase_shift), "10,20,");
    }

    #[test]
    fn print_to_chat_snapshot_matches_cpp_argument_payloads() {
        let personal_guid = ObjectGuid::create_player(1, 99);
        let mut object = world_object();
        object
            .phase_shift_mut()
            .set_flags_like_cpp(PhaseShiftFlags::ALWAYS_VISIBLE);
        object
            .phase_shift_mut()
            .set_personal_guid_like_cpp(personal_guid);
        object
            .phase_shift_mut()
            .add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
        object
            .phase_shift_mut()
            .add_phase_like_cpp(10, PhaseFlags::COSMETIC, 1);
        object.phase_shift_mut().add_visible_map_id_like_cpp(609, 1);
        object.phase_shift_mut().add_visible_map_id_like_cpp(571, 1);
        object.phase_shift_mut().add_ui_map_phase_id_like_cpp(42, 1);

        let snapshot = print_to_chat_snapshot_like_cpp(
            &object,
            |guid| (guid == personal_guid).then(|| String::from("Owner")),
            |phase_id| Some(format!("Phase {phase_id}")),
            "Cosmetic",
            "Personal",
        );

        assert_eq!(snapshot.flags, object.phase_shift().flags_like_cpp().bits());
        assert_eq!(snapshot.personal_guid, personal_guid);
        assert_eq!(snapshot.personal_owner_name, "Owner");
        assert_eq!(
            snapshot.phases,
            Some(String::from(
                "\r\n   10 (Phase 10) (Cosmetic)\r\n   20 (Phase 20) (Personal)"
            ))
        );
        assert_eq!(snapshot.visible_map_ids, Some(String::from("571, 609, ")));
        assert_eq!(snapshot.ui_map_phase_ids, Some(String::from("42, ")));
    }

    #[test]
    fn print_to_chat_snapshot_uses_cpp_missing_owner_and_phase_name_fallbacks() {
        let mut object = world_object();
        object
            .phase_shift_mut()
            .add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);

        let snapshot =
            print_to_chat_snapshot_like_cpp(&object, |_| None, |_| None, "Cosmetic", "Personal");

        assert_eq!(snapshot.personal_owner_name, "N/A");
        assert_eq!(
            snapshot.phases,
            Some(String::from("\r\n   20 (Unknown Name) (Personal)"))
        );
        assert_eq!(snapshot.visible_map_ids, None);
        assert_eq!(snapshot.ui_map_phase_ids, None);
    }

    #[test]
    fn phase_shift_change_packet_rejects_values_that_do_not_fit_cpp_wire_types() {
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_phase_like_cpp(u16::MAX as u32 + 1, PhaseFlags::NONE, 1);

        let error = match phase_shift_change_for_player_like_cpp(ObjectGuid::EMPTY, &phase_shift) {
            Ok(_) => panic!("overflowing phase id must be rejected"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            PhaseShiftPacketBuildError::PhaseIdOutOfRange(u16::MAX as u32 + 1)
        );
    }
}
