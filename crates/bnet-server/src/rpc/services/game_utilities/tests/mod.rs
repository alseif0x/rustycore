//! Game-utilities RPC regressions.
//!
//! Separated from game_utilities.rs under #685.

use super::super::*;

use super::{
    BnetLastLoginInfoUpdateLikeCpp, JoinRealmLoginInfoUpdateLikeCpp,
    account_info_or_status_like_cpp, apply_bnet_last_login_info_update_like_cpp,
    apply_join_realm_login_info_update_like_cpp, bnet_session_key_data_like_cpp,
    find_command_attr_like_cpp, find_command_param_like_cpp, find_param_like_cpp,
    join_realm_response_attributes_like_cpp, last_char_played_response_attributes_like_cpp,
    locale_string_to_id_like_cpp, parse_realm_list_ticket_client_secret_like_cpp,
    parse_realm_list_ticket_game_account_id_like_cpp, process_client_request_command_like_cpp,
    remove_suffix, selected_game_account_like_cpp, should_write_sub_regions_like_cpp,
};
use crate::rpc::session::RpcStatusError;
use crate::state::{AccountInfo, GameAccountInfo, LastPlayedCharInfo};
use std::collections::HashMap;
use wow_database::{PreparedStatement, SqlParam};
use wow_proto::bgs::protocol::{Attribute, Variant};
use wow_proto::status;

fn test_game_account(id: u32, name: &str) -> GameAccountInfo {
    GameAccountInfo {
        id,
        name: name.to_string(),
        display_name: name.to_string(),
        unban_date: 0,
        is_permanently_banned: false,
        is_banned: false,
        security_level: 0,
        char_counts: HashMap::new(),
        last_played_chars: HashMap::new(),
    }
}

fn client_info_attr(blob: &str) -> Attribute {
    Attribute {
        name: "Param_ClientInfo".to_string(),
        value: Variant {
            blob_value: Some(blob.as_bytes().to_vec()),
            ..Default::default()
        },
    }
}
mod scenarios;
