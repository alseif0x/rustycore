use wow_core::{ObjectGuid, Position};

use crate::{
    MoveSpline, MoveSplineInit, MoveSplineLaunchError, MoveSplineLaunchInput,
    MoveSplineLaunchResult, MovementGenerator, MovementGeneratorFlags, MovementGeneratorMode,
    MovementGeneratorPriority, MovementGeneratorState, MovementGeneratorType,
};

pub const UNIT_STATE_ROAMING_LIKE_CPP: u32 = 0x0000_0010;

pub type GenericSplineInitializer = Box<dyn FnOnce(&mut MoveSplineInit) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericArrivalSpell {
    pub spell_id: u32,
    pub target: ObjectGuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericMovementInform {
    pub movement_type: MovementGeneratorType,
    pub movement_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericMovementFinalize {
    pub arrival_spell: Option<GenericArrivalSpell>,
    pub movement_inform: Option<GenericMovementInform>,
}

pub struct GenericMovementGenerator {
    state: MovementGeneratorState,
    initializer: Option<GenericSplineInitializer>,
    movement_type: MovementGeneratorType,
    point_id: u32,
    remaining_duration_ms: i32,
    arrival_spell: Option<GenericArrivalSpell>,
    pub last_launch_result: Option<MoveSplineLaunchResult>,
    pub last_launch_error: Option<MoveSplineLaunchError>,
    pub finalize_result: Option<GenericMovementFinalize>,
}

impl GenericMovementGenerator {
    #[must_use]
    pub fn new(
        initializer: impl FnOnce(&mut MoveSplineInit) + Send + Sync + 'static,
        movement_type: MovementGeneratorType,
        point_id: u32,
    ) -> Self {
        Self::new_with_arrival_spell(initializer, movement_type, point_id, None)
    }

    #[must_use]
    pub fn new_with_arrival_spell(
        initializer: impl FnOnce(&mut MoveSplineInit) + Send + Sync + 'static,
        movement_type: MovementGeneratorType,
        point_id: u32,
        arrival_spell: Option<GenericArrivalSpell>,
    ) -> Self {
        Self {
            state: MovementGeneratorState {
                mode: MovementGeneratorMode::Default,
                priority: MovementGeneratorPriority::Normal,
                flags: MovementGeneratorFlags::INITIALIZATION_PENDING,
                base_unit_state: UNIT_STATE_ROAMING_LIKE_CPP,
            },
            initializer: Some(Box::new(initializer)),
            movement_type,
            point_id,
            remaining_duration_ms: 0,
            arrival_spell,
            last_launch_result: None,
            last_launch_error: None,
            finalize_result: None,
        }
    }

    #[must_use]
    pub const fn movement_type(&self) -> MovementGeneratorType {
        self.movement_type
    }

    #[must_use]
    pub const fn point_id(&self) -> u32 {
        self.point_id
    }

    #[must_use]
    pub const fn remaining_duration_ms(&self) -> i32 {
        self.remaining_duration_ms
    }

    pub fn initialize_with_spline_like_cpp(
        &mut self,
        move_spline: &mut MoveSpline,
        launch_input: MoveSplineLaunchInput,
        spline_id: u32,
    ) -> Result<MoveSplineLaunchResult, MoveSplineLaunchError> {
        if self.has_flag(MovementGeneratorFlags::DEACTIVATED)
            && !self.has_flag(MovementGeneratorFlags::INITIALIZATION_PENDING)
        {
            self.remove_flag(MovementGeneratorFlags::DEACTIVATED);
            self.add_flag(MovementGeneratorFlags::FINALIZED);
            return Ok(MoveSplineLaunchResult {
                real_position: launch_input
                    .active_spline_position
                    .unwrap_or(launch_input.current_position),
                movement_flags: launch_input.movement_flags,
                duration_ms: move_spline.duration_ms(),
            });
        }

        self.remove_flag(
            MovementGeneratorFlags::INITIALIZATION_PENDING | MovementGeneratorFlags::DEACTIVATED,
        );
        self.add_flag(MovementGeneratorFlags::INITIALIZED);

        let mut init = MoveSplineInit::new(spline_id);
        if let Some(initializer) = self.initializer.take() {
            initializer(&mut init);
        }
        match init.launch(move_spline, launch_input) {
            Ok(result) => {
                self.remaining_duration_ms = result.duration_ms;
                self.last_launch_result = Some(result);
                self.last_launch_error = None;
                Ok(result)
            }
            Err(error) => {
                self.last_launch_result = None;
                self.last_launch_error = Some(error);
                Err(error)
            }
        }
    }

    pub fn reset_with_spline_like_cpp(
        &mut self,
        move_spline: &mut MoveSpline,
        launch_input: MoveSplineLaunchInput,
        spline_id: u32,
    ) -> Result<MoveSplineLaunchResult, MoveSplineLaunchError> {
        self.initialize_with_spline_like_cpp(move_spline, launch_input, spline_id)
    }

    pub fn update_with_spline_like_cpp(
        &mut self,
        owner_exists: bool,
        diff_ms: u32,
        move_spline: &MoveSpline,
    ) -> bool {
        if !owner_exists || self.has_flag(MovementGeneratorFlags::FINALIZED) {
            return false;
        }

        if !move_spline.is_cyclic() {
            self.remaining_duration_ms = self.remaining_duration_ms.saturating_sub(diff_ms as i32);
        }

        if self.remaining_duration_ms <= 0 || move_spline.finalized() {
            self.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
            return false;
        }

        true
    }

    pub fn finalize_with_owner_like_cpp(
        &mut self,
        movement_inform: bool,
        owner_is_creature: bool,
    ) -> GenericMovementFinalize {
        self.add_flag(MovementGeneratorFlags::FINALIZED);
        let result = if movement_inform && self.has_flag(MovementGeneratorFlags::INFORM_ENABLED) {
            GenericMovementFinalize {
                arrival_spell: self.arrival_spell,
                movement_inform: owner_is_creature.then_some(GenericMovementInform {
                    movement_type: self.movement_type,
                    movement_id: self.point_id,
                }),
            }
        } else {
            GenericMovementFinalize {
                arrival_spell: None,
                movement_inform: None,
            }
        };
        self.finalize_result = Some(result);
        result
    }
}

impl MovementGenerator for GenericMovementGenerator {
    fn state(&self) -> &MovementGeneratorState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut MovementGeneratorState {
        &mut self.state
    }

    fn kind(&self) -> MovementGeneratorType {
        self.movement_type
    }

    fn initialize(&mut self) {
        if self.has_flag(MovementGeneratorFlags::DEACTIVATED)
            && !self.has_flag(MovementGeneratorFlags::INITIALIZATION_PENDING)
        {
            self.remove_flag(MovementGeneratorFlags::DEACTIVATED);
            self.add_flag(MovementGeneratorFlags::FINALIZED);
            return;
        }
        self.remove_flag(
            MovementGeneratorFlags::INITIALIZATION_PENDING | MovementGeneratorFlags::DEACTIVATED,
        );
        self.add_flag(MovementGeneratorFlags::INITIALIZED);
    }

    fn reset(&mut self) {
        self.initialize();
    }

    fn update(&mut self, diff_ms: u32) -> bool {
        if self.has_flag(MovementGeneratorFlags::FINALIZED) {
            return false;
        }
        self.remaining_duration_ms = self.remaining_duration_ms.saturating_sub(diff_ms as i32);
        if self.remaining_duration_ms <= 0 {
            self.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
            false
        } else {
            true
        }
    }

    fn deactivate(&mut self) {
        self.add_flag(MovementGeneratorFlags::DEACTIVATED);
    }

    fn finalize(&mut self, _active: bool, movement_inform: bool) {
        self.finalize_with_owner_like_cpp(movement_inform, false);
    }

    fn reset_position(&self) -> Option<(f32, f32, f32)> {
        self.last_launch_result.map(|result| {
            let Position { x, y, z, .. } = result.real_position;
            (x, y, z)
        })
    }
}

#[cfg(test)]
mod tests {
    use wow_constants::movement::MovementFlag;

    use super::*;

    fn launch_input() -> MoveSplineLaunchInput {
        MoveSplineLaunchInput {
            current_position: Position::new(0.0, 0.0, 0.0, 0.0),
            active_spline_position: None,
            movement_flags: MovementFlag::empty(),
            selected_speed: 4.0,
            run_speed: 7.0,
            assistance_speed_factor: 1.0,
            on_transport: false,
        }
    }

    #[test]
    fn generic_movement_generator_matches_cpp_constructor_shape() {
        let generator = GenericMovementGenerator::new(
            |init| init.move_to(Position::new(10.0, 0.0, 0.0, 0.0)),
            MovementGeneratorType::Effect,
            11,
        );
        assert_eq!(generator.kind(), MovementGeneratorType::Effect);
        assert_eq!(generator.movement_type(), MovementGeneratorType::Effect);
        assert_eq!(generator.point_id(), 11);
        assert_eq!(generator.state().mode, MovementGeneratorMode::Default);
        assert_eq!(
            generator.state().priority,
            MovementGeneratorPriority::Normal
        );
        assert_eq!(
            generator.state().flags,
            MovementGeneratorFlags::INITIALIZATION_PENDING
        );
        assert_eq!(
            generator.state().base_unit_state,
            UNIT_STATE_ROAMING_LIKE_CPP
        );
        assert_eq!(generator.remaining_duration_ms(), 0);
    }

    #[test]
    fn generic_initialize_launches_spline_and_update_uses_non_cyclic_duration_like_cpp() {
        let mut generator = GenericMovementGenerator::new(
            |init| init.move_to(Position::new(10.0, 0.0, 0.0, 0.0)),
            MovementGeneratorType::Effect,
            11,
        );
        let mut spline = MoveSpline::new();
        let result = generator
            .initialize_with_spline_like_cpp(&mut spline, launch_input(), 77)
            .expect("launch");
        assert!(generator.has_flag(MovementGeneratorFlags::INITIALIZED));
        assert_eq!(spline.id(), 77);
        assert_eq!(generator.remaining_duration_ms(), result.duration_ms);
        assert!(generator.update_with_spline_like_cpp(true, 1, &spline));
        assert_eq!(generator.remaining_duration_ms(), result.duration_ms - 1);
        assert!(!generator.update_with_spline_like_cpp(true, result.duration_ms as u32, &spline));
        assert!(generator.has_flag(MovementGeneratorFlags::INFORM_ENABLED));
    }

    #[test]
    fn generic_update_keeps_cyclic_spline_running_until_finalized_like_cpp() {
        let mut generator = GenericMovementGenerator::new(
            |init| {
                init.move_by_path(
                    [
                        Position::new(0.0, 0.0, 0.0, 0.0),
                        Position::new(10.0, 0.0, 0.0, 0.0),
                        Position::new(10.0, 10.0, 0.0, 0.0),
                    ],
                    0,
                );
                init.set_cyclic();
            },
            MovementGeneratorType::Effect,
            11,
        );
        let mut spline = MoveSpline::new();
        let result = generator
            .initialize_with_spline_like_cpp(&mut spline, launch_input(), 77)
            .expect("launch");
        assert!(spline.is_cyclic());
        assert!(generator.update_with_spline_like_cpp(
            true,
            (result.duration_ms * 2) as u32,
            &spline
        ));
        assert_eq!(generator.remaining_duration_ms(), result.duration_ms);

        let mut stop_init = MoveSplineInit::new(78);
        stop_init.args.path.push(Position::new(0.0, 0.0, 0.0, 0.0));
        stop_init.args.flags = crate::MoveSplineFlag::DONE;
        spline.initialize(&stop_init.args).expect("stop");
        assert!(!generator.update_with_spline_like_cpp(true, 1, &spline));
        assert!(generator.has_flag(MovementGeneratorFlags::INFORM_ENABLED));
    }

    #[test]
    fn generic_deactivated_initialize_finalizes_instead_of_resuming_like_cpp() {
        let mut generator = GenericMovementGenerator::new(
            |init| init.move_to(Position::new(10.0, 0.0, 0.0, 0.0)),
            MovementGeneratorType::Effect,
            11,
        );
        generator.initialize();
        generator.deactivate();
        let mut spline = MoveSpline::new();
        generator
            .initialize_with_spline_like_cpp(&mut spline, launch_input(), 77)
            .expect("resume fallback");
        assert!(!generator.has_flag(MovementGeneratorFlags::DEACTIVATED));
        assert!(generator.has_flag(MovementGeneratorFlags::FINALIZED));
        assert!(!spline.initialized());
    }

    #[test]
    fn generic_finalize_emits_arrival_spell_and_creature_inform_like_cpp() {
        let target = ObjectGuid::create_uniq(0x1234);
        let mut generator = GenericMovementGenerator::new_with_arrival_spell(
            |init| init.move_to(Position::new(10.0, 0.0, 0.0, 0.0)),
            MovementGeneratorType::Effect,
            11,
            Some(GenericArrivalSpell {
                spell_id: 123,
                target,
            }),
        );
        generator.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
        let result = generator.finalize_with_owner_like_cpp(true, true);
        assert_eq!(
            result.arrival_spell,
            Some(GenericArrivalSpell {
                spell_id: 123,
                target,
            })
        );
        assert_eq!(
            result.movement_inform,
            Some(GenericMovementInform {
                movement_type: MovementGeneratorType::Effect,
                movement_id: 11,
            })
        );

        let mut no_inform = GenericMovementGenerator::new(
            |init| init.move_to(Position::new(10.0, 0.0, 0.0, 0.0)),
            MovementGeneratorType::Effect,
            11,
        );
        assert_eq!(
            no_inform.finalize_with_owner_like_cpp(true, true),
            GenericMovementFinalize {
                arrival_spell: None,
                movement_inform: None,
            }
        );
    }
}
