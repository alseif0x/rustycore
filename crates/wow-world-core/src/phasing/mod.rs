use std::{error::Error, fmt};
use wow_constants::{PhaseFlags, PhaseShiftFlags};
use wow_data::{PhaseGroupStore, PhaseStore, TerrainSwapStore};
use wow_entities::PhaseShift;
use wow_packet::packets::party::{PartyMemberPhase, PartyMemberPhaseStates};

pub const PHASE_USE_FLAGS_ALWAYS_VISIBLE: u8 = 0x01;
pub const PHASE_USE_FLAGS_INVERSE: u8 = 0x02;
const DEFAULT_PHASE: u32 = 169;

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

mod visibility;
pub use visibility::*;
