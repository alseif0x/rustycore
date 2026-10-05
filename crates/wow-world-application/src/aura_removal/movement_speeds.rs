// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{AuraRemovalCxLikeCpp, RemovedAuraLikeCpp};
use wow_entities::RepresentedAuraEffectLikeCpp;

impl AuraRemovalCxLikeCpp<'_> {
    pub(super) fn remove_movement_speed_phase_like_cpp(&mut self, removed: &RemovedAuraLikeCpp) {
        let aura = &removed.aura;
        if matches!(
            aura.represented_effect,
            Some(
                RepresentedAuraEffectLikeCpp::MountedSpeed
                    | RepresentedAuraEffectLikeCpp::Speed
                    | RepresentedAuraEffectLikeCpp::SpeedAlways
                    | RepresentedAuraEffectLikeCpp::SpeedNotStack
                    | RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed
                    | RepresentedAuraEffectLikeCpp::DecreaseSpeed
                    | RepresentedAuraEffectLikeCpp::MinimumSpeed
                    | RepresentedAuraEffectLikeCpp::MinimumSpeedRate
                    | RepresentedAuraEffectLikeCpp::MountedSpeedAlways
                    | RepresentedAuraEffectLikeCpp::MountedSpeedNotStack
            )
        ) {
            self.mount
                .recompute_represented_run_speed_rate_like_cpp(&self.player);
        }
        if matches!(
            aura.represented_effect,
            Some(
                RepresentedAuraEffectLikeCpp::MountedFlightSpeed
                    | RepresentedAuraEffectLikeCpp::Fly
                    | RepresentedAuraEffectLikeCpp::FlightSpeed
                    | RepresentedAuraEffectLikeCpp::VehicleFlightSpeed
                    | RepresentedAuraEffectLikeCpp::MountedFlightSpeedAlways
                    | RepresentedAuraEffectLikeCpp::FlightSpeedNotStack
            )
        ) {
            if matches!(
                aura.represented_effect,
                Some(
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed
                        | RepresentedAuraEffectLikeCpp::Fly
                )
            ) {
                self.mount
                    .update_flight_flags_for_aura_like_cpp(&self.player, false);
            }
            self.mount
                .recompute_represented_flight_speed_rate_like_cpp(&self.player);
        }
        if matches!(
            aura.represented_effect,
            Some(RepresentedAuraEffectLikeCpp::SwimSpeed)
        ) {
            self.mount
                .recompute_represented_swim_speed_rate_like_cpp(&self.player);
        }
        if matches!(
            aura.represented_effect,
            Some(
                RepresentedAuraEffectLikeCpp::DecreaseSpeed
                    | RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed
            )
        ) {
            self.mount
                .recompute_represented_swim_speed_rate_like_cpp(&self.player);
            self.mount
                .recompute_represented_flight_speed_rate_like_cpp(&self.player);
        }
        if aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::DecreaseSpeed) {
            self.mount
                .recompute_represented_backward_speed_rates_like_cpp(&self.player);
        }
    }
}
