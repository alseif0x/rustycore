//! Protobuf message types for the Battle.net RPC protocol.
//!
//! Generated from `.proto` definitions via `prost-build`. The proto files
//! follow the Blizzard BNet protocol structure used by WoW 3.4.3 clients.
//!
//! # Module Structure
//!
//! The generated code mirrors the protobuf package hierarchy:
//! - `bgs::protocol` — Core RPC types (Header, ProcessId, EntityId, Attribute, etc.)
//! - `bgs::protocol::authentication::v1` — Authentication messages
//! - `bgs::protocol::authentication::v2::client` — Modern authentication messages
//! - `bgs::protocol::connection::v1` — Connection messages
//! - `bgs::protocol::challenge::v1` — Challenge messages
//! - `bgs::protocol::game_utilities::v1` — GameUtilities messages
//! - `bgs::protocol::account::v1` — Account messages
//! - `bgs::protocol::account::v2` — Modern game-account handles

// Include the generated protobuf code.
// prost-build generates one file per package, named by the package path.
pub mod bgs {
    pub mod protocol {
        // Core types: Header, ProcessId, NoData, EntityId, Attribute, Variant, etc.
        include!(concat!(env!("OUT_DIR"), "/bgs.protocol.rs"));

        pub mod v2 {
            include!(concat!(env!("OUT_DIR"), "/bgs.protocol.v2.rs"));
        }

        pub mod authentication {
            pub mod v1 {
                include!(concat!(
                    env!("OUT_DIR"),
                    "/bgs.protocol.authentication.v1.rs"
                ));
            }

