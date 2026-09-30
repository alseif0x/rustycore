//! Feature-gated forwards to the real Character operation; no replacement logic.

use super::*;

use crate::handlers::character::login_transport_support as original;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapTransportCreateForTest {
    pub guid_low: u32,
    pub entry: u32,
    pub display_id: u32,
    pub scale: f32,
    pub taxi_path_id: u16,
    pub move_speed: u32,
    pub accel_rate: u32,
    pub allow_stopping: bool,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub gameobject_flags: u32,
    pub faction_template: i32,
}

impl From<MapTransportCreateForTest> for original::MapTransportCreateLikeCpp {
    fn from(value: MapTransportCreateForTest) -> Self {
        Self {
            guid_low: value.guid_low,
            entry: value.entry,
            display_id: value.display_id,
            scale: value.scale,
            taxi_path_id: value.taxi_path_id,
            move_speed: value.move_speed,
            accel_rate: value.accel_rate,
            allow_stopping: value.allow_stopping,
            phase_use_flags: value.phase_use_flags,
            phase_id: value.phase_id,
            phase_group_id: value.phase_group_id,
            gameobject_flags: value.gameobject_flags,
            faction_template: value.faction_template,
        }
    }
}

impl From<original::MapTransportCreateLikeCpp> for MapTransportCreateForTest {
    fn from(value: original::MapTransportCreateLikeCpp) -> Self {
        Self {
            guid_low: value.guid_low,
            entry: value.entry,
            display_id: value.display_id,
            scale: value.scale,
            taxi_path_id: value.taxi_path_id,
            move_speed: value.move_speed,
            accel_rate: value.accel_rate,
            allow_stopping: value.allow_stopping,
            phase_use_flags: value.phase_use_flags,
            phase_id: value.phase_id,
            phase_group_id: value.phase_group_id,
            gameobject_flags: value.gameobject_flags,
            faction_template: value.faction_template,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportCreatePositionForTest {
    pub map_id: u16,
    pub position: Position,
    pub timer_ms: u32,
    pub total_time_ms: u32,
}

impl From<TransportCreatePositionForTest> for original::TransportCreatePositionLikeCpp {
    fn from(value: TransportCreatePositionForTest) -> Self {
        Self {
            map_id: value.map_id,
            position: value.position,
            timer_ms: value.timer_ms,
            total_time_ms: value.total_time_ms,
        }
    }
}

impl From<original::TransportCreatePositionLikeCpp> for TransportCreatePositionForTest {
    fn from(value: original::TransportCreatePositionLikeCpp) -> Self {
        Self {
            map_id: value.map_id,
            position: value.position,
            timer_ms: value.timer_ms,
            total_time_ms: value.total_time_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersistedTransportLoginForTest {
    pub guid: ObjectGuid,
    pub map_id: u16,
    pub offset: Position,
    pub world_position: Position,
    pub transport_position: TransportCreatePositionForTest,
    pub transport_create: MapTransportCreateForTest,
}

impl From<PersistedTransportLoginForTest> for original::PersistedTransportLoginLikeCpp {
    fn from(value: PersistedTransportLoginForTest) -> Self {
        Self {
            guid: value.guid,
            map_id: value.map_id,
            offset: value.offset,
            world_position: value.world_position,
            transport_position: value.transport_position.into(),
            transport_create: value.transport_create.into(),
        }
    }
}

impl From<original::PersistedTransportLoginLikeCpp> for PersistedTransportLoginForTest {
    fn from(value: original::PersistedTransportLoginLikeCpp) -> Self {
        Self {
            guid: value.guid,
            map_id: value.map_id,
            offset: value.offset,
            world_position: value.world_position,
            transport_position: value.transport_position.into(),
            transport_create: value.transport_create.into(),
        }
    }
}

pub fn validate_persisted_transport_login_for_test(
    guid: ObjectGuid,
    offset: Position,
    transport_position: TransportCreatePositionForTest,
    transport_create: MapTransportCreateForTest,
) -> Option<PersistedTransportLoginForTest> {
    crate::handlers::character::login_transport_support::validate_persisted_transport_login_like_cpp(guid, offset, transport_position.into(), transport_create.into()).map(Into::into)
}

pub fn transport_route_contains_saved_map_for_test(
    route_map_ids: impl IntoIterator<Item = u16>,
    saved_map_id: u16,
) -> bool {
    crate::handlers::character::login_transport_support::transport_route_contains_saved_map_like_cpp(route_map_ids, saved_map_id)
}

pub fn compose_init_self_create_blocks_for_test(
    player_update: &mut UpdateObject,
    item_creates: Vec<ItemCreateData>,
    own_transport: Option<(ObjectGuid, UpdateBlock)>,
    fellow_passenger_blocks: Vec<UpdateBlock>,
) -> Option<ObjectGuid> {
    crate::handlers::character::login_transport_support::compose_init_self_create_blocks_like_cpp(player_update, item_creates, own_transport, fellow_passenger_blocks)
}
