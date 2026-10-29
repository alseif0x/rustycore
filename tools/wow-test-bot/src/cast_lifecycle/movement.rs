//! Finite 3.4.3 client movement actions and acknowledgements.

use super::*;
use anyhow::{bail, ensure, Context, Result};

pub(super) const SMSG_NEW_WORLD: u16 = 0x2594;
pub(super) const SMSG_MOVE_ROOT: u16 = 0x2DF9;
pub(super) const SMSG_MOVE_UNROOT: u16 = 0x2DFA;
pub(super) const SMSG_MOVE_SET_RUN_SPEED: u16 = 0x2DF0;
pub(super) const SMSG_MOVE_SET_SWIM_SPEED: u16 = 0x2DF2;
pub(super) const SMSG_MOVE_SET_WATER_WALK: u16 = 0x2DFB;
pub(super) const SMSG_MOVE_SET_LAND_WALK: u16 = 0x2DFE;
pub(super) const SMSG_MOVE_TELEPORT: u16 = 0x2E04;
pub(super) const SMSG_MOVE_SET_CAN_FLY: u16 = 0x2E05;
pub(super) const SMSG_MOVE_SET_FEATHER_FALL: u16 = 0x2DFF;
pub(super) const SMSG_MOVE_SET_FLIGHT_BACK_SPEED: u16 = 0x2DF5;
pub(super) const SMSG_MOVE_SET_FLIGHT_SPEED: u16 = 0x2DF4;
pub(super) const SMSG_MOVE_SET_HOVERING: u16 = 0x2E01;
pub(super) const SMSG_MOVE_SET_NORMAL_FALL: u16 = 0x2E00;
pub(super) const SMSG_MOVE_SET_PITCH_RATE: u16 = 0x2DF8;
pub(super) const SMSG_MOVE_SET_RUN_BACK_SPEED: u16 = 0x2DF1;
pub(super) const SMSG_MOVE_SET_SWIM_BACK_SPEED: u16 = 0x2DF3;
pub(super) const SMSG_MOVE_SET_TURN_RATE: u16 = 0x2DF7;
pub(super) const SMSG_MOVE_SET_WALK_SPEED: u16 = 0x2DF6;
pub(super) const SMSG_RESUME_TOKEN: u16 = 0x25A9;
pub(super) const SMSG_SUSPEND_TOKEN: u16 = 0x25A8;
pub(super) const SMSG_TRANSFER_PENDING: u16 = 0x25CD;

const CMSG_MOVE_FORCE_RUN_SPEED_CHANGE_ACK: u16 = 0x3A0B;
const CMSG_MOVE_FORCE_SWIM_SPEED_CHANGE_ACK: u16 = 0x3A0D;
const CMSG_MOVE_FORCE_ROOT_ACK: u16 = 0x3A0E;
const CMSG_MOVE_FORCE_UNROOT_ACK: u16 = 0x3A0F;
const CMSG_MOVE_TELEPORT_ACK: u16 = 0x39FA;
const CMSG_MOVE_WATER_WALK_ACK: u16 = 0x3A1D;
const CMSG_WORLD_PORT_RESPONSE: u16 = 0x35FA;
const CMSG_SUSPEND_TOKEN_RESPONSE: u16 = 0x376A;

const MOVEMENTFLAG_FORWARD: u32 = 0x0000_0001;
const MOVEMENTFLAG_BACKWARD: u32 = 0x0000_0002;
const MOVEMENTFLAG_STRAFE_LEFT: u32 = 0x0000_0004;
const MOVEMENTFLAG_STRAFE_RIGHT: u32 = 0x0000_0008;
const MOVEMENTFLAG_ROOT: u32 = 0x0000_0400;
const MOVEMENTFLAG_FALLING: u32 = 0x0000_0800;
const MOVEMENTFLAG_FALLING_FAR: u32 = 0x0000_1000;
const MOVEMENTFLAG_ASCENDING: u32 = 0x0020_0000;
const MOVEMENTFLAG_DESCENDING: u32 = 0x0040_0000;
const MOVEMENTFLAG_WATERWALKING: u32 = 0x0400_0000;
// UnitDefines.h MOVEMENTFLAG_MASK_MOVING; FALLING_FAR is also cleared because
// this controller's jump data is tied to either falling flag.
const MOVEMENTFLAG_MASK_MOVING: u32 = MOVEMENTFLAG_FORWARD
    | MOVEMENTFLAG_BACKWARD
    | MOVEMENTFLAG_STRAFE_LEFT
    | MOVEMENTFLAG_STRAFE_RIGHT
    | MOVEMENTFLAG_FALLING
    | MOVEMENTFLAG_ASCENDING
    | MOVEMENTFLAG_DESCENDING;
