pub mod defines;
pub mod generator;
pub mod generators;
pub mod motion_master;
pub mod path_generator;
pub mod spline;

pub use defines::{
    CONTACT_DISTANCE_LIKE_CPP, ChaseAngle, ChaseRange, JumpArrivalCastArgs,
    MovementWalkRunSpeedSelectionMode, RotateDirection, normalize_orientation_like_cpp,
};
pub use generator::{
    MovementGenerator, MovementGeneratorFlags, MovementGeneratorMode, MovementGeneratorPriority,
    MovementGeneratorState, MovementGeneratorType, MovementSlot,
};
pub use generators::{
    AbstractFollower, AbstractFollowerEvent, AssistanceDistractFinalizeAction,
    AssistanceDistractMovementGenerator, AssistanceMovementFinalize, AssistanceMovementGenerator,
    CHASE_RANGE_CHECK_INTERVAL_MS_LIKE_CPP, CONFUSED_LOS_RETRY_MS_LIKE_CPP,
    CONFUSED_PATH_LENGTH_LIMIT_LIKE_CPP, CONFUSED_PATH_RETRY_MS_LIKE_CPP,
    CONFUSED_RANDOM_DELAY_MAX_MS_LIKE_CPP, CONFUSED_RANDOM_DELAY_MIN_MS_LIKE_CPP,
    CONFUSED_RANDOM_DISTANCE_OFFSET_LIKE_CPP, CONFUSED_RANDOM_DISTANCE_SCALE_LIKE_CPP,
    CREATURE_FAMILY_ASSISTANCE_DELAY_MS_LIKE_CPP, ChaseFinalizeAction, ChaseLaunchPlan,
    ChaseMovementAction, ChaseMovementGenerator, ChaseMovementInform, ChaseRangeBounds,
    ChaseUnitSnapshot, ChaseWalkMode, ConfusedDestinationPlan, ConfusedFinalizeAction,
    ConfusedLaunchPlan, ConfusedMovementAction, ConfusedMovementGenerator, ConfusedPathResult,
    ConfusedUnitSnapshot, DistractFacingSpline, DistractFinalizeAction, DistractInitializeAction,
    DistractMovementGenerator, EVENT_CHARGE_LIKE_CPP, EVENT_CHARGE_PREPATH_LIKE_CPP,
    FLEEING_LOS_RETRY_MS_LIKE_CPP, FLEEING_PATH_LENGTH_LIMIT_LIKE_CPP,
    FLEEING_PATH_RETRY_MS_LIKE_CPP, FLEEING_RANDOM_DELAY_MAX_MS_LIKE_CPP,
    FLEEING_RANDOM_DELAY_MIN_MS_LIKE_CPP, FLIGHT_SKIP_SPLINE_POINT_DISTANCE_SQ_LIKE_CPP,
    FLIGHT_TIMEDIFF_NEXT_WP_MS_LIKE_CPP, FLIGHT_TRAVEL_UPDATE_MS_LIKE_CPP,
    FOLLOW_CHECK_INTERVAL_MS_LIKE_CPP, FOLLOW_RANGE_TOLERANCE_LIKE_CPP,
    FORMATION_MOVEMENT_INTERVAL_MS_LIKE_CPP, FORMATION_PREDICTED_SPLINE_SECONDS_LIKE_CPP,
    FleeingDestinationPlan, FleeingFinalizeAction, FleeingLaunchPlan, FleeingMovementAction,
    FleeingMovementGenerator, FleeingPathResult, FleeingRandomInputs, FleeingUnitSnapshot,
    FlightEndGridInfo, FlightFinalizeAction, FlightFinalizeContext, FlightLaunchPlan,
    FlightMovementAction, FlightPathEvent, FlightPathMovementGenerator, FlightPathSwitchAction,
    FlightUpdateAction, FollowFinalizeAction, FollowLaunchPlan, FollowMovementAction,
    FollowMovementGenerator, FollowMovementInform, FollowUnitSnapshot, FormationArrivalAction,
    FormationFinalizeAction, FormationLaunchPlan, FormationMovementAction,
    FormationMovementGenerator, FormationMovementInform, FormationUnitSnapshot,
    GenericArrivalSpell, GenericMovementFinalize, GenericMovementGenerator, GenericMovementInform,
    GenericSplineInitializer, HOME_CLEAR_ON_FINALIZE_MASK_LIKE_CPP,
    HOME_CLEAR_ON_TARGET_MASK_LIKE_CPP, HomeFinalizeAction, HomeLaunchPlan, HomeMovementAction,
    HomeMovementGenerator, HomeUnitSnapshot, IdleMovementGenerator, MAX_QUIET_DISTANCE_LIKE_CPP,
    MIN_QUIET_DISTANCE_LIKE_CPP, PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP,
    PLAYER_FLIGHT_SPEED_LIKE_CPP, PointMovementAction, PointMovementFinalize,
    PointMovementGenerator, PointMovementInform, PointMovementLaunch, RANDOM_LOS_RETRY_MS_LIKE_CPP,
    RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP, RANDOM_PATH_RETRY_MS_LIKE_CPP, RANDOM_PAUSE_MAX_MS_LIKE_CPP,
    RANDOM_PAUSE_MIN_MS_LIKE_CPP, RANDOM_WANDER_STEPS_MAX_LIKE_CPP,
    RANDOM_WANDER_STEPS_MIN_LIKE_CPP, RandomDestinationPlan, RandomFinalizeAction,
    RandomLaunchPlan, RandomMovementAction, RandomMovementGenerator, RandomMovementInform,
    RandomPathResult, RandomUnitSnapshot, RotateFacingSpline, RotateMovementGenerator,
    RotateMovementInform, RotateMovementUpdate, SplineChainFinalizeAction, SplineChainInform,
    SplineChainLaunchPlan, SplineChainLink, SplineChainMovementAction,
    SplineChainMovementGenerator, SplineChainResumeInfo, TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP,
    TAXI_PATH_NODE_FLAG_TELEPORT_LIKE_CPP, TaxiNodeChangeInfo, TaxiPathNode, TaxiPathSegment,
    TimedFleeingFinalizeAction, TimedFleeingInform, TimedFleeingMovementGenerator,
    UNIT_FLAG_CAN_SWIM_LIKE_CPP, UNIT_FLAG_CONFUSED_LIKE_CPP, UNIT_FLAG_FLEEING_LIKE_CPP,
    UNIT_FLAG_ON_TAXI_LIKE_CPP, UNIT_FLAG_REMOVE_CLIENT_CONTROL_LIKE_CPP,
    UNIT_STATE_ALL_ERASABLE_LIKE_CPP, UNIT_STATE_CHASE_LIKE_CPP, UNIT_STATE_CHASE_MOVE_LIKE_CPP,
    UNIT_STATE_CONFUSED_LIKE_CPP, UNIT_STATE_CONFUSED_MOVE_LIKE_CPP,
    UNIT_STATE_DISTRACTED_LIKE_CPP, UNIT_STATE_FLEEING_LIKE_CPP, UNIT_STATE_FLEEING_MOVE_LIKE_CPP,
    UNIT_STATE_FOLLOW_FORMATION_LIKE_CPP, UNIT_STATE_FOLLOW_FORMATION_MOVE_LIKE_CPP,
    UNIT_STATE_FOLLOW_LIKE_CPP, UNIT_STATE_FOLLOW_MOVE_LIKE_CPP,
    UNIT_STATE_FORMATION_NOT_MOVE_LIKE_CPP, UNIT_STATE_HOME_INTERRUPT_MASK_LIKE_CPP,
    UNIT_STATE_IN_FLIGHT_LIKE_CPP, UNIT_STATE_RANDOM_LOST_CONTROL_LIKE_CPP,
    UNIT_STATE_RANDOM_NOT_MOVE_LIKE_CPP, UNIT_STATE_ROAMING_LIKE_CPP,
    UNIT_STATE_ROAMING_MOVE_LIKE_CPP, UNIT_STATE_ROTATING_LIKE_CPP,
    UNIT_STATE_SPLINE_CHAIN_ROAMING_LIKE_CPP, UNIT_STATE_SPLINE_CHAIN_ROAMING_MOVE_LIKE_CPP,
    UNIT_STATE_WAYPOINT_LOST_CONTROL_LIKE_CPP, UNIT_STATE_WAYPOINT_NOT_MOVE_LIKE_CPP,
    UNIT_STATE_WAYPOINT_ROAMING_LIKE_CPP, UNIT_STATE_WAYPOINT_ROAMING_MOVE_LIKE_CPP,
    WAYPOINT_BLOCKED_RETRY_MS_LIKE_CPP, WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP,
    WAYPOINT_PATH_FLAG_FOLLOW_PATH_BACKWARDS_MINIMUM_NODES_LIKE_CPP,
    WAYPOINT_RESUME_GUARD_MS_LIKE_CPP, WaypointAnimation, WaypointArrivalAction,
    WaypointCurrentInfo, WaypointFinalizeAction, WaypointInform, WaypointLaunchPlan,
    WaypointMoveType, WaypointMovementAction, WaypointMovementGenerator, WaypointNode,
    WaypointPath, WaypointPathEnded, WaypointRandomAtPathEnd, WaypointStarted,
    WaypointUnitSnapshot, chase_position_okay_like_cpp, compute_confused_destination_like_cpp,
    compute_flee_destination_like_cpp, compute_random_destination_like_cpp,
    formation_move_position_like_cpp, is_node_included_in_shortened_path_like_cpp,
    position_okay_like_cpp, random_walk_like_cpp, selected_relative_angle_like_cpp,
};
pub use motion_master::{
    DelayedAction, DelayedActionQueue, MotionMaster, MotionMasterDelayedActionType,
    MotionMasterFlags, ResolvedDelayedAction, StopOnDeathAction,
};
pub use path_generator::{
    INVALID_POLYREF_LIKE_CPP, MAX_PATH_LENGTH_LIKE_CPP, MAX_POINT_PATH_LENGTH_LIKE_CPP,
    PathGenerator, PathType, SMOOTH_PATH_SLOP_LIKE_CPP, SMOOTH_PATH_STEP_SIZE_LIKE_CPP,
    VERTEX_SIZE_LIKE_CPP, distance_3d, distance_3d_sq, in_range_like_cpp, in_range_yzx_like_cpp,
};
pub use spline::{
    AnimTierTransition, FacingInfo, JumpSpeeds, MonsterMovePathData, MonsterMoveType, MoveSpline,
    MoveSplineFlag, MoveSplineInit, MoveSplineInitArgs, MoveSplineLaunchError,
    MoveSplineLaunchInput, MoveSplineLaunchResult, MoveSplineStopInput, MoveSplineStopResult,
    MoveSplineValidationError, SpellEffectExtraData, SplineUpdateResult,
    calculate_jump_speeds_like_cpp, compute_fall_elevation, compute_fall_time,
    compute_jump_max_height_like_cpp,
};
pub use wow_data_model::jump_charge::{JumpChargeParams, JumpChargeSpec};
