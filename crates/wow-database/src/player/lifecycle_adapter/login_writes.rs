//! Login repairs, online marking and homebind statement plans.
//! Private MariaDB implementation; no semantic port or transaction changes.

use crate::statements::StatementDef;

use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use wow_persistence::{
    PlayerHomebindPersistenceRequestLikeCpp, PlayerLoginItemRepairActionLikeCpp,
    PlayerLoginItemRepairRequestLikeCpp, PlayerOnlineMarkRequestLikeCpp,
};

pub(super) fn player_login_item_repair_statements_like_cpp(
    request: &PlayerLoginItemRepairRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::new();
    for action in &request.actions {
        match action {
            PlayerLoginItemRepairActionLikeCpp::ClearRefundable {
                item_guid,
                new_flags,
            } => {
                let mut delete =
                    PreparedStatement::new(CharStatements::DEL_ITEM_REFUND_INSTANCE.sql());
                delete.set_u64(0, *item_guid);
                statements.push(delete);

                let mut update =
                    PreparedStatement::new(CharStatements::UPD_ITEM_INSTANCE_FLAGS.sql());
                update.set_u32(0, *new_flags);
                update.set_u64(1, *item_guid);
                statements.push(update);
            }
            PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad {
                item_guid,
                expiration,
                flags,
                durability,
            } => {
                let mut update =
                    PreparedStatement::new(CharStatements::UPD_ITEM_INSTANCE_ON_LOAD.sql());
                update.set_u32(0, *expiration);
                update.set_u32(1, *flags);
                update.set_u32(2, *durability);
                update.set_u64(3, *item_guid);
                statements.push(update);
            }
        }
    }
    statements
}

pub(super) fn player_login_pet_talent_reset_statements_like_cpp(
    player_guid: u64,
) -> [PreparedStatement; 2] {
    let mut delete_spells =
        PreparedStatement::new(CharStatements::DEL_ALL_PET_SPELLS_BY_OWNER.sql());
    delete_spells.set_u64(0, player_guid);

    let mut reset_specializations =
        PreparedStatement::new(CharStatements::UPD_PET_SPECS_BY_OWNER.sql());
    reset_specializations.set_u64(0, player_guid);

    [delete_spells, reset_specializations]
}

pub(super) fn player_online_mark_statement_like_cpp(
    request: PlayerOnlineMarkRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::new(CharStatements::UPD_CHAR_ONLINE.sql());
    statement.set_u32(0, request.player_guid);
    statement
}

pub(super) fn player_homebind_persistence_statement_like_cpp(
    request: PlayerHomebindPersistenceRequestLikeCpp,
) -> PreparedStatement {
    match request {
        PlayerHomebindPersistenceRequestLikeCpp::DeleteInvalid { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::DEL_PLAYER_HOMEBIND);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerHomebindPersistenceRequestLikeCpp::InsertRepaired {
            player_guid,
            map_id,
            area_id,
            x,
            y,
            z,
            orientation,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::INS_PLAYER_HOMEBIND);
            statement.set_u64(0, player_guid);
            statement.set_u16(1, map_id);
            statement.set_u16(2, area_id);
            statement.set_f32(3, x);
            statement.set_f32(4, y);
            statement.set_f32(5, z);
            statement.set_f32(6, orientation);
            statement
        }
        PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
            player_guid,
            map_id,
            area_id,
            x,
            y,
            z,
            orientation,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::UPD_PLAYER_HOMEBIND);
            // C++ PreparedStatement::setUInt16 narrows these uint32 values at
            // this concrete adapter boundary.
            statement.set_u16(0, map_id as u16);
            statement.set_u16(1, area_id as u16);
            statement.set_f32(2, x);
            statement.set_f32(3, y);
            statement.set_f32(4, z);
            statement.set_f32(5, orientation);
            statement.set_u64(6, player_guid);
            statement
        }
    }
}