const MOVEMENTFLAG_MASK_ROOT_CLEAR: u32 = MOVEMENTFLAG_MASK_MOVING | MOVEMENTFLAG_FALLING_FAR;

pub(super) const SMSG_LOGIN_VERIFY_WORLD: u16 = 0x2597;
const SMSG_TIME_SYNC_REQUEST: u16 = 0x2DD2;
const CMSG_TIME_SYNC_RESPONSE: u16 = 0x3A3D;
const CMSG_MOVE_HEARTBEAT: u16 = 0x3A10;
const CMSG_MOVE_START_FORWARD: u16 = 0x39E4;
const CMSG_MOVE_STOP: u16 = 0x39E6;
const CMSG_MOVE_FALL_LAND: u16 = 0x39FB;

#[derive(Debug, Clone, Copy)]
struct Position {
    map_id: i32,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
}

#[derive(Debug, Clone, Copy)]
struct PendingMovementAck {
    incoming_opcode: u16,
    response_opcode: u16,
    sequence: u32,
    flags: u32,
    fall: Option<FallData>,
    speed: Option<f32>,
}

#[derive(Debug)]
pub(super) struct Outbound {
    pub(super) connection: Connection,
    pub(super) action: &'static str,
    pub(super) opcode: u16,
    pub(super) body: Vec<u8>,
}

/// One transient client mover and movement state for the opt-in lifecycle run.
pub(crate) struct Controller {
    pub(super) options: Options,
    pub(super) evidence: Evidence,
    mover: Guid,
    position: Option<Position>,
    flags: u32,
    fall: Option<FallData>,
    client_time: u32,
    login_active_mover_complete_sent: bool,
    active_mover_complete_count: u32,
    pending_acks: Vec<PendingMovementAck>,
}

impl Controller {
    pub(crate) fn new(options: Options, character_guid: u64, realm_id: u32) -> Self {
        let (low, high) = create_player_guid_raw(character_guid, realm_id);
        let evidence = Evidence {
            observe_only: options.plan.observe_only,
            plan_path: options.source_path.clone(),
            timeout_ms: options.plan.timeout_ms,
            actions: options.plan.actions.clone(),
            expected_rules: options
                .plan
                .expect
                .iter()
                .cloned()
                .map(|rule| ExpectedFact {
                    rule,
                    matched: false,
                    event_sequence: None,
                    detail: "not evaluated".to_string(),
                })
                .collect(),
            finish_policy: options.plan.finish_policy,
            ..Evidence::default()
        };
        let mut controller = Self {
            options,
            evidence,
            mover: Guid { low, high },
            position: None,
            flags: 0,
            fall: None,
            client_time: 0,
            login_active_mover_complete_sent: false,
            active_mover_complete_count: 0,
            pending_acks: Vec::new(),
        };
        controller.update_report_state();
        controller
    }

