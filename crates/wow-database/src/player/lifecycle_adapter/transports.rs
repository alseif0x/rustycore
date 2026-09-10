//! World transport read statements and row decoding.
//! Private MariaDB implementation; no semantic port or transaction changes.

use crate::params::PreparedStatement;
use crate::statements::WorldStatements;
use wow_persistence::{PlayerLoginTransportLoadRequestLikeCpp, PlayerLoginTransportLoadRowLikeCpp};

pub(super) fn player_login_transport_load_statement_like_cpp(
    request: PlayerLoginTransportLoadRequestLikeCpp,
) -> PreparedStatement {
    match request {
        PlayerLoginTransportLoadRequestLikeCpp::All => {
            PreparedStatement::for_statement(WorldStatements::SEL_LOGIN_TRANSPORTS)
        }
        PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low } => {
            let mut statement =
                PreparedStatement::for_statement(WorldStatements::SEL_LOGIN_TRANSPORT_BY_GUID);
            statement.set_u64(0, guid_low);
            statement
        }
    }
}

pub(super) fn player_login_transport_load_rows_like_cpp(
    mut result: crate::SqlResult,
) -> Vec<PlayerLoginTransportLoadRowLikeCpp> {
    let mut rows = Vec::new();
    if result.is_empty() {
        return rows;
    }
    loop {
        rows.push(PlayerLoginTransportLoadRowLikeCpp {
            guid_low: result
                .try_read::<i64>(0)
                .map(|value| value.max(0) as u32)
                .or_else(|| result.try_read::<u32>(0))
                .unwrap_or(0),
            entry: result
                .try_read::<i32>(1)
                .map(|value| value.max(0) as u32)
                .or_else(|| result.try_read::<u32>(1))
                .unwrap_or(0),
            phase_use_flags: result
                .try_read::<u8>(2)
                .or_else(|| result.try_read::<i16>(2).map(|value| value.max(0) as u8))
                .unwrap_or(0),
            phase_id: result
                .try_read::<u16>(3)
                .or_else(|| result.try_read::<i32>(3).map(|value| value.max(0) as u16))
                .unwrap_or(0),
            phase_group_id: result
                .try_read::<u32>(4)
                .or_else(|| result.try_read::<i32>(4).map(|value| value.max(0) as u32))
                .unwrap_or(0),
            display_id: result
                .try_read::<i32>(5)
                .map(|value| value.max(0) as u32)
                .or_else(|| result.try_read::<u32>(5))
                .unwrap_or(0),
            scale: result.try_read::<f32>(6).unwrap_or(1.0),
            taxi_path_id: result
                .try_read::<i32>(7)
                .map(|value| value.max(0) as u16)
                .or_else(|| result.try_read::<u16>(7))
                .unwrap_or(0),
            move_speed: result
                .try_read::<i32>(8)
                .map(|value| value.max(1) as u32)
                .or_else(|| result.try_read::<u32>(8))
                .unwrap_or(1),
            accel_rate: result
                .try_read::<i32>(9)
                .map(|value| value.max(1) as u32)
                .or_else(|| result.try_read::<u32>(9))
                .unwrap_or(1),
            allow_stopping: result
                .try_read::<i32>(10)
                .map(|value| value != 0)
                .or_else(|| result.try_read::<u8>(10).map(|value| value != 0))
                .unwrap_or(false),
            gameobject_flags: result
                .try_read::<i64>(11)
                .map(|value| value.max(0) as u32)
                .or_else(|| result.try_read::<u32>(11))
                .unwrap_or(0),
            faction_template: result
                .try_read::<i64>(12)
                .map(|value| value as i32)
                .or_else(|| result.try_read::<i32>(12))
                .unwrap_or(0),
        });
        if !result.next_row() {
            break;
        }
    }
    rows
}
