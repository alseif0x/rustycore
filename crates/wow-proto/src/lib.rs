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
//! - `bgs::protocol::connection::v1` — Connection messages
//! - `bgs::protocol::challenge::v1` — Challenge messages
//! - `bgs::protocol::game_utilities::v1` — GameUtilities messages
//! - `bgs::protocol::account::v1` — Account messages

// Include the generated protobuf code.
// prost-build generates one file per package, named by the package path.
pub mod bgs {
    pub mod protocol {
        // Core types: Header, ProcessId, NoData, EntityId, Attribute, Variant, etc.
        include!(concat!(env!("OUT_DIR"), "/bgs.protocol.rs"));

        pub mod authentication {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/bgs.protocol.authentication.v1.rs"));
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
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/bgs.protocol.game_utilities.v1.rs"));
            }
        }

        pub mod account {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/bgs.protocol.account.v1.rs"));
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
    pub const CONNECTION_SERVICE: u32 = 0x6544_6991;
    pub const ACCOUNT_SERVICE: u32 = 0x62DA_0891;
    pub const GAME_UTILITIES_SERVICE: u32 = 0x3FC1_274D;

    // Client-side listeners (server sends notifications to client)
    pub const AUTHENTICATION_LISTENER: u32 = 0x7124_0E35;
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
    pub const ERROR_DENIED: u32 = 3;
    pub const ERROR_NO_GAME_ACCOUNT: u32 = 12;
    pub const ERROR_WOW_SERVICES_GAME_ACCOUNT_LOCKED: u32 = 0x0002_0014;
    pub const ERROR_GAME_ACCOUNT_BANNED: u32 = 0x0002_000B;
    pub const ERROR_GAME_ACCOUNT_SUSPENDED: u32 = 0x0002_000C;
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
        assert_eq!(decoded.service_hash, Some(service_hash::AUTHENTICATION_SERVICE));
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
            client_id: Some(ProcessId { label: 1, epoch: 100 }),
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
            attribute: vec![
                Attribute {
                    name: "Command_RealmListRequest_v1".to_string(),
                    value: Variant {
                        string_value: Some("test".to_string()),
                        ..Default::default()
                    },
                },
            ],
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
        assert_eq!(service_hash::CONNECTION_SERVICE, 0x6544_6991);
        assert_eq!(service_hash::ACCOUNT_SERVICE, 0x62DA_0891);
        assert_eq!(service_hash::GAME_UTILITIES_SERVICE, 0x3FC1_274D);
        assert_eq!(service_hash::AUTHENTICATION_LISTENER, 0x7124_0E35);
        assert_eq!(service_hash::CHALLENGE_LISTENER, 0xBBDA_171F);
    }
}