    /// Process every encrypted login/drain/action observation through this
    /// single controller. `instance` names the socket that received the packet.
    pub(crate) async fn observe_packet(
        &mut self,
        instance: bool,
        opcode: u16,
        payload: &[u8],
        stream: &mut TcpStream,
        crypt: &mut WorldCrypt,
        realm: &mut Option<EncryptedWorldConnection>,
    ) -> Result<()> {
        let connection = if instance {
            Connection::Instance
        } else {
            Connection::Realm
        };
        let outbound = self.record_and_process_packet(connection, opcode, payload)?;
        for packet in outbound {
            let write = match packet.connection {
                Connection::Instance => {
                    send_traced_packet(
                        stream,
                        crypt,
                        &mut self.evidence,
                        packet.connection,
                        None,
                        packet.action,
                        packet.opcode,
                        &packet.body,
                    )
                    .await
                }
                Connection::Realm => {
                    if let Some(realm) = realm.as_mut() {
                        send_traced_packet(
                            &mut realm.stream,
                            &mut realm.crypt,
                            &mut self.evidence,
                            packet.connection,
                            None,
                            packet.action,
                            packet.opcode,
                            &packet.body,
                        )
                        .await
                    } else {
                        // Before ConnectTo, the primary stream still is REALM.
                        send_traced_packet(
                            stream,
                            crypt,
                            &mut self.evidence,
                            packet.connection,
                            None,
                            packet.action,
                            packet.opcode,
                            &packet.body,
                        )
                        .await
                    }
                }
            };
            if let Err(error) = write {
                self.evidence
                    .failure
                    .get_or_insert_with(|| format!("movement control ACK write failed: {error}"));
                return Err(error);
            }
        }
        self.update_report_state();
        Ok(())
    }

    pub(super) fn record_and_process_packet(
        &mut self,
        connection: Connection,
        opcode: u16,
        payload: &[u8],
    ) -> Result<Vec<Outbound>> {
        let event_sequence =
            record_observed_packet(&mut self.evidence, connection, opcode, payload);
        match self.process_packet(connection, opcode, payload) {
            Ok(outbound) => Ok(outbound),
            Err(error) => {
                if let Some(sequence) = event_sequence {
                    if let Some(event) = self.evidence.events.get_mut(sequence) {
                        event.error = Some(error.to_string());
                    }
                }
                self.evidence
                    .failure
                    .get_or_insert_with(|| format!("movement control decode failed: {error}"));
                Err(error)
            }
        }
    }

    pub(super) fn movement_action(&mut self, action: &Action) -> Result<(u16, Vec<u8>)> {
        let Action::Movement {
            kind,
            time_ms,
            x,
            y,
            z,
            orientation,
            fall,
        } = action
        else {
            bail!("movement packet requested for a non-movement action");
        };
        let mut position = self
            .position
            .context("movement action requires LOGIN_VERIFY_WORLD or NEW_WORLD position")?;
        position.x = *x;
        position.y = *y;
        position.z = *z;
        position.orientation = *orientation;
        let mut flags = self.flags;
        let mut fall_state = self.fall;

        match kind {
            MovementKind::Heartbeat => {}
            MovementKind::StartForward => {
                ensure!(
                    flags & MOVEMENTFLAG_ROOT == 0,
                    "start_forward is invalid while the controller is rooted"
                );
                flags |= MOVEMENTFLAG_FORWARD;
            }
            MovementKind::Stop => {
                flags &= !MOVEMENTFLAG_MASK_ROOT_CLEAR;
                fall_state = None;
            }
            MovementKind::FallLand => {
                ensure!(
                    flags & MOVEMENTFLAG_ROOT == 0,
                    "fall_land is invalid while the controller is rooted"
                );
                flags |= MOVEMENTFLAG_FALLING;
            }
        }
        if let Some(fall) = fall {
            fall_state = Some(*fall);
            flags |= MOVEMENTFLAG_FALLING;
        }
        ensure!(
            flags & (MOVEMENTFLAG_FALLING | MOVEMENTFLAG_FALLING_FAR) == 0 || fall_state.is_some(),
            "falling movement requires explicit finite fall data"
        );
        let body = build_movement_info(self.mover, flags, *time_ms, position, fall_state)?;
        let opcode = match kind {
            MovementKind::Heartbeat => CMSG_MOVE_HEARTBEAT,
            MovementKind::StartForward => CMSG_MOVE_START_FORWARD,
            MovementKind::Stop => CMSG_MOVE_STOP,
            MovementKind::FallLand => CMSG_MOVE_FALL_LAND,
        };
        if *kind == MovementKind::FallLand {
            flags &= !(MOVEMENTFLAG_FALLING | MOVEMENTFLAG_FALLING_FAR);
            fall_state = None;
        }
        self.position = Some(position);
        self.client_time = *time_ms;
        self.flags = flags;
        self.fall = fall_state;
        self.update_report_state();
        Ok((opcode, body))
    }

