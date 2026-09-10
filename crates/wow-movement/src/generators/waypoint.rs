use wow_core::Position;

use crate::{
    MovementGenerator, MovementGeneratorFlags, MovementGeneratorMode, MovementGeneratorPriority,
    MovementGeneratorState, MovementGeneratorType, MovementWalkRunSpeedSelectionMode,
};

pub const WAYPOINT_PATH_FLAG_FOLLOW_PATH_BACKWARDS_MINIMUM_NODES_LIKE_CPP: usize = 2;
pub const WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP: i32 = 1_000;
pub const WAYPOINT_BLOCKED_RETRY_MS_LIKE_CPP: i32 = 1_000;
pub const WAYPOINT_RESUME_GUARD_MS_LIKE_CPP: i32 = 1;
pub const UNIT_STATE_WAYPOINT_ROAMING_LIKE_CPP: u32 = 0x0000_0010;
pub const UNIT_STATE_WAYPOINT_ROAMING_MOVE_LIKE_CPP: u32 = 0x0080_0000;
pub const UNIT_STATE_WAYPOINT_NOT_MOVE_LIKE_CPP: u32 = 0x0000_0409;
pub const UNIT_STATE_WAYPOINT_LOST_CONTROL_LIKE_CPP: u32 = 0x0007_008c;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum WaypointMoveType {
    #[default]
    Walk = 0,
    Run = 1,
    Land = 2,
    TakeOff = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaypointAnimation {
    Ground,
    Hover,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaypointNode {
    pub id: u32,
    pub position: Position,
    pub orientation: Option<f32>,
    pub delay_ms: i32,
    pub move_type: WaypointMoveType,
}

impl WaypointNode {
    #[must_use]
    pub const fn new(id: u32, x: f32, y: f32, z: f32) -> Self {
        Self {
            id,
            position: Position {
                x,
                y,
                z,
                orientation: 0.0,
            },
            orientation: None,
            delay_ms: 0,
            move_type: WaypointMoveType::Walk,
        }
    }

    #[must_use]
    pub const fn with_delay(mut self, delay_ms: i32) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    #[must_use]
    pub const fn with_orientation(mut self, orientation: f32) -> Self {
        self.orientation = Some(orientation);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaypointPath {
    pub id: u32,
    pub nodes: Vec<WaypointNode>,
    pub move_type: WaypointMoveType,
    pub follow_path_backwards_from_end_to_start: bool,
}

impl WaypointPath {
    #[must_use]
    pub fn new(id: u32, nodes: Vec<WaypointNode>) -> Self {
        Self {
            id,
            nodes,
            move_type: WaypointMoveType::Walk,
            follow_path_backwards_from_end_to_start: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaypointUnitSnapshot {
    pub owner_alive: bool,
    pub owner_unit_state: u32,
    pub movement_prevented_by_casting: bool,
    pub move_spline_finalized: bool,
    pub owner_is_on_transport: bool,
    pub owner_is_formation_leader: bool,
    pub formation_leader_move_allowed: bool,
    pub owner_orientation: f32,
    pub owner_position: Position,
    pub ai_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaypointLaunchPlan {
    pub node_id: u32,
    pub path_id: u32,
    pub destination: Position,
    pub generate_path: bool,
    pub disable_transport_transform: bool,
    pub facing: Option<f32>,
    pub walk: Option<bool>,
    pub animation: Option<WaypointAnimation>,
    pub velocity: Option<f32>,
    pub add_unit_state: u32,
    pub signal_formation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaypointInform {
    pub movement_type: MovementGeneratorType,
    pub node_id: u32,
    pub path_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaypointStarted {
    pub node_id: u32,
    pub path_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaypointPathEnded {
    pub node_id: u32,
    pub path_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaypointCurrentInfo {
    pub node_id: u32,
    pub path_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaypointArrivalAction {
    pub clear_roaming_move: bool,
    pub timer_ms: Option<i32>,
    pub move_random_at_path_end: Option<WaypointRandomAtPathEnd>,
    pub duration_after_wait_ms: Option<i32>,
    pub inform: WaypointInform,
    pub current_info: WaypointCurrentInfo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaypointRandomAtPathEnd {
    pub wander_distance: f32,
    pub duration_ms: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaypointMovementAction {
    Continue,
    Finished,
    MissingPath,
    StopMoving,
    StartBlocked { timer_ms: i32 },
    Arrived(WaypointArrivalAction),
    PathEnded(WaypointPathEnded),
    Launch(WaypointLaunchPlan),
    DurationFinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaypointFinalizeAction {
    pub clear_roaming_move: bool,
    pub stop_moving: bool,
    pub set_walk_false: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaypointMovementGenerator {
    state: MovementGeneratorState,
    path: Option<WaypointPath>,
    current_node: usize,
    next_move_time_ms: i32,
    path_id: u32,
    repeating: bool,
    loaded_from_db: bool,
    duration_ms: Option<i32>,
    speed: Option<f32>,
    speed_selection_mode: MovementWalkRunSpeedSelectionMode,
    wait_time_range_at_path_end_ms: Option<(i32, i32)>,
    wander_distance_at_path_ends: Option<f32>,
    follow_path_backwards_from_end_to_start: bool,
    is_returning_to_start: bool,
    generate_path: bool,
    pub stop_moving_calls: u32,
    pub signal_formation_calls: u32,
    pub waypoint_started: Vec<WaypointStarted>,
    pub waypoint_reached: Vec<WaypointInform>,
    pub waypoint_path_ended: Vec<WaypointPathEnded>,
    pub current_info_updates: Vec<WaypointCurrentInfo>,
    pub finalize_action: Option<WaypointFinalizeAction>,
}

impl WaypointMovementGenerator {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_path(
        path: WaypointPath,
        repeating: bool,
        duration_ms: Option<i32>,
        speed: Option<f32>,
        speed_selection_mode: MovementWalkRunSpeedSelectionMode,
        wait_time_range_at_path_end_ms: Option<(i32, i32)>,
        wander_distance_at_path_ends: Option<f32>,
        follow_path_backwards_from_end_to_start: bool,
        generate_path: bool,
    ) -> Self {
        Self {
            state: MovementGeneratorState {
                mode: MovementGeneratorMode::Default,
                priority: MovementGeneratorPriority::Normal,
                flags: MovementGeneratorFlags::INITIALIZATION_PENDING,
                base_unit_state: UNIT_STATE_WAYPOINT_ROAMING_LIKE_CPP,
            },
            path_id: 0,
            path: Some(path),
            current_node: 0,
            next_move_time_ms: 0,
            repeating,
            loaded_from_db: false,
            duration_ms,
            speed,
            speed_selection_mode,
            wait_time_range_at_path_end_ms,
            wander_distance_at_path_ends,
            follow_path_backwards_from_end_to_start,
            is_returning_to_start: false,
            generate_path,
            stop_moving_calls: 0,
            signal_formation_calls: 0,
            waypoint_started: Vec::new(),
            waypoint_reached: Vec::new(),
            waypoint_path_ended: Vec::new(),
            current_info_updates: Vec::new(),
            finalize_action: None,
        }
    }

    #[must_use]
    pub fn from_db_path_id(path_id: u32, repeating: bool) -> Self {
        Self {
            loaded_from_db: true,
            path_id,
            path: None,
            ..Self::from_path(
                WaypointPath::new(0, Vec::new()),
                repeating,
                None,
                None,
                MovementWalkRunSpeedSelectionMode::Default,
                None,
                None,
                false,
                true,
            )
        }
    }

    #[must_use]
    pub const fn current_node(&self) -> usize {
        self.current_node
    }

    #[must_use]
    pub const fn next_move_time_ms(&self) -> i32 {
        self.next_move_time_ms
    }

    #[must_use]
    pub const fn duration_ms(&self) -> Option<i32> {
        self.duration_ms
    }

    #[must_use]
    pub const fn is_returning_to_start(&self) -> bool {
        self.is_returning_to_start
    }

    #[must_use]
    pub const fn repeating(&self) -> bool {
        self.repeating
    }

    pub fn pause_like_cpp(&mut self, timer_ms: u32) {
        if timer_ms > 0 {
            if self.has_flag(MovementGeneratorFlags::PAUSED) {
                return;
            }
            self.add_flag(MovementGeneratorFlags::TIMED_PAUSED);
            self.next_move_time_ms = timer_ms as i32;
            self.remove_flag(MovementGeneratorFlags::PAUSED);
        } else {
            self.add_flag(MovementGeneratorFlags::PAUSED);
            self.next_move_time_ms = WAYPOINT_RESUME_GUARD_MS_LIKE_CPP;
            self.remove_flag(MovementGeneratorFlags::TIMED_PAUSED);
        }
    }

    pub fn resume_like_cpp(&mut self, override_timer_ms: u32) {
        if override_timer_ms > 0 {
            self.next_move_time_ms = override_timer_ms as i32;
        }
        if self.next_move_time_ms <= 0 {
            self.next_move_time_ms = WAYPOINT_RESUME_GUARD_MS_LIKE_CPP;
        }
        self.remove_flag(MovementGeneratorFlags::PAUSED);
    }

    pub fn initialize_like_cpp(
        &mut self,
        owner_exists: bool,
        owner_default_path_id: u32,
        loaded_path: Option<WaypointPath>,
    ) -> WaypointMovementAction {
        self.remove_flag(
            MovementGeneratorFlags::INITIALIZATION_PENDING
                | MovementGeneratorFlags::TRANSITORY
                | MovementGeneratorFlags::DEACTIVATED,
        );

        if self.loaded_from_db {
            if self.path_id == 0 {
                self.path_id = owner_default_path_id;
            }
            self.path = loaded_path;
        }

        if self.path.as_ref().is_none_or(|path| path.nodes.is_empty()) {
            return WaypointMovementAction::MissingPath;
        }

        let path = self.path.as_ref().expect("path checked above");
        self.follow_path_backwards_from_end_to_start = path.follow_path_backwards_from_end_to_start;
        if path.nodes.len() == 1 {
            self.repeating = false;
        }

        if owner_exists {
            self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
        }
        self.next_move_time_ms = WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP;
        WaypointMovementAction::StopMoving
    }

    pub fn reset_like_cpp(&mut self, owner_exists: bool) -> WaypointMovementAction {
        self.remove_flag(MovementGeneratorFlags::TRANSITORY | MovementGeneratorFlags::DEACTIVATED);
        if owner_exists {
            self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
        }
        if !self.has_flag(MovementGeneratorFlags::FINALIZED) && self.next_move_time_ms <= 0 {
            self.next_move_time_ms = WAYPOINT_RESUME_GUARD_MS_LIKE_CPP;
        }
        WaypointMovementAction::StopMoving
    }

    pub fn get_reset_position_like_cpp(&self) -> Option<Position> {
        let path = self.path.as_ref()?;
        path.nodes.get(self.current_node).map(|node| node.position)
    }

    pub fn update_like_cpp(
        &mut self,
        owner_exists: bool,
        diff_ms: u32,
        snapshot: WaypointUnitSnapshot,
        wait_time_roll_ms: Option<i32>,
    ) -> WaypointMovementAction {
        if !owner_exists || !snapshot.owner_alive {
            return WaypointMovementAction::Continue;
        }
        if self.has_flag(MovementGeneratorFlags::FINALIZED | MovementGeneratorFlags::PAUSED)
            || self.path.as_ref().is_none_or(|path| path.nodes.is_empty())
        {
            return WaypointMovementAction::Continue;
        }

        if let Some(duration) = self.duration_ms {
            let remaining = duration.saturating_sub(diff_ms as i32);
            self.duration_ms = Some(remaining);
            if remaining <= 0 {
                self.remove_flag(MovementGeneratorFlags::TRANSITORY);
                self.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
                return WaypointMovementAction::DurationFinished;
            }
        }

        if snapshot.owner_unit_state
            & (UNIT_STATE_WAYPOINT_NOT_MOVE_LIKE_CPP | UNIT_STATE_WAYPOINT_LOST_CONTROL_LIKE_CPP)
            != 0
            || snapshot.movement_prevented_by_casting
        {
            self.add_flag(MovementGeneratorFlags::INTERRUPTED);
            self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
            return WaypointMovementAction::StopMoving;
        }

        if self.has_flag(MovementGeneratorFlags::INTERRUPTED) {
            if self.has_flag(MovementGeneratorFlags::INITIALIZED)
                && (self.next_move_time_ms <= 0
                    || !self.has_flag(MovementGeneratorFlags::INFORM_ENABLED))
            {
                return self.start_move_like_cpp(snapshot, true);
            }
            self.remove_flag(MovementGeneratorFlags::INTERRUPTED);
        }

        if !snapshot.move_spline_finalized {
            if self.has_flag(MovementGeneratorFlags::SPEED_UPDATE_PENDING) {
                return self.start_move_like_cpp(snapshot, true);
            }
            return WaypointMovementAction::Continue;
        }

        if self.next_move_time_ms > 0 {
            self.next_move_time_ms = self.next_move_time_ms.saturating_sub(diff_ms as i32);
            if self.next_move_time_ms <= 0 {
                self.next_move_time_ms = 0;
                if !self.has_flag(MovementGeneratorFlags::INITIALIZED)
                    || !self.has_flag(MovementGeneratorFlags::INFORM_ENABLED)
                {
                    return self.start_move_like_cpp(snapshot, false);
                }
            } else {
                return WaypointMovementAction::Continue;
            }
        }

        if self.has_flag(MovementGeneratorFlags::INITIALIZED)
            && !self.has_flag(MovementGeneratorFlags::INFORM_ENABLED)
        {
            let arrived = self.on_arrived_like_cpp(wait_time_roll_ms);
            self.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
            return WaypointMovementAction::Arrived(arrived);
        }

        if self.next_move_time_ms <= 0 {
            return self.start_move_like_cpp(snapshot, false);
        }

        WaypointMovementAction::Continue
    }

    pub fn on_arrived_like_cpp(&mut self, wait_time_roll_ms: Option<i32>) -> WaypointArrivalAction {
        let path = self.path.as_ref().expect("path required");
        let waypoint = path.nodes[self.current_node];
        let mut clear_roaming_move = false;
        let mut timer_ms = None;
        let mut move_random_at_path_end = None;

        if waypoint.delay_ms > 0 {
            clear_roaming_move = true;
            self.next_move_time_ms = waypoint.delay_ms;
            timer_ms = Some(waypoint.delay_ms);
        }

        let at_path_end = self.wait_time_range_at_path_end_ms.is_some()
            && self.follow_path_backwards_from_end_to_start
            && ((self.is_returning_to_start && self.current_node == 0)
                || (!self.is_returning_to_start && self.current_node == path.nodes.len() - 1));

        if at_path_end {
            clear_roaming_move = true;
            let wait_ms = wait_time_roll_ms
                .or_else(|| self.wait_time_range_at_path_end_ms.map(|range| range.0))
                .unwrap_or(0);
            if let Some(duration) = self.duration_ms {
                self.duration_ms = Some(duration.saturating_sub(wait_ms));
            }
            if let Some(wander_distance) = self.wander_distance_at_path_ends {
                move_random_at_path_end = Some(WaypointRandomAtPathEnd {
                    wander_distance,
                    duration_ms: wait_ms,
                });
            } else {
                self.next_move_time_ms = wait_ms;
                timer_ms = Some(wait_ms);
            }
        }

        let inform = WaypointInform {
            movement_type: MovementGeneratorType::Waypoint,
            node_id: waypoint.id,
            path_id: path.id,
        };
        self.waypoint_reached.push(inform);
        let current_info = WaypointCurrentInfo {
            node_id: waypoint.id,
            path_id: path.id,
        };
        self.current_info_updates.push(current_info);

        WaypointArrivalAction {
            clear_roaming_move,
            timer_ms,
            move_random_at_path_end,
            duration_after_wait_ms: self.duration_ms,
            inform,
            current_info,
        }
    }

    pub fn start_move_like_cpp(
        &mut self,
        snapshot: WaypointUnitSnapshot,
        relaunch: bool,
    ) -> WaypointMovementAction {
        if self.has_flag(MovementGeneratorFlags::FINALIZED)
            || self.path.as_ref().is_none_or(|path| path.nodes.is_empty())
            || (relaunch
                && (self.has_flag(MovementGeneratorFlags::INFORM_ENABLED)
                    || !self.has_flag(MovementGeneratorFlags::INITIALIZED)))
        {
            return WaypointMovementAction::Continue;
        }

        if snapshot.owner_unit_state & UNIT_STATE_WAYPOINT_NOT_MOVE_LIKE_CPP != 0
            || snapshot.movement_prevented_by_casting
            || (snapshot.owner_is_formation_leader && !snapshot.formation_leader_move_allowed)
        {
            self.next_move_time_ms = WAYPOINT_BLOCKED_RETRY_MS_LIKE_CPP;
            return WaypointMovementAction::StartBlocked {
                timer_ms: WAYPOINT_BLOCKED_RETRY_MS_LIKE_CPP,
            };
        }

        if self.has_flag(MovementGeneratorFlags::INFORM_ENABLED)
            && self.has_flag(MovementGeneratorFlags::INITIALIZED)
        {
            if self.compute_next_node_like_cpp() {
                let started = self.started_for_current_node();
                self.waypoint_started.push(started);
            } else {
                let ended = self.path_ended_for_current_node();
                self.add_flag(MovementGeneratorFlags::FINALIZED);
                self.current_info_updates.push(WaypointCurrentInfo {
                    node_id: 0,
                    path_id: 0,
                });
                self.waypoint_path_ended.push(ended);
                return WaypointMovementAction::PathEnded(ended);
            }
        } else if !self.has_flag(MovementGeneratorFlags::INITIALIZED) {
            self.add_flag(MovementGeneratorFlags::INITIALIZED);
            let started = self.started_for_current_node();
            self.waypoint_started.push(started);
        }

        let launch = self.launch_plan_for_current_node(snapshot);
        self.remove_flag(
            MovementGeneratorFlags::TRANSITORY
                | MovementGeneratorFlags::INFORM_ENABLED
                | MovementGeneratorFlags::TIMED_PAUSED,
        );
        self.signal_formation_calls = self.signal_formation_calls.saturating_add(1);
        WaypointMovementAction::Launch(launch)
    }

    pub fn compute_next_node_like_cpp(&mut self) -> bool {
        let Some(path) = &self.path else {
            return false;
        };
        if path.nodes.is_empty() {
            return false;
        }
        if self.current_node == path.nodes.len() - 1 && !self.repeating {
            return false;
        }

        if !self.follow_path_backwards_from_end_to_start
            || path.nodes.len() < WAYPOINT_PATH_FLAG_FOLLOW_PATH_BACKWARDS_MINIMUM_NODES_LIKE_CPP
        {
            self.current_node = (self.current_node + 1) % path.nodes.len();
        } else if !self.is_returning_to_start {
            self.current_node += 1;
            if self.current_node >= path.nodes.len() {
                self.current_node -=
                    WAYPOINT_PATH_FLAG_FOLLOW_PATH_BACKWARDS_MINIMUM_NODES_LIKE_CPP;
                self.is_returning_to_start = true;
            }
        } else if self.current_node == 0 {
            self.current_node = 1;
            self.is_returning_to_start = false;
        } else {
            self.current_node -= 1;
        }

        true
    }

    pub fn deactivate_like_cpp(&mut self) -> WaypointFinalizeAction {
        self.add_flag(MovementGeneratorFlags::DEACTIVATED);
        WaypointFinalizeAction {
            clear_roaming_move: true,
            stop_moving: false,
            set_walk_false: false,
        }
    }

    pub fn finalize_like_cpp(&mut self, active: bool) -> WaypointFinalizeAction {
        self.add_flag(MovementGeneratorFlags::FINALIZED);
        let action = WaypointFinalizeAction {
            clear_roaming_move: active,
            stop_moving: active,
            set_walk_false: active,
        };
        if active {
            self.stop_moving_calls = self.stop_moving_calls.saturating_add(1);
        }
        self.finalize_action = Some(action);
        action
    }

    fn launch_plan_for_current_node(&self, snapshot: WaypointUnitSnapshot) -> WaypointLaunchPlan {
        let path = self.path.as_ref().expect("path required");
        let waypoint = path.nodes[self.current_node];
        let last_node = self.current_node == path.nodes.len() - 1;
        let facing = waypoint
            .orientation
            .filter(|_| waypoint.delay_ms > 0 || last_node);
        let mut walk = match path.move_type {
            WaypointMoveType::Run => Some(false),
            WaypointMoveType::Walk => Some(true),
            WaypointMoveType::Land | WaypointMoveType::TakeOff => None,
        };
        match self.speed_selection_mode {
            MovementWalkRunSpeedSelectionMode::Default => {}
            MovementWalkRunSpeedSelectionMode::ForceRun => walk = Some(false),
            MovementWalkRunSpeedSelectionMode::ForceWalk => walk = Some(true),
        }

        WaypointLaunchPlan {
            node_id: waypoint.id,
            path_id: path.id,
            destination: waypoint.position,
            generate_path: self.generate_path,
            disable_transport_transform: snapshot.owner_is_on_transport,
            facing,
            walk,
            animation: match path.move_type {
                WaypointMoveType::Land => Some(WaypointAnimation::Ground),
                WaypointMoveType::TakeOff => Some(WaypointAnimation::Hover),
                WaypointMoveType::Walk | WaypointMoveType::Run => None,
            },
            velocity: self.speed,
            add_unit_state: UNIT_STATE_WAYPOINT_ROAMING_MOVE_LIKE_CPP,
            signal_formation: true,
        }
    }

    fn started_for_current_node(&self) -> WaypointStarted {
        let path = self.path.as_ref().expect("path required");
        WaypointStarted {
            node_id: path.nodes[self.current_node].id,
            path_id: path.id,
        }
    }

    fn path_ended_for_current_node(&self) -> WaypointPathEnded {
        let path = self.path.as_ref().expect("path required");
        WaypointPathEnded {
            node_id: path.nodes[self.current_node].id,
            path_id: path.id,
        }
    }
}

impl MovementGenerator for WaypointMovementGenerator {
    fn state(&self) -> &MovementGeneratorState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut MovementGeneratorState {
        &mut self.state
    }

    fn kind(&self) -> MovementGeneratorType {
        MovementGeneratorType::Waypoint
    }

    fn initialize(&mut self) {
        self.remove_flag(
            MovementGeneratorFlags::INITIALIZATION_PENDING
                | MovementGeneratorFlags::TRANSITORY
                | MovementGeneratorFlags::DEACTIVATED,
        );
    }

    fn reset(&mut self) {
        self.remove_flag(MovementGeneratorFlags::TRANSITORY | MovementGeneratorFlags::DEACTIVATED);
    }

    fn update(&mut self, _diff_ms: u32) -> bool {
        true
    }

    fn deactivate(&mut self) {
        self.deactivate_like_cpp();
    }

    fn finalize(&mut self, active: bool, _movement_inform: bool) {
        self.finalize_like_cpp(active);
    }

    fn unit_speed_changed(&mut self) {
        self.add_flag(MovementGeneratorFlags::SPEED_UPDATE_PENDING);
    }

    fn pause(&mut self, timer_ms: u32) {
        self.pause_like_cpp(timer_ms);
    }

    fn resume(&mut self, override_timer_ms: u32) {
        self.resume_like_cpp(override_timer_ms);
    }
}

#[cfg(test)]
#[path = "waypoint/tests/mod.rs"]
mod tests;
