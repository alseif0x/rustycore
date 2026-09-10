//! REST API handler implementations.

use num_bigint::BigUint;
use num_traits::Zero;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use wow_crypto::{
    BnetSrp6, SrpHashFunction, SrpVersion, srp_username, utf8_to_upper_only_latin_like_cpp,
};
use wow_database::LoginStatements;

use super::HttpResponse;
use super::types::*;
use crate::state::AppState;

mod encoding;
mod login;
mod responses;
mod srp;
mod tickets;
mod wrong_password;

pub use login::*;
pub use tickets::*;

use encoding::{decode_base64_standard_like_cpp, hex_decode, hex_encode, hex_encode_upper};

use responses::{
    empty_response, error_result, json_error_response, json_error_response_with_content_type,
    json_response, json_response_with_content_type,
};

use srp::{
    BnetRestSrpState, BotSrpState, post_bot_srp_challenge, srp_challenge_headers_like_cpp,
    srp_challenge_missing_account_response_like_cpp, verify_bot_srp_evidence_like_cpp,
};

#[cfg(test)]
use srp::{
    bot_broken_evidence_vector_like_cpp, bot_fixed_32_be_like_cpp, bot_srp_evidence_hash_like_cpp,
    bot_srp_k_like_cpp, bot_srp_n_like_cpp,
};

use tickets::{
    create_login_ticket, get_game_accounts, login_form_headers_like_cpp, make_login_ticket,
    refresh_login_ticket, store_login_ticket,
};

#[cfg(test)]
use tickets::extract_auth_ticket;

use wrong_password::apply_wrong_password_policy_like_cpp;

#[cfg(test)]
use wrong_password::wrong_password_remote_ip_from_headers_like_cpp;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