    pub(super) fn process_packet(
        &mut self,
        connection: Connection,
        opcode: u16,
        payload: &[u8],
    ) -> Result<Vec<Outbound>> {
        match opcode {
            SMSG_TRANSFER_PENDING => {
                ensure!(
                    connection == Connection::Realm,
                    "TRANSFER_PENDING must arrive on REALM"
                );
                Ok(Vec::new())
            }
            SMSG_RESUME_TOKEN => {
                ensure!(
                    connection == Connection::Instance,
                    "RESUME_TOKEN must arrive on INSTANCE"
                );
                ensure!(payload.len() == 5, "RESUME_TOKEN body must be five bytes");
                ensure!(
                    payload[4] & 0x3F == 0,
                    "RESUME_TOKEN reason-bit padding is nonzero"
                );
                Ok(Vec::new())
            }
            SMSG_LOGIN_VERIFY_WORLD => {
                ensure!(
                    connection == Connection::Instance,
                    "LOGIN_VERIFY_WORLD must arrive on INSTANCE"
                );
                let login = parse_login_verify_world_like_cpp(payload)?;
                self.seed_position(Position {
                    map_id: login.map_id,
                    x: login.x,
                    y: login.y,
                    z: login.z,
                    orientation: login.orientation,
                })?;
                let mut packets = self.flush_pending_acks()?;
                if !self.login_active_mover_complete_sent {
                    self.login_active_mover_complete_sent = true;
                    self.active_mover_complete_count += 1;
                    self.update_report_state();
                    packets.push(Outbound {
                        connection: Connection::Instance,
                        action: "initialize_active_mover",
                        opcode: CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
                        body: build_move_init_active_mover_complete_payload(0).to_vec(),
                    });
                }
                Ok(packets)
            }
            SMSG_NEW_WORLD => {
                ensure!(
                    connection == Connection::Realm,
                    "NEW_WORLD must arrive on REALM"
                );
                let position = parse_new_world(payload)?;
                self.seed_position(position)?;
                let mut packets = self.flush_pending_acks()?;
                packets.push(Outbound {
                    connection: Connection::Instance,
                    action: "world_port_response",
                    opcode: CMSG_WORLD_PORT_RESPONSE,
                    body: Vec::new(),
                });
                // Player::TeleportTo resets the visibility override. Re-send
                // the client's active-mover completion after world-port response.
                self.active_mover_complete_count += 1;
                self.update_report_state();
                packets.push(Outbound {
                    connection: Connection::Instance,
                    action: "complete_active_mover_after_worldport",
                    opcode: CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
                    body: build_move_init_active_mover_complete_payload(0).to_vec(),
                });
                Ok(packets)
            }
            SMSG_SUSPEND_TOKEN => {
                ensure!(
                    connection == Connection::Instance,
                    "SUSPEND_TOKEN must arrive on INSTANCE"
                );
                ensure!(payload.len() == 5, "SUSPEND_TOKEN body must be five bytes");
                let mut cursor = Cursor::new(payload);
                let sequence = cursor.u32()?;
                let reason_bits = cursor.u8()?;
                ensure!(
                    reason_bits & 0x3F == 0,
                    "SUSPEND_TOKEN reason-bit padding is nonzero"
                );
                Ok(vec![Outbound {
                    connection: Connection::Instance,
                    action: "suspend_token_response",
                    opcode: CMSG_SUSPEND_TOKEN_RESPONSE,
                    body: sequence.to_le_bytes().to_vec(),
                }])
            }
            SMSG_TIME_SYNC_REQUEST => {
                ensure!(
                    connection == Connection::Instance,
                    "TIME_SYNC_REQUEST must arrive on INSTANCE"
                );
                let sequence = parse_time_sync_request_sequence(payload)?;
                Ok(vec![Outbound {
                    connection: Connection::Instance,
                    action: "time_sync_response",
                    opcode: CMSG_TIME_SYNC_RESPONSE,
                    body: build_time_sync_response_payload(sequence, 0).to_vec(),
                }])
            }
            SMSG_MOVE_ROOT
            | SMSG_MOVE_UNROOT
            | SMSG_MOVE_SET_WATER_WALK
            | SMSG_MOVE_SET_LAND_WALK => {
                ensure!(
                    connection == Connection::Instance,
                    "movement flag control must arrive on INSTANCE"
                );
                let mut cursor = Cursor::new(payload);
                self.require_mover(cursor.guid()?)?;
                let sequence = cursor.u32()?;
                cursor.finish("MoveSetFlag")?;
                let response_opcode = match opcode {
                    SMSG_MOVE_ROOT => {
                        self.flags &= !MOVEMENTFLAG_MASK_ROOT_CLEAR;
                        self.fall = None;
                        self.flags |= MOVEMENTFLAG_ROOT;
                        CMSG_MOVE_FORCE_ROOT_ACK
                    }
                    SMSG_MOVE_UNROOT => {
                        self.flags &= !MOVEMENTFLAG_ROOT;
                        CMSG_MOVE_FORCE_UNROOT_ACK
                    }
                    SMSG_MOVE_SET_WATER_WALK => {
                        self.flags |= MOVEMENTFLAG_WATERWALKING;
                        CMSG_MOVE_WATER_WALK_ACK
                    }
                    SMSG_MOVE_SET_LAND_WALK => {
                        self.flags &= !MOVEMENTFLAG_WATERWALKING;
                        CMSG_MOVE_WATER_WALK_ACK
                    }
                    _ => unreachable!(),
                };
                self.queue_movement_ack(opcode, response_opcode, sequence, None)
            }
            SMSG_MOVE_SET_RUN_SPEED | SMSG_MOVE_SET_SWIM_SPEED => {
                ensure!(
                    connection == Connection::Instance,
                    "movement speed control must arrive on INSTANCE"
                );
                let mut cursor = Cursor::new(payload);
                self.require_mover(cursor.guid()?)?;
                let sequence = cursor.u32()?;
                let speed = cursor.f32()?;
                cursor.finish("MoveSetSpeed")?;
                ensure!(
                    speed.is_finite() && speed > 0.0,
                    "movement speed must be finite and positive"
                );
                let (response_opcode, _action) = if opcode == SMSG_MOVE_SET_RUN_SPEED {
                    (CMSG_MOVE_FORCE_RUN_SPEED_CHANGE_ACK, "ack_move_run_speed")
                } else {
                    (CMSG_MOVE_FORCE_SWIM_SPEED_CHANGE_ACK, "ack_move_swim_speed")
                };
                self.queue_movement_ack(opcode, response_opcode, sequence, Some(speed))
            }
            SMSG_MOVE_TELEPORT => {
                ensure!(
                    connection == Connection::Instance,
                    "MOVE_TELEPORT must arrive on INSTANCE"
                );
                let (mover, sequence, mut destination) = parse_move_teleport(payload)?;
                self.require_mover(mover)?;
                destination.map_id = self
                    .position
                    .context("near teleport arrived before client position was initialized")?
                    .map_id;
                self.seed_position(destination)?;
                let mut body = self.mover.packed();
                body.extend(sequence.to_le_bytes());
                body.extend(self.client_time.to_le_bytes());
                let mut packets = self.flush_pending_acks()?;
                packets.push(Outbound {
                    connection: Connection::Instance,
                    action: "ack_move_teleport",
                    opcode: CMSG_MOVE_TELEPORT_ACK,
                    body,
                });
                Ok(packets)
            }
            SMSG_MOVE_SET_CAN_FLY
            | SMSG_MOVE_SET_FEATHER_FALL
            | SMSG_MOVE_SET_FLIGHT_BACK_SPEED
            | SMSG_MOVE_SET_FLIGHT_SPEED
            | SMSG_MOVE_SET_HOVERING
            | SMSG_MOVE_SET_NORMAL_FALL
            | SMSG_MOVE_SET_PITCH_RATE
            | SMSG_MOVE_SET_RUN_BACK_SPEED
            | SMSG_MOVE_SET_SWIM_BACK_SPEED
            | SMSG_MOVE_SET_TURN_RATE
            | SMSG_MOVE_SET_WALK_SPEED => {
                ensure!(
                    connection == Connection::Instance,
                    "movement control must arrive on INSTANCE"
                );
                bail!("unsupported movement control opcode 0x{opcode:04X}");
            }
            _ => Ok(Vec::new()),
        }
    }

