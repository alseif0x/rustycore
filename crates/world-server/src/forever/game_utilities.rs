//! World-owned GameUtilities responses needed by character selection.
use super::bootstrap::Runtime;
use anyhow::{Context, Result, bail};
use flate2::{Compression, write::ZlibEncoder};
use prost::Message;
use serde_json::json;
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use wow_packet::forever::battlenet::{BattlenetRequest, BattlenetResponse};
use wow_proto::bgs::protocol::game_utilities::v2::client::{
    ProcessTaskRequest, ProcessTaskResponse,
};

const GAME_UTILITIES_SERVICE_V2: u32 = 0x5DBB51C2;
const PROCESS_TASK_METHOD_ID: u32 = 0x40000001;

pub(super) async fn respond(
    runtime: &Runtime,
    request: &BattlenetRequest,
) -> Result<Option<BattlenetResponse>> {
    if request.service_hash() != GAME_UTILITIES_SERVICE_V2
        || request.method_id() != PROCESS_TASK_METHOD_ID
    {
        return Ok(None);
    }
    let task = ProcessTaskRequest::decode(request.data()).context("GameUtilities request")?;
    let Some(command) = task.attribute.iter().find_map(|attribute| {
        attribute
            .name
            .as_deref()
            .filter(|name| name.starts_with("Command_"))
    }) else {
        bail!("GameUtilities command missing");
    };
    let command = command.rsplit_once('_').map_or(command, |(name, _)| name);
    if command != "Command_LastCharPlayedRequest_v1" {
        return Ok(None);
    }
    if let Some(filter) = task.attribute.iter().find_map(|attribute| {
        (attribute.name.as_deref() == Some("Param_ContentSetIDFilter"))
            .then(|| match attribute.value.as_ref()?.r#type.as_ref()? {
                wow_proto::bgs::protocol::v2::variant::Type::IntValue(value) => Some(*value),
                _ => None,
            })
            .flatten()
    }) {
        if filter >= 0 && filter != i64::from(runtime.policy.content_set) {
            return Ok(Some(BattlenetResponse::success(
                request.service_hash(),
                request.method_id(),
                request.token(),
                ProcessTaskResponse::default().encode_to_vec(),
            )));
        }
    }

    let realm = runtime
        .auth
        .direct_query(&format!(
            "SELECT name,timezone,icon,flag,population FROM realmlist WHERE id=1 AND gamebuild={} AND Region=2 AND Battlegroup=1 AND port=18085 AND flag=0 AND allowedSecurityLevel=0",
            runtime.build
        ))
        .await
        .context("realm query")?;
    let realm_name: String = realm.try_read(0).context("realm name")?;
    let timezone: u32 = realm.try_read(1).context("realm timezone")?;
    let icon: u8 = realm.try_read(2).context("realm type")?;
    let population: f32 = realm.try_read(4).context("realm population")?;
    let build = runtime
        .auth
        .direct_query(&format!(
            "SELECT majorVersion,minorVersion,bugfixVersion FROM build_info WHERE build={}",
            runtime.build
        ))
        .await
        .context("build query")?;
    let major = u32::try_from(build.try_read::<i32>(0).context("build major")?)
        .context("negative build major")?;
    let minor = u32::try_from(build.try_read::<i32>(1).context("build minor")?)
        .context("negative build minor")?;
    let revision = u32::try_from(build.try_read::<i32>(2).context("build revision")?)
        .context("negative build revision")?;
    let entry = json!({
        "wowRealmAddress": 0x02010001,
        "cfgTimezonesID": 1,
        "populationState": population_state(population),
        "cfgCategoriesID": timezone,
        "version": {
            "versionMajor": major,
            "versionMinor": minor,
            "versionRevision": revision,
            "versionBuild": runtime.build,
        },
        "cfgRealmsID": 1,
        "flags": 0,
        "name": realm_name,
        "cfgConfigsID": if icon == 16 { 2 } else if icon >= 14 { 1 } else { icon + 1 },
        "cfgLanguagesID": 1,
        "cfgContentSetID": runtime.policy.content_set,
        "superDistrictID": runtime.super_district,
        "useBleepChance": 0.0,
    });
    let now = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?;
    let attributes = vec![
        blob_attribute(
            "Param_RealmEntry",
            &zlib(format!("JamJSONRealmEntry:{entry}\0").as_bytes())?,
        ),
        wow_proto::bgs::protocol::v2::Attribute {
            name: Some("Param_LastPlayedTime".into()),
            value: Some(wow_proto::bgs::protocol::v2::Variant {
                r#type: Some(wow_proto::bgs::protocol::v2::variant::Type::IntValue(now)),
            }),
        },
        blob_attribute(
            "Param_UtilityInfo",
            &zlib(b"JSONUtilityInfo:{\"realmPermissions\":512}\0")?,
        ),
    ];
    Ok(Some(BattlenetResponse::success(
        request.service_hash(),
        request.method_id(),
        request.token(),
        ProcessTaskResponse { result: attributes }.encode_to_vec(),
    )))
}

fn population_state(population: f32) -> u8 {
    if population > 0.95 {
        6
    } else if population > 0.66 {
        3
    } else if population > 0.33 {
        2
    } else {
        1
    }
}

fn blob_attribute(name: &str, data: &[u8]) -> wow_proto::bgs::protocol::v2::Attribute {
    wow_proto::bgs::protocol::v2::Attribute {
        name: Some(name.into()),
        value: Some(wow_proto::bgs::protocol::v2::Variant {
            r#type: Some(wow_proto::bgs::protocol::v2::variant::Type::BlobValue(
                data.to_vec(),
            )),
        }),
    }
}

fn zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    let mut output = (data.len() as u32).to_le_bytes().to_vec();
    output.extend_from_slice(&compressed);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utility_blob_has_exact_size_and_zlib_envelope() {
        let blob = zlib(b"JSONUtilityInfo:{\"realmPermissions\":512}\0").unwrap();
        assert_eq!(&blob[..4], &41_u32.to_le_bytes());
        let mut decoder = flate2::read::ZlibDecoder::new(&blob[4..]);
        let mut text = String::new();
        decoder.read_to_string(&mut text).unwrap();
        assert_eq!(text, "JSONUtilityInfo:{\"realmPermissions\":512}\0");
    }

    #[test]
    fn population_matches_reference_thresholds() {
        for (value, expected) in [(0.0, 1), (0.34, 2), (0.67, 3), (0.96, 6)] {
            assert_eq!(population_state(value), expected);
        }
    }
}
