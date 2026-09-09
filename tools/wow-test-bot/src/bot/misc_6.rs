//! Misc operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn cleanup_homebind_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &HomebindSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    let offline_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|e| anyhow!("Check homebind bot offline state before cleanup: {e}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => bail!(
                "character {} remained online during homebind cleanup",
                bot.character_guid
            ),
            None => bail!(
                "No characters row for guid {} during homebind cleanup",
                bot.character_guid
            ),
        }
    }

    let mut tx = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start homebind cleanup transaction: {e}"))?;
    tx.exec_drop(
        "UPDATE characters SET map = ?, zone = ?, instance_id = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?",
        (
            fixture.original_position.map_id,
            fixture.original_position.zone_id,
            fixture.original_position.instance_id,
            fixture.original_position.x,
            fixture.original_position.y,
            fixture.original_position.z,
            fixture.original_position.orientation,
            bot.character_guid,
        ),
    )
    .map_err(|e| anyhow!("Restore homebind bot position: {e}"))?;
    if let Some(homebind) = &fixture.original_homebind {
        tx.exec_drop(
            "INSERT INTO character_homebind (guid, mapId, zoneId, posX, posY, posZ, orientation) \
             VALUES (?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE mapId=VALUES(mapId), zoneId=VALUES(zoneId), posX=VALUES(posX), posY=VALUES(posY), posZ=VALUES(posZ), orientation=VALUES(orientation)",
            (
                bot.character_guid,
                homebind.map_id,
                homebind.zone_id,
                homebind.x,
                homebind.y,
                homebind.z,
                homebind.orientation,
            ),
        )
        .map_err(|e| anyhow!("Restore original character_homebind: {e}"))?;
    } else {
        tx.exec_drop(
            "DELETE FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Delete homebind fixture row: {e}"))?;
    }
    tx.commit()
        .map_err(|e| anyhow!("Commit homebind fixture cleanup: {e}"))?;
    Ok(())
}
pub(crate) fn verify_bank_fixture_location(
    bot: &config::BotConfig,
    item_guid: u64,
    expected_slot: u8,
) -> Result<bool> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    let row: Option<(u64, u8, u64, u64)> = conn
        .exec_first(
            "SELECT ci.bag, ci.slot, ii.owner_guid, ii.count \
             FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ci.item = ?",
            (bot.character_guid, item_guid),
        )
        .map_err(|e| anyhow!("Load bank fixture location: {e}"))?;
    Ok(matches!(
        row,
        Some((0, slot, owner, 1)) if slot == expected_slot && owner == bot.character_guid
    ))
}
pub(crate) fn cleanup_bank_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &BankSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    // A failed packet phase can drop the socket before the normal logout path.
    // Wait until the world server has finished its disconnect save before
    // deleting the fixture, otherwise that late save could recreate it or
    // overwrite the restored character position.
    let offline_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|e| anyhow!("Check bank bot offline state before cleanup: {e}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => {
                bail!(
                    "character {} remained online; refusing bank fixture cleanup before disconnect save",
                    bot.character_guid
                );
            }
            None => bail!(
                "No characters row for guid {} during cleanup",
                bot.character_guid
            ),
        }
    }

    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start bank cleanup transaction: {e}"))?;
    transaction
        .exec_drop(
            "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
            (bot.character_guid, fixture.options.item_guid),
        )
        .map_err(|e| anyhow!("Delete bank fixture inventory row: {e}"))?;
    transaction
        .exec_drop(
            "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?",
            (fixture.options.item_guid, bot.character_guid),
        )
        .map_err(|e| anyhow!("Delete bank fixture item: {e}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, zone = ?, instance_id = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ?",
            (
                fixture.original_position.map_id,
                fixture.original_position.zone_id,
                fixture.original_position.instance_id,
                fixture.original_position.x,
                fixture.original_position.y,
                fixture.original_position.z,
                fixture.original_position.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|e| anyhow!("Restore bank bot position: {e}"))?;
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit bank fixture cleanup: {e}"))?;
    info!(
        "Bank smoke fixture cleaned: character={} item={}",
        bot.character_guid, fixture.options.item_guid
    );
    Ok(())
}
pub(crate) fn database_url(env_name: &str, conf_key: &str) -> Result<String> {
    if let Ok(value) = std::env::var(env_name) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    database_url_from_worldserver_conf(conf_key).map_err(|e| {
        anyhow!(
            "{} is not set and {} could not be read from worldserver config: {}",
            env_name,
            conf_key,
            e
        )
    })
}
pub(crate) fn worldserver_config_f32(key: &str, default: f32) -> Result<f32> {
    let path = std::env::var("WOW_BOT_DB_CONF")
        .unwrap_or_else(|_| "/home/server/trinity-legacy-install/etc/worldserver.conf".to_string());
    let contents =
        std::fs::read_to_string(&path).map_err(|error| anyhow!("Read {path} failed: {error}"))?;
    worldserver_config_f32_from_contents(&contents, key, default)
        .with_context(|| format!("Read {key} from {path}"))
}
pub(crate) fn worldserver_config_u32(key: &str, default: u32) -> Result<u32> {
    let path = std::env::var("WOW_BOT_DB_CONF")
        .unwrap_or_else(|_| "/home/server/trinity-legacy-install/etc/worldserver.conf".to_string());
    let contents =
        std::fs::read_to_string(&path).map_err(|error| anyhow!("Read {path} failed: {error}"))?;
    worldserver_config_u32_from_contents(&contents, key, default)
        .with_context(|| format!("Read {key} from {path}"))
}
pub(crate) fn worldserver_config_u32_from_contents(
    contents: &str,
    key: &str,
    default: u32,
) -> Result<u32> {
    let mut effective = None;
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let Some((candidate_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        if !candidate_key.trim().eq_ignore_ascii_case(key) {
            continue;
        }
        effective = Some(raw_value.trim().trim_matches('"'));
    }
    let Some(value) = effective else {
        return Ok(default);
    };
    value
        .parse::<u32>()
        .map_err(|error| anyhow!("invalid {key} value `{value}`: {error}"))
}
pub(crate) fn worldserver_config_f32_from_contents(
    contents: &str,
    key: &str,
    default: f32,
) -> Result<f32> {
    let mut effective = None;
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let Some((candidate_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        if !candidate_key.trim().eq_ignore_ascii_case(key) {
            continue;
        }
        effective = Some(raw_value.trim().trim_matches('"'));
    }
    let Some(value) = effective else {
        return Ok(default);
    };
    value
        .parse::<f32>()
        .map_err(|error| anyhow!("invalid {key} value `{value}`: {error}"))
}
pub(crate) fn database_url_from_worldserver_conf(conf_key: &str) -> Result<String> {
    let path = std::env::var("WOW_BOT_DB_CONF")
        .unwrap_or_else(|_| "/home/server/trinity-legacy-install/etc/worldserver.conf".to_string());
    let contents =
        std::fs::read_to_string(&path).map_err(|e| anyhow!("Read {} failed: {}", path, e))?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(conf_key) {
            continue;
        }
        let value = trimmed
            .split_once('=')
            .map(|(_, v)| v.trim())
            .ok_or_else(|| anyhow!("Malformed {} line", conf_key))?;
        let value = value.trim_matches('"');
        let mut parts = value.split(';');
        let host = parts.next().ok_or_else(|| anyhow!("Missing DB host"))?;
        let port = parts.next().ok_or_else(|| anyhow!("Missing DB port"))?;
        let user = parts.next().ok_or_else(|| anyhow!("Missing DB user"))?;
        let password = parts.next().ok_or_else(|| anyhow!("Missing DB password"))?;
        let database = parts.next().ok_or_else(|| anyhow!("Missing DB name"))?;
        let mut url = String::from("mysql://");
        url.push_str(user);
        url.push(':');
        url.push_str(password);
        url.push('@');
        url.push_str(host);
        url.push(':');
        url.push_str(port);
        url.push('/');
        url.push_str(database);
        return Ok(url);
    }
    bail!("{} not found in {}", conf_key, path)
}
/// Derive the 16-byte AES-GCM encryption key the same way TrinityCore's worldserver
/// does: HMAC the 64-byte session_key_bnet to a 32-byte seed, expand that seed to 40
/// bytes through the SessionKeyGenerator cascade, then HMAC again for the final key.
/// Falling back to a "32 bytes || 8 zero bytes" shortcut (the previous implementation)
/// produced an HMAC key that disagreed with the server, so the very first server
/// packet failed AES-GCM tag verification.
pub(crate) fn derive_encryption_key(
    session_key: &[u8],
    local: &[u8; 16],
    server: &[u8; 16],
) -> [u8; 16] {
    let session_key_40 = derive_realm_session_key(session_key, local, server);
    srp6_auth::calculate_encrypt_key(&session_key_40, local, server)
}
pub(crate) fn derive_instance_encryption_key(
    session_key: &[u8],
    local: &[u8; 16],
    server: &[u8; 16],
) -> [u8; 16] {
    use hmac::{Hmac, Mac};

    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut mac = HmacSha256::new_from_slice(session_key).unwrap();
    mac.update(local);
    mac.update(server);
    mac.update(&ENCRYPTION_KEY_SEED);
    let result = mac.finalize().into_bytes();

    let mut key = [0u8; 16];
    key.copy_from_slice(&result[..16]);
    key
}
pub(crate) fn create_player_guid_raw(guid: u64, realm_id: u32) -> (u64, u64) {
    // C++ ObjectGuid::Create<HighGuid::Player>(realmId, guid).
    let high = (2u64 << 58) | ((u64::from(realm_id) & 0xFFFF) << 42);
    (guid, high)
}
pub(crate) fn create_creature_guid_raw(map_id: u16, entry: u32, counter: u64) -> (u64, u64) {
    let high = (8u64 << 58) | ((map_id as u64 & 0x1FFF) << 29) | ((entry as u64 & 0x7F_FFFF) << 6);
    let low = counter & OBJECT_GUID_COUNTER_MASK;
    (low, high)
}
pub(crate) fn resolve_vendor_runtime_target(
    target: &ResolvedCreatureTarget,
    discovered: Option<DiscoveredCreatureGuid>,
) -> Result<DiscoveredCreatureGuid> {
    let candidate = discovered.ok_or_else(|| {
        if target.guid_counter == 0 {
            anyhow!(
                "vendor entry {} spawn {} was not discovered near its SQL position in login SMSG_UPDATE_OBJECT packets",
                target.entry,
                target.spawn_guid
            )
        } else {
            anyhow!(
                "vendor runtime counter {} was not discovered near SQL spawn {}; the override cannot be linked safely",
                target.guid_counter & OBJECT_GUID_COUNTER_MASK,
                target.spawn_guid
            )
        }
    })?;
    if target.guid_counter != 0 {
        let expected = create_creature_guid_raw(target.map_id, target.entry, target.guid_counter);
        if (candidate.low, candidate.high) != expected {
            bail!(
                "vendor runtime counter override {} did not match discovered counter {} for SQL spawn {}",
                expected.0,
                candidate.low & OBJECT_GUID_COUNTER_MASK,
                target.spawn_guid
            );
        }
    }
    Ok(candidate)
}
pub(crate) fn resolve_rested_xp_runtime_target(
    target: &ResolvedCreatureTarget,
    discovered: Option<DiscoveredCreatureGuid>,
) -> Result<DiscoveredCreatureGuid> {
    let candidate = discovered.ok_or_else(|| {
        if target.guid_counter == 0 {
            anyhow!(
                "target entry {} spawn {} was not discovered near its SQL position in login SMSG_UPDATE_OBJECT packets; restart the QA world and retry",
                target.entry,
                target.spawn_guid
            )
        } else {
            anyhow!(
                "runtime counter {} for target entry {} spawn {} was not discovered near that spawn's SQL position; the override cannot be linked safely",
                target.guid_counter & OBJECT_GUID_COUNTER_MASK,
                target.entry,
                target.spawn_guid
            )
        }
    })?;

    if target.guid_counter != 0 {
        let expected = create_creature_guid_raw(target.map_id, target.entry, target.guid_counter);
        if (candidate.low, candidate.high) != expected {
            bail!(
                "runtime counter override {} did not match discovered counter {} for SQL spawn {}",
                expected.0,
                candidate.low & OBJECT_GUID_COUNTER_MASK,
                target.spawn_guid
            );
        }
    }

    Ok(candidate)
}
pub(crate) fn find_creature_guid_in_update_object(
    payload: &[u8],
    map_id: u16,
    entry: u32,
) -> Option<(u64, u64)> {
    // CreateObject blocks start with UpdateType (1/2) followed by a packed
    // ObjectGuid. Scanning is intentional: blocks are variable-sized, while
    // the GUID's high fields make a false match for type/map/entry negligible.
    for offset in 0..payload.len().saturating_sub(2) {
        if !matches!(payload[offset], 1 | 2) {
            continue;
        }
        let Some((_, low, high)) = parse_packed_guid(&payload[offset + 1..]) else {
            continue;
        };
        if ((high >> 58) & 0x3F) == 8
            && ((high >> 29) & 0x1FFF) == u64::from(map_id)
            && ((high >> 6) & 0x7F_FFFF) == u64::from(entry)
        {
            return Some((low, high));
        }
    }
    None
}
pub(crate) fn issue20_take_u8(data: &[u8], cursor: &mut usize, field: &str) -> Result<u8> {
    let value = *data
        .get(*cursor)
        .ok_or_else(|| anyhow!("truncated issue #20 item CreateObject at {field}"))?;
    *cursor += 1;
    Ok(value)
}
pub(crate) fn issue20_take_u16(data: &[u8], cursor: &mut usize, field: &str) -> Result<u16> {
    let end = cursor
        .checked_add(2)
        .ok_or_else(|| anyhow!("issue #20 item CreateObject {field} offset overflow"))?;
    let bytes: [u8; 2] = data
        .get(*cursor..end)
        .ok_or_else(|| anyhow!("truncated issue #20 item CreateObject at {field}"))?
        .try_into()?;
    *cursor = end;
    Ok(u16::from_le_bytes(bytes))
}
pub(crate) fn issue20_take_u32(data: &[u8], cursor: &mut usize, field: &str) -> Result<u32> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| anyhow!("issue #20 item CreateObject {field} offset overflow"))?;
    let bytes: [u8; 4] = data
        .get(*cursor..end)
        .ok_or_else(|| anyhow!("truncated issue #20 item CreateObject at {field}"))?
        .try_into()?;
    *cursor = end;
    Ok(u32::from_le_bytes(bytes))
}
pub(crate) fn issue20_take_u64(data: &[u8], cursor: &mut usize, field: &str) -> Result<u64> {
    let end = cursor
        .checked_add(8)
        .ok_or_else(|| anyhow!("issue #20 item CreateObject {field} offset overflow"))?;
    let bytes: [u8; 8] = data
        .get(*cursor..end)
        .ok_or_else(|| anyhow!("truncated issue #20 item CreateObject at {field}"))?
        .try_into()?;
    *cursor = end;
    Ok(u64::from_le_bytes(bytes))
}
pub(crate) fn issue20_take_i32(data: &[u8], cursor: &mut usize, field: &str) -> Result<i32> {
    Ok(issue20_take_u32(data, cursor, field)? as i32)
}
pub(crate) fn issue20_take_packed_guid(
    data: &[u8],
    cursor: &mut usize,
    field: &str,
) -> Result<(u64, u64)> {
    let (consumed, low, high) = parse_packed_guid(
        data.get(*cursor..)
            .ok_or_else(|| anyhow!("truncated issue #20 item CreateObject at {field}"))?,
    )
    .ok_or_else(|| anyhow!("invalid packed GUID in issue #20 item CreateObject at {field}"))?;
    *cursor = cursor
        .checked_add(consumed)
        .ok_or_else(|| anyhow!("issue #20 item CreateObject {field} offset overflow"))?;
    Ok((low, high))
}
pub(crate) fn parse_bind_point_update(
    payload: &[u8],
    expected_orientation: f32,
) -> Option<HomebindRowSnapshot> {
    if payload.len() != 20 {
        return None;
    }
    let x = f32::from_le_bytes(payload[0..4].try_into().ok()?);
    let y = f32::from_le_bytes(payload[4..8].try_into().ok()?);
    let z = f32::from_le_bytes(payload[8..12].try_into().ok()?);
    let map_id = u16::try_from(i32::from_le_bytes(payload[12..16].try_into().ok()?)).ok()?;
    let zone_id = u16::try_from(i32::from_le_bytes(payload[16..20].try_into().ok()?)).ok()?;
    Some(HomebindRowSnapshot {
        map_id,
        zone_id,
        x,
        y,
        z,
        orientation: expected_orientation,
    })
}
pub(crate) fn take_packed_guid(data: &[u8], position: &mut usize) -> Option<(u64, u64)> {
    let (consumed, low, high) = parse_packed_guid(data.get(*position..)?)?;
    *position = position.checked_add(consumed)?;
    Some((low, high))
}
pub(crate) fn take_u32(data: &[u8], position: &mut usize) -> Option<u32> {
    let bytes: [u8; 4] = data
        .get(*position..position.checked_add(4)?)?
        .try_into()
        .ok()?;
    *position = position.checked_add(4)?;
    Some(u32::from_le_bytes(bytes))
}
pub(crate) fn take_u8(data: &[u8], position: &mut usize) -> Option<u8> {
    let value = *data.get(*position)?;
    *position = position.checked_add(1)?;
    Some(value)
}
pub(crate) fn player_bound_matches(
    payload: &[u8],
    expected_low: u64,
    expected_high: u64,
    expected_area_id: u32,
) -> bool {
    let Some((consumed, low, high)) = parse_packed_guid(payload) else {
        return false;
    };
    payload
        .get(consumed..consumed + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        == Some(expected_area_id)
        && low == expected_low
        && high == expected_high
        && payload.len() == consumed + 4
}
pub(crate) fn build_packed_guid(low: u64, high: u64) -> Vec<u8> {
    let (low_mask, low_bytes) = pack_u64(low);
    let (high_mask, high_bytes) = pack_u64(high);
    let mut data = Vec::with_capacity(2 + low_bytes.len() + high_bytes.len());
    data.push(low_mask);
    data.push(high_mask);
    data.extend_from_slice(&low_bytes);
    data.extend_from_slice(&high_bytes);
    data
}
pub(crate) fn build_gossip_select_option(
    packed_guid: &[u8],
    gossip_id: i32,
    gossip_option_id: i32,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(packed_guid.len() + 9);
    data.extend_from_slice(packed_guid);
    data.extend_from_slice(&gossip_id.to_le_bytes());
    data.extend_from_slice(&gossip_option_id.to_le_bytes());
    data.push(0x00); // PromotionCode length = 0, written as 8 MSB-first bits.
    data
}
pub(crate) fn validate_pinned_instance_port(
    advertised_port: u16,
    expected_port: Option<&str>,
) -> Result<()> {
    let Some(expected_port) = expected_port else {
        return Ok(());
    };
    let expected_port = expected_port
        .parse::<u16>()
        .with_context(|| "INSTANCE_PORT must be a valid nonzero TCP port")?;
    if expected_port == 0 {
        bail!("INSTANCE_PORT must be a valid nonzero TCP port");
    }
    if advertised_port != expected_port {
        bail!(
            "SMSG_CONNECT_TO advertised instance port {}, expected pinned INSTANCE_PORT {}",
            advertised_port,
            expected_port
        );
    }
    Ok(())
}
/// Pack u64 into WoW's packed format (mask + non-zero bytes)
pub(crate) fn pack_u64(value: u64) -> (u8, Vec<u8>) {
    let mut mask = 0u8;
    let mut bytes = Vec::new();

    for i in 0..8 {
        let b = (value >> (i * 8)) as u8;
        if b != 0 {
            mask |= 1 << i;
            bytes.push(b);
        }
    }

    (mask, bytes)
}
/// Echo the Ticket+InstanceID+ProposalID prefix from a SMSG_LFG_PROPOSAL_UPDATE
/// payload back into a CMSG_DF_PROPOSAL_RESPONSE body, with `Accepted=true`.
///
/// SMSG_LFG_PROPOSAL_UPDATE layout (LFGPackets.cpp:375):
///   Ticket {
///     ObjectGuid RequesterGuid  // 1 lowMask + 1 highMask + popcnt(low)+popcnt(high) bytes
///     uint32 Id, uint32 Type, uint64 Time
///     bit Unknown925, FlushBits  // 1 byte
///   }
///   uint64 InstanceID, uint32 ProposalID, ... rest we ignore
///
/// CMSG_DF_PROPOSAL_RESPONSE just wants the same Ticket+InstanceID+ProposalID
/// prefix plus a single Accepted bit.
pub(crate) fn build_proposal_response(payload: &[u8]) -> Option<Vec<u8>> {
    if payload.len() < 2 {
        return None;
    }
    let low_mask = payload[0];
    let high_mask = payload[1];
    let guid_len = 2 + (low_mask.count_ones() + high_mask.count_ones()) as usize;
    let prefix_len = guid_len
        + 4  // Id
        + 4  // Type
        + 8  // Time
        + 1  // bit byte (Unknown925 + flush)
        + 8  // InstanceID
        + 4; // ProposalID
    if payload.len() < prefix_len {
        return None;
    }
    let mut response = Vec::with_capacity(prefix_len + 1);
    response.extend_from_slice(&payload[..prefix_len]);
    response.push(0x80); // Accepted=true bit, MSB-first per Trinity's WriteBit
    Some(response)
}
/// Build CMSG_DF_JOIN body — layout matches main_srp6_complete.rs::build_lfg_join_packet,
/// which is what the 3.4.3 server's DF_JOIN handler parses.
///
///   bits[8]:  QueueAsGroup | hasPartyIndex | Unknown | 5b padding → all zero
///   u8:       Roles bitmask (2=Tank, 4=Healer, 8=DPS)
///   u32 LE:   NumDungeons (always 1)
///   u32 LE:   DungeonID
pub(crate) fn build_lfg_join(dungeon_id: u32, roles: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(10);
    data.push(0x00);
    data.push(roles);
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&dungeon_id.to_le_bytes());
    data
}