    fn require_mover(&self, mover: Guid) -> Result<()> {
        ensure!(
            mover == self.mover,
            "server movement control named a non-player mover"
        );
        Ok(())
    }

    fn seed_position(&mut self, position: Position) -> Result<()> {
        ensure!(
            [position.x, position.y, position.z, position.orientation]
                .into_iter()
                .all(f32::is_finite),
            "server movement position contains a non-finite float"
        );
        if self
            .position
            .is_some_and(|old| old.map_id != position.map_id)
        {
            self.flags &= !(MOVEMENTFLAG_FALLING | MOVEMENTFLAG_FALLING_FAR);
            self.fall = None;
        }
        self.position = Some(position);
        self.update_report_state();
        Ok(())
    }

    fn queue_movement_ack(
        &mut self,
        incoming_opcode: u16,
        response_opcode: u16,
        sequence: u32,
        speed: Option<f32>,
    ) -> Result<Vec<Outbound>> {
        let pending = PendingMovementAck {
            incoming_opcode,
            response_opcode,
            sequence,
            flags: self.flags,
            fall: self.fall,
            speed,
        };
        self.update_report_state();
        if self.position.is_none() {
            ensure!(
                self.pending_acks.len() < 32,
                "too many movement ACKs arrived before initial position"
            );
            self.pending_acks.push(pending);
            return Ok(Vec::new());
        }
        Ok(vec![self.build_pending_ack(pending)?])
    }

