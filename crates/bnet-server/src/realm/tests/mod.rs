//! Realm list regressions.
//!
//! Separated from mod.rs under #685.

use super::*;
use flate2::read::ZlibDecoder;
use serde_json::Value;
use std::io::Read;

fn test_realm(id: u32, region: u8, battlegroup: u8, timezone: u8, icon: u8) -> Realm {
    Realm {
        id,
        name: format!("Realm{id}"),
        normalized_name: format!("Realm{id}"),
        external_address: "203.0.113.10".to_string(),
        local_address: "10.0.0.10".to_string(),
        port: 8085,
        icon: RealmTypeLikeCpp::from_db_like_cpp(icon),
        flag: RealmFlagsLikeCpp::NONE,
        timezone,
        allowed_security_level: 0,
        population: 2.0,
        build: 51943,
        region,
        battlegroup,
    }
}

fn test_build_info(build: u32, major: u32, minor: u32, bugfix: u32) -> RealmBuildInfo {
    RealmBuildInfo {
        major_version: major,
        minor_version: minor,
        bugfix_version: bugfix,
        hotfix_version: [0; 4],
        build,
        win64_auth_seed: [0; 16],
        mac64_auth_seed: [0; 16],
    }
}

fn inflate_payload(payload: &[u8]) -> String {
    let expected_len = u32::from_le_bytes(payload[0..4].try_into().unwrap()) as usize;
    let mut decoder = ZlibDecoder::new(&payload[4..]);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).unwrap();
    assert_eq!(out.len(), expected_len);
    String::from_utf8(out).unwrap()
}

fn parse_enveloped_json<'a>(payload: &'a str, prefix: &str) -> Value {
    let json = payload
        .strip_prefix(prefix)
        .expect("expected JSON envelope prefix")
        .trim_end_matches('\0');
    serde_json::from_str(json).unwrap()
}

mod scenarios;