            /// Wire projection for the modern client authentication service.
            /// This is intentionally not a complete BNet V2 API surface.
            pub mod v2 {
                pub mod client {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/bgs.protocol.authentication.v2.client.rs"
                    ));
                }
            }
        }

        pub mod connection {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/bgs.protocol.connection.v1.rs"));
            }
        }

        pub mod challenge {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/bgs.protocol.challenge.v1.rs"));
            }
        }

        pub mod game_utilities {
            pub mod v2 {
                pub mod client {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/bgs.protocol.game_utilities.v2.client.rs"
                    ));
                }
            }
            pub mod v1 {
                include!(concat!(
                    env!("OUT_DIR"),
                    "/bgs.protocol.game_utilities.v1.rs"
                ));
            }
        }

        pub mod account {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/bgs.protocol.account.v1.rs"));
            }

            pub mod v2 {
                include!(concat!(env!("OUT_DIR"), "/bgs.protocol.account.v2.rs"));
                pub mod client {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/bgs.protocol.account.v2.client.rs"
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Service hash constants (from C# OriginalHash enum)
// ---------------------------------------------------------------------------

/// Service hashes used for BNet RPC dispatch.
///
/// The `service_hash` field in `Header` identifies which service a request
/// targets. These are fixed values defined by the Blizzard BNet protocol.
pub mod service_hash {
    // Server-side services (server handles client requests)
    pub const AUTHENTICATION_SERVICE: u32 = 0x0DEC_FC01;
    pub const AUTHENTICATION_SERVICE_V2: u32 = 0xC02F_8216;
    pub const CONNECTION_SERVICE: u32 = 0x6544_6991;
    pub const ACCOUNT_SERVICE: u32 = 0x62DA_0891;
    pub const ACCOUNT_SERVICE_V2: u32 = 0x22DC_2464;
    pub const GAME_UTILITIES_SERVICE: u32 = 0x3FC1_274D;
    pub const GAME_UTILITIES_SERVICE_V2: u32 = 0x5DBB_51C2;

    // Client-side listeners (server sends notifications to client)
    pub const AUTHENTICATION_LISTENER: u32 = 0x7124_0E35;
    pub const AUTHENTICATION_LISTENER_V2: u32 = 0x9DA8_116B;
    pub const CHALLENGE_LISTENER: u32 = 0xBBDA_171F;
    pub const ACCOUNT_LISTENER: u32 = 0x54DF_DA17;

    // Other services (not currently used by BNet server)
    pub const FRIENDS_SERVICE: u32 = 0xA3DD_B1BD;
    pub const FRIENDS_LISTENER: u32 = 0x6F25_9A13;
    pub const PRESENCE_SERVICE: u32 = 0xFA07_96FF;
    pub const PRESENCE_LISTENER: u32 = 0x890A_B85F;
    pub const REPORT_SERVICE: u32 = 0x7CAF_61C9;
    pub const REPORT_SERVICE_V2: u32 = 0x3A42_18FB;
    pub const RESOURCES_SERVICE: u32 = 0xECBE_75BA;
    pub const USER_MANAGER_SERVICE: u32 = 0x3E19_268A;
    pub const USER_MANAGER_LISTENER: u32 = 0xBC87_2C22;
}

/// BNet RPC status codes.
pub mod status {
    pub const OK: u32 = 0;
    pub const ERROR_INTERNAL: u32 = 1;
    pub const ERROR_TIMED_OUT: u32 = 2;
    pub const ERROR_DENIED: u32 = 3;
    pub const ERROR_RPC_INVALID_METHOD: u32 = 0x0000_0BC3;
    pub const ERROR_RPC_MALFORMED_REQUEST: u32 = 0x0000_0BC5;
    pub const ERROR_RPC_NOT_IMPLEMENTED: u32 = 0x0000_0BC7;
    pub const ERROR_BAD_PROGRAM: u32 = 0x4D;
    pub const ERROR_BAD_LOCALE: u32 = 0x4E;
    pub const ERROR_BAD_PLATFORM: u32 = 0x4F;
    pub const ERROR_NO_GAME_ACCOUNT: u32 = 12;
    pub const ERROR_GAME_ACCOUNT_BANNED: u32 = 0x34;
    pub const ERROR_GAME_ACCOUNT_SUSPENDED: u32 = 0x35;
    pub const ERROR_UTIL_SERVER_UNKNOWN_REALM: u32 = 0x8000_0069;
    pub const ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS: u32 = 0x8000_006E;
    pub const ERROR_UTIL_SERVER_FAILED_TO_SERIALIZE_RESPONSE: u32 = 0x8000_0073;
    pub const ERROR_USER_SERVER_BAD_WOW_ACCOUNT: u32 = 0x8000_00D3;
    pub const ERROR_USER_SERVER_NOT_PERMITTED_ON_REALM: u32 = 0x8000_00E1;
    pub const ERROR_WOW_SERVICES_INVALID_JOIN_TICKET: u32 = 0x8000_012E;
    pub const ERROR_WOW_SERVICES_DENIED_REALM_LIST_TICKET: u32 = 0x8000_0132;
    pub const ERROR_WOW_SERVICES_GAME_ACCOUNT_LOCKED: u32 = 0x0002_0014;
    pub const ERROR_RISK_ACCOUNT_LOCKED: u32 = 0xA413;
}

/// The special `service_id` value used for response messages.
pub const RESPONSE_SERVICE_ID: u32 = 0xFE;

// ---------------------------------------------------------------------------
// Re-exports for convenience
// ---------------------------------------------------------------------------

pub use bgs::protocol::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        use prost::Message;

        let header = Header {
            service_id: 0,
            method_id: Some(1),
            token: 42,
            service_hash: Some(service_hash::AUTHENTICATION_SERVICE),
            size: Some(0),
            ..Default::default()
        };

        let mut buf = Vec::new();
        header.encode(&mut buf).unwrap();

        let decoded = Header::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.service_id, 0);
        assert_eq!(decoded.method_id, Some(1));
        assert_eq!(decoded.token, 42);
        assert_eq!(
            decoded.service_hash,
            Some(service_hash::AUTHENTICATION_SERVICE)
        );
    }

    #[test]
    fn entity_id_roundtrip() {
        use prost::Message;

        let id = EntityId {
            high: 0x0100_0000_0000_0000,
            low: 1,
        };

        let mut buf = Vec::new();
        id.encode(&mut buf).unwrap();

        let decoded = EntityId::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.high, 0x0100_0000_0000_0000);
        assert_eq!(decoded.low, 1);
    }

    #[test]
    fn logon_request_encode() {
        use prost::Message;

        let req = authentication::v1::LogonRequest {
            program: Some("WoW".to_string()),
            platform: Some("Wn64".to_string()),
            locale: Some("enUS".to_string()),
            ..Default::default()
        };

        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();
        assert!(!buf.is_empty());

        let decoded = authentication::v1::LogonRequest::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.program.as_deref(), Some("WoW"));
        assert_eq!(decoded.platform.as_deref(), Some("Wn64"));
    }

    #[test]
    fn connect_request_encode() {
        use prost::Message;

        let req = connection::v1::ConnectRequest {
            client_id: Some(ProcessId {
                label: 1,
                epoch: 100,
            }),
            use_bindless_rpc: Some(true),
            ..Default::default()
        };

        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();

        let decoded = connection::v1::ConnectRequest::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.client_id.unwrap().label, 1);
        assert_eq!(decoded.use_bindless_rpc, Some(true));
    }

    #[test]
    fn client_request_with_attributes() {
        use prost::Message;

        let req = game_utilities::v1::ClientRequest {
            attribute: vec![Attribute {
                name: "Command_RealmListRequest_v1".to_string(),
                value: Variant {
                    string_value: Some("test".to_string()),
                    ..Default::default()
                },
            }],
            ..Default::default()
        };

        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();

        let decoded = game_utilities::v1::ClientRequest::decode(buf.as_slice()).unwrap();
        assert_eq!(decoded.attribute.len(), 1);
        assert_eq!(decoded.attribute[0].name, "Command_RealmListRequest_v1");
    }

    #[test]
    fn service_hash_constants_are_correct() {
        // Verify against known values from C# source
        assert_eq!(service_hash::AUTHENTICATION_SERVICE, 0x0DEC_FC01);
        assert_eq!(service_hash::AUTHENTICATION_SERVICE_V2, 0xC02F_8216);
        assert_eq!(service_hash::CONNECTION_SERVICE, 0x6544_6991);
        assert_eq!(service_hash::ACCOUNT_SERVICE, 0x62DA_0891);
        assert_eq!(service_hash::GAME_UTILITIES_SERVICE, 0x3FC1_274D);
        assert_eq!(service_hash::AUTHENTICATION_LISTENER, 0x7124_0E35);
        assert_eq!(service_hash::AUTHENTICATION_LISTENER_V2, 0x9DA8_116B);
        assert_eq!(service_hash::CHALLENGE_LISTENER, 0xBBDA_171F);
    }

    #[test]
    fn authentication_v2_logon_wire_golden_roundtrip() {
        use prost::Message;

        let request = authentication::v2::client::LogonRequest {
            title_id: Some(0x1234),
            platform: Some("Wn64".to_string()),
            locale: Some("esES".to_string()),
            application_version: Some(70_170),
            logon_options: Some(authentication::v2::client::LogonOptions {
                auth_token: Some(vec![0xAA, 0xBB]),
                device_id: Some("UTCO".to_string()),
                ..Default::default()
            }),
        };

        let encoded = request.encode_to_vec();
        assert_eq!(
            encoded,
            vec![
                0x08, 0xB4, 0x24, 0x12, 0x04, b'W', b'n', b'6', b'4', 0x1A, 0x04, b'e', b's', b'E',
                b'S', 0x20, 0x9A, 0xA4, 0x04, 0x52, 0x0A, 0x0A, 0x02, 0xAA, 0xBB, 0x1A, 0x04, b'U',
                b'T', b'C', b'O',
            ]
        );

        let decoded = authentication::v2::client::LogonRequest::decode(encoded.as_slice())
            .expect("golden V2 LogonRequest should decode");
        assert_eq!(decoded.title_id, Some(0x1234));
        assert_eq!(decoded.platform.as_deref(), Some("Wn64"));
        assert_eq!(decoded.locale.as_deref(), Some("esES"));
        assert_eq!(decoded.application_version, Some(70_170));
        let options = decoded.logon_options.expect("logon options");
        assert_eq!(options.auth_token, Some(vec![0xAA, 0xBB]));
        assert_eq!(options.device_id.as_deref(), Some("UTCO"));
    }

    #[test]
    fn authentication_v2_unknown_fields_are_ignored() {
        use prost::Message;

        let mut encoded = authentication::v2::client::VerifyAuthTokenRequest {
            auth_token: Some(vec![0x01, 0x02]),
        }
        .encode_to_vec();
        // Field 100, varint value 1: an unknown field from a newer client.
        encoded.extend_from_slice(&[0xA0, 0x06, 0x01]);

        let decoded =
            authentication::v2::client::VerifyAuthTokenRequest::decode(encoded.as_slice())
                .expect("unknown protobuf fields must be ignored");
        assert_eq!(decoded.auth_token, Some(vec![0x01, 0x02]));
    }

    #[test]
    fn authentication_v2_token_messages_roundtrip() {
        use prost::Message;

        let request = authentication::v2::client::GenerateAuthTokenRequest {
            title_id: Some(0x1234),
        };
        let decoded_request = authentication::v2::client::GenerateAuthTokenRequest::decode(
            request.encode_to_vec().as_slice(),
        )
        .expect("generate token request should round-trip");
        assert_eq!(decoded_request.title_id, Some(0x1234));

        let response = authentication::v2::client::GenerateAuthTokenResponse {
            auth_token: Some(vec![0x10, 0x20]),
        };
        let decoded_response = authentication::v2::client::GenerateAuthTokenResponse::decode(
            response.encode_to_vec().as_slice(),
        )
        .expect("generate token response should round-trip");
        assert_eq!(decoded_response.auth_token, Some(vec![0x10, 0x20]));
    }

    #[test]
    fn authentication_v2_listener_messages_roundtrip() {
        use prost::Message;

        let challenge = authentication::v2::client::ExternalChallengeNotification {
            request_token: Some("request".to_string()),
            payload_type: Some("web_auth_url".to_string()),
            payload: Some(b"https://127.0.0.1/login".to_vec()),
        };
        let challenge_decoded = authentication::v2::client::ExternalChallengeNotification::decode(
            challenge.encode_to_vec().as_slice(),
        )
        .expect("external challenge should round-trip");
        assert_eq!(challenge_decoded.request_token, challenge.request_token);
        assert_eq!(challenge_decoded.payload_type, challenge.payload_type);
        assert_eq!(challenge_decoded.payload, challenge.payload);

        let complete = authentication::v2::client::LogonCompleteNotification {
            error_code: Some(0),
            record: Some(authentication::v2::client::LogonRecord {
                account_id: Some(1),
                game_account: vec![account::v2::GameAccountHandle {
                    id: Some(1),
                    title_id: Some(0x574F57),
                    region: Some(2),
                }],
                session_key: Some(vec![0x5A; 64]),
                ..Default::default()
            }),
        };
        let complete_decoded = authentication::v2::client::LogonCompleteNotification::decode(
            complete.encode_to_vec().as_slice(),
        )
        .expect("logon complete should round-trip");
        let record = complete_decoded.record.expect("logon record");
        assert_eq!(complete_decoded.error_code, Some(0));
        assert_eq!(record.account_id, Some(1));
        assert_eq!(record.game_account[0].id, Some(1));
        assert_eq!(record.game_account[0].title_id, Some(0x574F57));
        assert_eq!(record.game_account[0].region, Some(2));
        assert_eq!(record.session_key, Some(vec![0x5A; 64]));
    }

    #[test]
    fn modern_service_hashes_match_versioned_definitions() {
        fn fnv1a32(name: &str) -> u32 {
            name.bytes().fold(0x811C_9DC5, |hash, byte| {
                (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
            })
        }

        assert_eq!(
            fnv1a32("bnet.protocol.account.v2.client.AccountService"),
            service_hash::ACCOUNT_SERVICE_V2
        );
        // GameUtilities' OriginalHash must not be inferred from the current
        // descriptor name. TC 6ebe044c api/client/v2/game_utilities_service.pb.h
        // declares this literal independently from its NameHash (0x0FD547D8).
        assert_eq!(service_hash::GAME_UTILITIES_SERVICE_V2, 0x5DBB_51C2);

        assert_eq!(
            fnv1a32("bnet.protocol.authentication.v2.client.AuthenticationService"),
            service_hash::AUTHENTICATION_SERVICE_V2
        );
        assert_eq!(
            fnv1a32("bnet.protocol.authentication.v2.client.AuthenticationListener"),
            service_hash::AUTHENTICATION_LISTENER_V2
        );
    }
}
