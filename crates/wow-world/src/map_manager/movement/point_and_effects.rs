//! Point and effects operations of movement.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl WorldCreature {
    pub fn begin_point_movement_like_cpp(
        &mut self,
        movement_id: u32,
        dst: Position,
        can_move: bool,
    ) -> Option<(Position, MoveSpline)> {
        if movement_id == EVENT_CHARGE_PREPATH {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_charge(movement_id);
        } else {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_point(movement_id);
        }

        let action = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion.active_generators.iter_mut().find(|generator| {
                generator.kind == MovementGeneratorKind::Point
                    && generator.movement_id == movement_id
            })?;
            generator.initialize_point_like_cpp(can_move)
        };

        match action {
            PointMovementAction::LaunchSpline => self.begin_move_spline_like_cpp(dst),
            PointMovementAction::MarkRoamingMove => {
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::ROAMING_MOVE.bits());
                None
            }
            PointMovementAction::StopMoving => {
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                None
            }
            _ => None,
        }
    }

    pub fn finalize_point_movement_like_cpp(
        &mut self,
        active: bool,
        movement_inform: bool,
    ) -> Option<PointMovementInform> {
        let finalize = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Point)?;
            generator.finalize_point_like_cpp(active, movement_inform)
        };
        if finalize.clear_roaming_move {
            self.creature
                .unit_mut()
                .clear_unit_state(UnitState::ROAMING_MOVE.bits());
        }
        if let Some(inform) = finalize.inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        finalize.inform
    }

    pub fn begin_distract_movement_like_cpp(
        &mut self,
        timer_ms: u32,
        orientation: f32,
    ) -> Option<(DistractMovementAction, Position, MoveSpline)> {
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .move_distract_like_cpp(timer_ms);

        let owner_is_standing = self.creature.unit().is_stand_state_like_cpp();
        let action = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Distract)?;
            generator.initialize_distract_like_cpp(owner_is_standing)
        };
        if action.stand_up {
            self.creature
                .unit_mut()
                .set_stand_state_like_cpp(UnitStandStateType::Stand);
        }
        let (from, spline) = self.begin_facing_spline_like_cpp(orientation)?;
        Some((action, from, spline))
    }

    pub fn tick_rotate_movement_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> Option<(RotateMovementUpdate, MoveSpline)> {
        let update = {
            let current_orientation = self.position().orientation;
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Rotate)?;
            generator.update_rotate_like_cpp(true, diff_ms, current_orientation)
        };
        let (_, spline) = self.begin_facing_spline_like_cpp(update.facing_angle?)?;
        Some((update, spline))
    }

    pub fn finalize_distract_movement_like_cpp(&mut self, movement_inform: bool) -> bool {
        let finalize = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let Some(generator) = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Distract)
            else {
                return false;
            };
            generator.finalize_distract_like_cpp(movement_inform, true)
        };

        if finalize.set_home_orientation {
            let current = self.position();
            let home = self.home_position();
            self.creature.set_ai_position(Position::new(
                current.x,
                current.y,
                current.z,
                home.orientation,
            ));
        }
        finalize.set_home_orientation
    }

    pub fn finalize_rotate_movement_like_cpp(
        &mut self,
        movement_inform: bool,
    ) -> Option<PointMovementInform> {
        let inform = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Rotate)?;
            generator
                .finalize_rotate_like_cpp(movement_inform, true)
                .inform
        };
        if let Some(inform) = inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        inform
    }

    pub fn finalize_generic_movement_like_cpp(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        movement_inform: bool,
    ) -> Option<GenericMovementInform> {
        let inform = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == kind && generator.movement_id == movement_id)?;
            generator.finalize_generic_like_cpp(movement_inform)
        };
        if let Some(inform) = inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        inform
    }
}
