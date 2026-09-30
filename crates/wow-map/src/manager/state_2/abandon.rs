//! Recover the same admitted plan when the existing abandon gate rejects it.
use super::*;

impl MapManager {
    /// Success changes only Map coordination to Idle, not APP ticket settlement.
    pub fn try_abandon_tick(
        &mut self,
        plan: MapTickPlanLikeCpp,
    ) -> Result<(), (MapTickResumeLikeCpp, MapTickPlanLikeCpp)> {
        if !self.can_resume_tick(&plan) {
            let rejected = MapTickResumeLikeCpp::Rejected {
                state: self.tick_coordination_like_cpp,
                plan_epoch: plan.epoch,
            };
            return Err((rejected, plan));
        }
        self.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Idle;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
