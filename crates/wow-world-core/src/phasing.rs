use std::{error::Error, fmt};
use wow_entities::PhaseShift;
use wow_packet::packets::party::{PartyMemberPhase, PartyMemberPhaseStates};

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
