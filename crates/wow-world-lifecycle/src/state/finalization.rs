use wow_world_core::session::{HubMut, HubRef};

use super::SessionLifecycleState;
use crate::{
    FinalizationDisposition, FinalizationOutcome, FinalizationReport, FinalizationStep,
    SessionFinalization,
};

impl SessionLifecycleState {
    pub fn finalization(&self) -> Option<&SessionFinalization> {
        self.finalization.as_ref()
    }

    pub fn finalization_mut(&mut self) -> Option<&mut SessionFinalization> {
        self.finalization.as_mut()
    }

    pub fn install_finalization(&mut self, finalization: SessionFinalization) {
        self.finalization = Some(finalization);
    }

    pub fn finalization_result(&mut self, hub: &mut HubMut<'_>) -> FinalizationReport {
        let report = self
            .finalization
            .as_ref()
            .expect("admitted finalization")
            .report();
        if report.disposition == FinalizationDisposition::RetainAndEscalate {
            hub.core
                .kick("session finalization retained an unresolved obligation");
        }
        report
    }

    pub fn finalization_identity_is_current(&self, hub: HubRef<'_>) -> bool {
        let report = self.finalization.as_ref().unwrap().report();
        if report.outcome(FinalizationStep::Retirement) == FinalizationOutcome::Applied {
            return hub.core.player_handle_like_cpp.is_none();
        }
        report.player == hub.core.player_handle_like_cpp
            && report
                .player
                .is_none_or(|handle| hub.core.player_guid() == Some(handle.guid()))
            && !(hub.core.player_guid().is_none() && hub.core.player_handle_like_cpp.is_some())
    }
}