    fn flush_pending_acks(&mut self) -> Result<Vec<Outbound>> {
        let pending = std::mem::take(&mut self.pending_acks);
        pending
            .into_iter()
            .map(|pending| self.build_pending_ack(pending))
            .collect()
    }

    fn build_pending_ack(&self, pending: PendingMovementAck) -> Result<Outbound> {
        let position = self
            .position
            .context("movement ACK requires a seeded player position")?;
        let body = build_movement_info(
            self.mover,
            pending.flags,
            self.client_time,
            position,
            pending.fall,
        )?;
        let mut body = body;
        body.extend(pending.sequence.to_le_bytes());
        if let Some(speed) = pending.speed {
            body.extend(speed.to_le_bytes());
        }
        let action = match pending.response_opcode {
            CMSG_MOVE_FORCE_ROOT_ACK => "ack_move_root",
            CMSG_MOVE_FORCE_UNROOT_ACK => "ack_move_unroot",
            CMSG_MOVE_WATER_WALK_ACK if pending.incoming_opcode == SMSG_MOVE_SET_LAND_WALK => {
                "ack_move_land_walk"
            }
            CMSG_MOVE_WATER_WALK_ACK => "ack_move_water_walk",
            CMSG_MOVE_FORCE_RUN_SPEED_CHANGE_ACK => "ack_move_run_speed",
            CMSG_MOVE_FORCE_SWIM_SPEED_CHANGE_ACK => "ack_move_swim_speed",
            _ => bail!(
                "unsupported movement ACK opcode 0x{:04X}",
                pending.response_opcode
            ),
        };
        Ok(Outbound {
            connection: Connection::Instance,
            action,
            opcode: pending.response_opcode,
            body,
        })
    }

