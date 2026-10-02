//! Secret-safe diagnostics for the build-70170 discovery requests.
//!
//! This module deliberately projects only an allowlist of request fields.  It
//! must remain safe to enable at debug level while investigating the client
//! transition from SuperDistrict discovery to realm discovery.

use wow_proto::bgs::protocol::{Attribute, Variant};

const LAST_CHAR: &str = "Command_LastCharPlayedRequest_v1";
const REALM_LIST: &str = "Command_RealmListRequest_v1";
const REALM_JOIN: &str = "Command_RealmJoinRequest_v1";
const SUPER_DISTRICT: &str = "Command_SuperDistrictListRequest_v1";
const MAX_TRACE_FIELDS: usize = 16;

#[derive(Debug, PartialEq, Eq)]
struct TraceProjection {
    command: &'static str,
    attr_count: usize,
    known: Vec<TraceAttribute>,
    unknown_count: usize,
    omitted_known_count: usize,
}

#[derive(Debug, PartialEq, Eq)]
enum TraceAttribute {
    Command {
        value: CommandValue,
    },
    Param {
        name: &'static str,
        value: ParamValue,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum CommandValue {
    SubRegion { region: u32, site: u32, realm: u32 },
    Invalid { kind: ValueKind, len: Option<usize> },
    Empty,
    Multiple,
}

#[derive(Debug, PartialEq, Eq)]
enum ParamValue {
    Bool(bool),
    Uint32(u32),
    Int32(i32),
    StringLen(usize),
    BlobLen(usize),
    Rejected(ValueKind),
    Empty,
    Multiple,
}

#[derive(Debug, PartialEq, Eq)]
enum ValueKind {
    Bool,
    Int,
    Float,
    String,
    Blob,
    Message,
    Fourcc,
    Uint,
    EntityId,
    Empty,
    Multiple,
}

/// Emit a bounded, secret-safe projection for discovery and the join transition.
pub(super) fn trace_discovery(command: &str, attrs: &[Attribute]) {
    if !tracing::enabled!(target: "bnet.discovery", tracing::Level::DEBUG) {
        return;
    }
    let Some(projection) = project_discovery(command, attrs) else {
        return;
    };

    tracing::debug!(
        target: "bnet.discovery",
        command = projection.command,
        attr_count = projection.attr_count,
        unknown_count = projection.unknown_count,
        omitted_known_count = projection.omitted_known_count,
        known = ?projection.known,
        "safe discovery request"
    );
}

fn project_discovery(command: &str, attrs: &[Attribute]) -> Option<TraceProjection> {
    let command = match command {
        LAST_CHAR => LAST_CHAR,
        REALM_LIST => REALM_LIST,
        REALM_JOIN => REALM_JOIN,
        SUPER_DISTRICT => SUPER_DISTRICT,
        _ => return None,
    };

    let mut known = Vec::new();
    let mut unknown_count = 0;
    let mut omitted_known_count = 0;
    for attr in attrs {
        if known.len() >= MAX_TRACE_FIELDS
            && (is_command_attribute(&attr.name, command) || known_param(&attr.name).is_some())
        {
            omitted_known_count += 1;
            continue;
        }
        if is_command_attribute(&attr.name, command) {
            known.push(TraceAttribute::Command {
                value: command_value(&attr.value),
            });
        } else if let Some(name) = known_param(&attr.name) {
            known.push(TraceAttribute::Param {
                name,
                value: param_value(&attr.value),
            });
        } else {
            unknown_count += 1;
        }
    }

    Some(TraceProjection {
        command,
        attr_count: attrs.len(),
        known,
        unknown_count,
        omitted_known_count,
    })
}

fn is_command_attribute(name: &str, command: &str) -> bool {
    name == command
        || name
            .strip_prefix(command)
            .is_some_and(|suffix| suffix.starts_with('_'))
}

fn known_param(name: &str) -> Option<&'static str> {
    match name {
        "Param_ContentSetIDFilter" => Some("Param_ContentSetIDFilter"),
        "Param_FilterToPreferredLocality" => Some("Param_FilterToPreferredLocality"),
        "Param_RealmAddress" => Some("Param_RealmAddress"),
        "Param_SuperDistrictID" => Some("Param_SuperDistrictID"),
        _ => None,
    }
}

fn command_value(value: &Variant) -> CommandValue {
    match value_kind(value) {
        ValueKind::String => {
            let string = value.string_value.as_deref().unwrap_or_default();
            match parse_sub_region(string) {
                Some((region, site, realm)) => CommandValue::SubRegion {
                    region,
                    site,
                    realm,
                },
                None => CommandValue::Invalid {
                    kind: ValueKind::String,
                    len: Some(string.len()),
                },
            }
        }
        ValueKind::Blob => CommandValue::Invalid {
            kind: ValueKind::Blob,
            len: value.blob_value.as_ref().map(Vec::len),
        },
        ValueKind::Empty => CommandValue::Empty,
        ValueKind::Multiple => CommandValue::Multiple,
        kind => CommandValue::Invalid { kind, len: None },
    }
}

fn param_value(value: &Variant) -> ParamValue {
    match value_kind(value) {
        ValueKind::Bool => ParamValue::Bool(value.bool_value.unwrap_or_default()),
        ValueKind::Uint => match value.uint_value {
            Some(value) if value <= u32::MAX as u64 => ParamValue::Uint32(value as u32),
            _ => ParamValue::Rejected(ValueKind::Uint),
        },
        ValueKind::Int => match value.int_value {
            Some(value) if (i32::MIN as i64..=i32::MAX as i64).contains(&value) => {
                ParamValue::Int32(value as i32)
            }
            _ => ParamValue::Rejected(ValueKind::Int),
        },
        ValueKind::String => {
            ParamValue::StringLen(value.string_value.as_deref().map_or(0, str::len))
        }
        ValueKind::Blob => ParamValue::BlobLen(value.blob_value.as_ref().map_or(0, Vec::len)),
        ValueKind::Empty => ParamValue::Empty,
        ValueKind::Multiple => ParamValue::Multiple,
        kind => ParamValue::Rejected(kind),
    }
}

fn parse_sub_region(value: &str) -> Option<(u32, u32, u32)> {
    if value.is_empty() || value.len() > 32 || !value.is_ascii() {
        return None;
    }
    let mut parts = value.split('-');
    let parse_group = |group: &str| -> Option<u32> {
        if group.is_empty() || !group.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        group.parse::<u32>().ok()
    };
    let region = parse_group(parts.next()?)?;
    let site = parse_group(parts.next()?)?;
    let realm = parse_group(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((region, site, realm))
}

fn value_kind(value: &Variant) -> ValueKind {
    let kinds = [
        value.bool_value.is_some(),
        value.int_value.is_some(),
        value.float_value.is_some(),
        value.string_value.is_some(),
        value.blob_value.is_some(),
        value.message_value.is_some(),
        value.fourcc_value.is_some(),
        value.uint_value.is_some(),
        value.entity_id_value.is_some(),
    ];
    if kinds.iter().filter(|present| **present).count() != 1 {
        return if kinds.iter().any(|present| *present) {
            ValueKind::Multiple
        } else {
            ValueKind::Empty
        };
    }
    match kinds.iter().position(|present| *present).unwrap() {
        0 => ValueKind::Bool,
        1 => ValueKind::Int,
        2 => ValueKind::Float,
        3 => ValueKind::String,
        4 => ValueKind::Blob,
        5 => ValueKind::Message,
        6 => ValueKind::Fourcc,
        7 => ValueKind::Uint,
        _ => ValueKind::EntityId,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(name: &str, value: Variant) -> Attribute {
        Attribute {
            name: name.to_string(),
            value,
        }
    }

    fn string(value: &str) -> Variant {
        Variant {
            string_value: Some(value.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn projects_only_allowlisted_values_without_secret_material() {
        let marker = "ticket-secret-marker";
        let projection = project_discovery(
            LAST_CHAR,
            &[
                attr("Command_LastCharPlayedRequest_v1_classic", string("2-1-0")),
                attr("Param_ContentSetIDFilter", string(marker)),
                attr(
                    "Param_FilterToPreferredLocality",
                    Variant {
                        blob_value: Some(marker.as_bytes().to_vec()),
                        ..Default::default()
                    },
                ),
                attr(
                    "Param_RealmAddress",
                    Variant {
                        uint_value: Some(0x0102_0304),
                        ..Default::default()
                    },
                ),
                attr(
                    "Param_UnknownTicket",
                    Variant {
                        blob_value: Some(marker.as_bytes().to_vec()),
                        ..Default::default()
                    },
                ),
            ],
        )
        .unwrap();
        let debug = format!("{projection:?}");
        assert!(debug.contains("SubRegion"));
        assert!(debug.contains("StringLen"));
        assert!(debug.contains("BlobLen"));
        assert!(debug.contains("Uint32"));
        assert!(!debug.contains(marker));
        assert!(!debug.contains("Param_UnknownTicket"));
        assert_eq!(projection.attr_count, 5);
        assert_eq!(projection.unknown_count, 1);
    }

    #[test]
    fn command_subregion_requires_three_ascii_u32_groups() {
        assert_eq!(parse_sub_region("2-1-0"), Some((2, 1, 0)));
        for invalid in [
            "2-1",
            "2-1-0-4",
            "2-one-0",
            "2-1-4294967296",
            "2-1-0\n",
            "2-1-€",
        ] {
            let projection = project_discovery(
                REALM_LIST,
                &[attr("Command_RealmListRequest_v1", string(invalid))],
            )
            .unwrap();
            let debug = format!("{projection:?}");
            assert!(!debug.contains(invalid));
            assert!(debug.contains("Invalid"));
        }
    }

    #[test]
    fn duplicate_known_attributes_cannot_expand_the_trace_without_bound() {
        let attrs = vec![attr("Param_RealmAddress", string("secret-marker")); 100];
        let projection = project_discovery(LAST_CHAR, &attrs).unwrap();
        assert_eq!(projection.known.len(), MAX_TRACE_FIELDS);
        assert_eq!(projection.omitted_known_count, 100 - MAX_TRACE_FIELDS);
        assert_eq!(projection.attr_count, 100);
        assert!(!format!("{projection:?}").contains("secret-marker"));
    }

    #[test]
    fn numeric_params_accept_only_bounded_i32_and_u32_domains() {
        for outside in [i32::MIN as i64 - 1, i32::MAX as i64 + 1] {
            assert_eq!(
                param_value(&Variant {
                    int_value: Some(outside),
                    ..Default::default()
                }),
                ParamValue::Rejected(ValueKind::Int)
            );
        }
        let projection = project_discovery(
            SUPER_DISTRICT,
            &[
                attr("Command_SuperDistrictListRequest_v1", string("2-1-0")),
                attr(
                    "Param_SuperDistrictID",
                    Variant {
                        int_value: Some(7),
                        ..Default::default()
                    },
                ),
                attr(
                    "Param_RealmAddress",
                    Variant {
                        uint_value: Some(u32::MAX as u64 + 1),
                        ..Default::default()
                    },
                ),
                attr(
                    "Param_ContentSetIDFilter",
                    Variant {
                        int_value: Some(-1),
                        ..Default::default()
                    },
                ),
            ],
        )
        .unwrap();
        assert!(matches!(
            &projection.known[1],
            TraceAttribute::Param {
                value: ParamValue::Int32(7),
                ..
            }
        ));
        assert!(matches!(
            &projection.known[2],
            TraceAttribute::Param {
                value: ParamValue::Rejected(ValueKind::Uint),
                ..
            }
        ));
        assert!(matches!(
            &projection.known[3],
            TraceAttribute::Param {
                value: ParamValue::Int32(-1),
                ..
            }
        ));
    }

    #[test]
    fn malformed_values_and_unknown_commands_are_not_projected() {
        let marker = "secret-marker";
        let projection = project_discovery(
            SUPER_DISTRICT,
            &[attr(
                "Command_SuperDistrictListRequest_v1",
                Variant {
                    string_value: Some(marker.to_string()),
                    blob_value: Some(marker.as_bytes().to_vec()),
                    ..Default::default()
                },
            )],
        )
        .unwrap();
        let debug = format!("{projection:?}");
        assert!(debug.contains("Multiple"));
        assert!(!debug.contains(marker));
        assert!(project_discovery("Command_Unknown_v1", &[]).is_none());
    }
}
