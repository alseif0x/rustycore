//! Build-70170 LastChar also selects a realm for a new account's ruleset.
//! Source: advocaite/TrinityCore 02245dcd GetLastCharPlayed; approved client
//! callback RVA 0x22ae8e0 requires RealmEntry/blob then LastPlayedTime/int.

use super::*;
use crate::realm::{RealmManager, forever::ForeverCatalog};

pub(super) fn last_character(
    catalog: &ForeverCatalog,
    realms: &RealmManager,
    game: &GameAccountInfo,
    request: &ClientRequest,
    now: u64,
) -> Result<ClientResponse> {
    let sub_region =
        find_command_param_like_cpp(&request.attribute, "Command_LastCharPlayedRequest_v1")
            .and_then(|a| a.value.string_value.as_deref())
            .ok_or_else(|| RpcStatusError::new(status::ERROR_UTIL_SERVER_UNKNOWN_REALM))?;
    let filter = content_filter(&request.attribute)?;
    let (address, last) = if let Some(content) = filter {
        let Some(address) = catalog.realm_for_content(content) else {
            return Ok(ClientResponse::default());
        };
        // Auth already owns this snapshot. Match the full realm address so
        // changing a ruleset cannot return another ruleset's character.
        let last = game
            .last_played_chars
            .values()
            .filter(|last| last.realm_address == address)
            .max_by_key(|last| last.last_played_time);
        (address, last)
    } else if let Some(last) = game.last_played_chars.get(sub_region) {
        (last.realm_address, Some(last))
    } else {
        return Ok(ClientResponse::default());
    };
    let Some(entry) = catalog.realm_entry(realms, address, 70170, game.security_level, now) else {
        return if last.is_some() {
            Err(RpcStatusError::new(status::ERROR_UTIL_SERVER_FAILED_TO_SERIALIZE_RESPONSE).into())
        } else {
            Ok(ClientResponse::default())
        };
    };
    let mut attributes = if let Some(last) = last {
        let time = i64::try_from(last.last_played_time)
            .map_err(|_| RpcStatusError::new(status::ERROR_INTERNAL))?;
        vec![
            make_blob_attribute("Param_RealmEntry", &entry),
            make_string_attribute("Param_CharacterName", &last.character_name),
            make_blob_attribute("Param_CharacterGUID", &last.character_guid.to_le_bytes()),
            make_int_attribute("Param_LastPlayedTime", time),
        ]
    } else {
        // Routing timestamp, like the Forever C++ reference; no character,
        // GUID, cache entry or DB last-played row is fabricated or persisted.
        let time = i64::try_from(now).map_err(|_| RpcStatusError::new(status::ERROR_INTERNAL))?;
        vec![
            make_blob_attribute("Param_RealmEntry", &entry),
            Attribute {
                name: "Param_LastPlayedTime".into(),
                value: Variant {
                    int_value: Some(time),
                    ..Default::default()
                },
            },
        ]
    };
    attributes.push(make_blob_attribute(
        "Param_UtilityInfo",
        &crate::realm::forever::compressed_utility_info(),
    ));
    Ok(ClientResponse {
        attribute: attributes,
    })
}

fn content_filter(attrs: &[Attribute]) -> Result<Option<i32>> {
    let Some(attr) = find_param_like_cpp(attrs, "Param_ContentSetIDFilter") else {
        return Ok(None);
    };
    // Real 70170 selection uses signed Variant (136 for the observed PvP
    // card); -1 is the unfiltered sentinel. Reject coercion and overflow.
    match attr.value.int_value {
        Some(-1) => Ok(None),
        Some(value) if (0..=i32::MAX as i64).contains(&value) => Ok(Some(value as i32)),
        _ => Err(RpcStatusError::new(status::ERROR_RPC_MALFORMED_REQUEST).into()),
    }
}

#[cfg(test)]
mod tests;