    fn update_report_state(&mut self) {
        self.evidence.movement_state = Some(MovementStateFact {
            mover: self.mover,
            map_id: self.position.map(|position| position.map_id),
            x: self.position.map(|position| position.x),
            y: self.position.map(|position| position.y),
            z: self.position.map(|position| position.z),
            orientation: self.position.map(|position| position.orientation),
            flags: self.flags,
            active_mover_complete_count: self.active_mover_complete_count,
        });
    }
}

fn build_movement_info(
    mover: Guid,
    flags: u32,
    time_ms: u32,
    position: Position,
    fall: Option<FallData>,
) -> Result<Vec<u8>> {
    let falling = flags & (MOVEMENTFLAG_FALLING | MOVEMENTFLAG_FALLING_FAR) != 0;
    ensure!(
        falling == fall.is_some(),
        "MovementInfo FALLING flag and fall data must agree"
    );
    if let Some(fall) = fall {
        ensure!(
            fall.fall_time_ms > 0
                && fall.fall_time_ms <= MAX_WAIT_MS as u32
                && [fall.z_speed, fall.sin_angle, fall.cos_angle, fall.xy_speed]
                    .into_iter()
                    .all(f32::is_finite),
            "MovementInfo fall data must be bounded and finite"
        );
    }
    let mut body = build_client_movement_payload(
        mover.low,
        mover.high,
        flags,
        time_ms,
        position.x,
        position.y,
        position.z,
        position.orientation,
    );
    if let Some(fall) = fall {
        let presence = body
            .last_mut()
            .context("movement payload is missing the eight presence bits")?;
        *presence |= 0x20; // C++ MovementInfo::operator>> hasFall bit.
        body.extend(fall.fall_time_ms.to_le_bytes());
        body.extend(fall.z_speed.to_le_bytes());
        body.push(0x80); // hasFallDirection
        body.extend(fall.sin_angle.to_le_bytes());
        body.extend(fall.cos_angle.to_le_bytes());
        body.extend(fall.xy_speed.to_le_bytes());
    }
    Ok(body)
}

fn parse_new_world(payload: &[u8]) -> Result<Position> {
    ensure!(payload.len() == 44, "NEW_WORLD body must be 44 bytes");
    let mut cursor = Cursor::new(payload);
    let map_id = cursor.i32()?;
    let position = Position {
        map_id,
        x: cursor.f32()?,
        y: cursor.f32()?,
        z: cursor.f32()?,
        orientation: cursor.f32()?,
    };
    let _unused_901_1 = cursor.i32()?;
    let _unused_901_2 = cursor.i32()?;
    let _reason = cursor.u32()?;
    let _movement_offset_x = cursor.f32()?;
    let _movement_offset_y = cursor.f32()?;
    let _movement_offset_z = cursor.f32()?;
    cursor.finish("NewWorld")?;
    Ok(position)
}

fn parse_move_teleport(payload: &[u8]) -> Result<(Guid, u32, Position)> {
    let mut cursor = Cursor::new(payload);
    let mover = cursor.guid()?;
    let sequence = cursor.u32()?;
    let position = Position {
        map_id: 0, // near teleports do not carry a map ID; caller retains current map.
        x: cursor.f32()?,
        y: cursor.f32()?,
        z: cursor.f32()?,
        orientation: cursor.f32()?,
    };
    let _preload_world = cursor.u8()?;
    let optional = cursor.u8()?;
    ensure!(
        optional & 0x3F == 0,
        "MOVE_TELEPORT optional-bit padding is nonzero"
    );
    let has_transport = optional & 0x80 != 0;
    let has_vehicle = optional & 0x40 != 0;
    ensure!(!has_vehicle, "MOVE_TELEPORT vehicle data are unsupported");
    if has_transport {
        ensure!(
            cursor.guid()?.empty(),
            "MOVE_TELEPORT nonempty transport is unsupported"
        );
    }
    cursor.finish("MoveTeleport")?;
    Ok((mover, sequence, position))
}
