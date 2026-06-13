//! Realm list management.
//!
//! Periodically polls the `realmlist` table and provides realm data to clients.

use anyhow::Result;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use crate::state::AppState;
use wow_database::LoginStatements;

const REALM_FLAG_VERSION_MISMATCH: u8 = 0x01;
const REALM_FLAG_OFFLINE: u8 = 0x02;
const REALM_TYPE_NORMAL: u8 = 0;
const REALM_TYPE_PVP: u8 = 1;
const MAX_CLIENT_REALM_TYPE: u8 = 14;
const REALM_TYPE_FFA_PVP: u8 = 16;
const SEC_ADMINISTRATOR: u8 = 3;
const DEFAULT_VERSION_MAJOR: u32 = 6;
const DEFAULT_VERSION_MINOR: u32 = 2;
const DEFAULT_VERSION_REVISION: u32 = 4;

/// A single realm entry from the `realmlist` table.
#[derive(Debug, Clone)]
pub struct Realm {
    pub id: u32,
    pub name: String,
    #[allow(dead_code)]
    pub normalized_name: String,
    pub external_address: String,
    pub local_address: String,
    pub port: u16,
    pub icon: u8,
    pub flag: u8,
    pub timezone: u8,
    pub allowed_security_level: u8,
    pub population: f32,
    pub build: u32,
    pub region: u8,
    pub battlegroup: u8,
}

/// Build info from the `build_info` table.
#[derive(Debug, Clone)]
pub struct RealmBuildInfo {
    pub major_version: u32,
    pub minor_version: u32,
    pub bugfix_version: u32,
    pub hotfix_version: String,
    pub build: u32,
    pub win64_auth_seed: Option<Vec<u8>>,
    pub mac64_auth_seed: Option<Vec<u8>>,
}

/// Manages the list of available realms.
pub struct RealmManager {
    pub realms: HashMap<u32, Realm>,
    pub builds: Vec<RealmBuildInfo>,
    pub sub_regions: Vec<String>,
}

impl RealmManager {
    pub fn new() -> Self {
        Self {
            realms: HashMap::new(),
            builds: Vec::new(),
            sub_regions: Vec::new(),
        }
    }

    /// Find a realm by its external or local address + port.
    pub fn find_realm_by_address(&self, address: &str, port: u16) -> Option<&Realm> {
        self.realms.values().find(|r| {
            r.port == port && (r.external_address == address || r.local_address == address)
        })
    }

    /// Find a realm from Battlenet::RealmHandle::GetAddress() like C++ JoinRealm.
    ///
    /// TrinityCore constructs `RealmHandle(realmAddress)` and `RealmHandle`
    /// equality/order only compares the `Realm` field, so the lookup resolves
    /// the low 16-bit realmlist id rather than the whole packed address.
    pub fn get_realm_by_realm_address_like_cpp(&self, realm_address: u32) -> Option<&Realm> {
        self.realms
            .get(&(realm_id_from_address_like_cpp(realm_address)))
    }

    #[allow(dead_code)]
    pub fn get_realm_names_like_cpp(&self, realm_address: u32) -> Option<(String, String)> {
        self.get_realm_by_realm_address_like_cpp(realm_address)
            .map(|realm| (realm.name.clone(), realm.normalized_name.clone()))
    }

    /// Get build info for a specific build number.
    pub fn get_build_info(&self, build: u32) -> Option<&RealmBuildInfo> {
        self.builds.iter().find(|b| b.build == build)
    }

    #[allow(dead_code)]
    pub fn get_minor_major_bugfix_version_for_build_like_cpp(&self, build: u32) -> u32 {
        self.builds
            .iter()
            .find(|build_info| build_info.build >= build)
            .map(|build_info| {
                build_info.major_version * 10_000
                    + build_info.minor_version * 100
                    + build_info.bugfix_version
            })
            .unwrap_or(0)
    }

    /// Generate compressed JSON realm list for a specific build and sub-region.
    ///
    /// Matches C# RealmManager.GetRealmList() logic:
    /// - All realms are included (not filtered by build)
    /// - VersionMismatch flag (0x01) added dynamically if build doesn't match
    /// - PopulationState = 0 if offline, else max(population_level, 1)
    pub fn get_realm_list_json(
        &self,
        build: u32,
        _sub_region: &str,
        char_counts: &HashMap<u32, u8>,
    ) -> (Vec<u8>, Vec<u8>) {
        let updates: Vec<RealmListUpdate> = self
            .realms
            .values()
            .filter(|r| realm_sub_region_address_like_cpp(r.region, r.battlegroup) == _sub_region)
            .map(|r| {
                let build_info = self.get_build_info(r.build);

                // Dynamically add VersionMismatch if client build != realm build
                let mut flags = r.flag;
                if r.build != build {
                    flags |= REALM_FLAG_VERSION_MISMATCH;
                }

                // Population: 0 if offline, else max(population_level, 1)
                let is_offline = (flags & REALM_FLAG_OFFLINE) != 0;
                let population_state = if is_offline {
                    0
                } else {
                    (r.population as i32).max(1)
                };

                RealmListUpdate {
                    update: RealmEntry {
                        wow_realm_address: realm_address_like_cpp(r.region, r.battlegroup, r.id)
                            as i32,
                        cfg_timezones_id: 1,
                        population_state,
                        cfg_categories_id: i32::from(r.timezone),
                        version: ClientVersion {
                            version_major: build_info
                                .map_or(DEFAULT_VERSION_MAJOR as i32, |b| b.major_version as i32),
                            version_build: r.build as i32,
                            version_minor: build_info
                                .map_or(DEFAULT_VERSION_MINOR as i32, |b| b.minor_version as i32),
                            version_revision: build_info
                                .map_or(DEFAULT_VERSION_REVISION as i32, |b| {
                                    b.bugfix_version as i32
                                }),
                        },
                        cfg_realms_id: r.id as i32,
                        flags: i32::from(flags),
                        name: r.name.clone(),
                        cfg_configs_id: i32::from(realm_config_id_like_cpp(r.icon)),
                        cfg_languages_id: 1,
                    },
                    deleting: false,
                }
            })
            .collect();

        let realm_list = RealmListUpdates { updates };
        let realm_json = format!(
            "JSONRealmListUpdates:{}\0",
            serde_json::to_string(&realm_list).unwrap_or_default()
        );
        let compressed_realms = zlib_compress(realm_json.as_bytes());

        let counts: Vec<RealmCharacterCountEntry> = char_counts
            .iter()
            .map(|(&realm_id, &count)| RealmCharacterCountEntry {
                wow_realm_address: realm_id as i32,
                count: i32::from(count),
            })
            .collect();
        let count_list = RealmCharacterCountList { counts };
        let count_json = format!(
            "JSONRealmCharacterCountList:{}\0",
            serde_json::to_string(&count_list).unwrap_or_default()
        );
        let compressed_counts = zlib_compress(count_json.as_bytes());

        (compressed_realms, compressed_counts)
    }

    /// Generate compressed JSON for server IP addresses of a realm.
    /// Selects local or external address based on the client's IP:
    /// - loopback (127.x) → local address
    /// - same /24 subnet as local address → local address
    /// - otherwise → external address
    pub fn get_realm_entry_json(
        &self,
        realm: &Realm,
        client_ip: Option<std::net::IpAddr>,
    ) -> Vec<u8> {
        let selected_ip =
            select_realm_ip_str(client_ip, &realm.external_address, &realm.local_address);
        let addresses = RealmListServerIpAddresses {
            families: vec![AddressFamily {
                family: 1,
                addresses: vec![IpAddress {
                    ip: selected_ip,
                    port: i32::from(realm.port),
                }],
            }],
        };
        let json = format!(
            "JSONRealmListServerIPAddresses:{}\0",
            serde_json::to_string(&addresses).unwrap_or_default()
        );
        zlib_compress(json.as_bytes())
    }
}

/// Pick the right realm IP for a given client address.
/// - loopback → local
/// - same /24 subnet as local → local
/// - otherwise → external
fn select_realm_ip_str(client_ip: Option<std::net::IpAddr>, external: &str, local: &str) -> String {
    let client = match client_ip {
        Some(std::net::IpAddr::V4(v4)) => v4.octets(),
        _ => return external.to_string(),
    };

    // loopback
    if client[0] == 127 {
        tracing::debug!("select_realm_ip: client is loopback → local ({})", local);
        return local.to_string();
    }

    // same /24 as local address?
    if let Ok(std::net::IpAddr::V4(local_v4)) = local.parse() {
        let loc = local_v4.octets();
        if client[0] == loc[0] && client[1] == loc[1] && client[2] == loc[2] {
            tracing::debug!(
                "select_realm_ip: client {}.{}.{}.{} on same /24 as local {} → local",
                client[0],
                client[1],
                client[2],
                client[3],
                local
            );
            return local.to_string();
        }
    }

    tracing::debug!(
        "select_realm_ip: client is external → external ({})",
        external
    );
    external.to_string()
}

/// Initialize the realm manager and start periodic updates.
pub async fn init_realm_manager(state: Arc<AppState>, update_interval_secs: u64) -> Result<()> {
    // Load build info
    load_build_info(&state).await?;
    // Initial realm load
    update_realms(&state).await?;

    // Start periodic update timer
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(update_interval_secs));
        loop {
            interval.tick().await;
            if let Err(e) = update_realms(&state_clone).await {
                tracing::warn!("Failed to update realm list: {e}");
            }
        }
    });

    Ok(())
}

async fn load_build_info(state: &AppState) -> Result<()> {
    let mut result = state.login_db
        .direct_query("SELECT majorVersion, minorVersion, bugfixVersion, hotfixVersion, build, win64AuthSeed, mac64AuthSeed FROM build_info ORDER BY build ASC")
        .await?;

    let mut builds = Vec::new();
    if !result.is_empty() {
        loop {
            let major: u32 = result.try_read::<i32>(0).unwrap_or(0) as u32;
            let minor: u32 = result.try_read::<i32>(1).unwrap_or(0) as u32;
            let bugfix: u32 = result.try_read::<i32>(2).unwrap_or(0) as u32;
            let hotfix: String = result.try_read::<String>(3).unwrap_or_default();
            let build: u32 = result.try_read::<i32>(4).unwrap_or(0) as u32;
            let win_seed: Option<String> = result.try_read(5);
            let mac_seed: Option<String> = result.try_read(6);

            builds.push(RealmBuildInfo {
                major_version: major,
                minor_version: minor,
                bugfix_version: bugfix,
                hotfix_version: hotfix,
                build,
                win64_auth_seed: win_seed.and_then(|s| parse_hex_seed(&s)),
                mac64_auth_seed: mac_seed.and_then(|s| parse_hex_seed(&s)),
            });

            if !result.next_row() {
                break;
            }
        }
    }

    tracing::info!("Loaded {} build info entries", builds.len());
    state.realm_mgr.write().builds = builds;
    Ok(())
}

async fn update_realms(state: &AppState) -> Result<()> {
    let stmt = state.login_db.prepare(LoginStatements::SEL_REALMLIST);
    let mut result = state.login_db.query(&stmt).await?;

    let mut realms = HashMap::new();
    let mut sub_regions = Vec::new();

    if !result.is_empty() {
        loop {
            // All numeric columns in `realmlist` are UNSIGNED in MySQL.
            // sqlx requires exact type matching: unsigned → u32/u16/u8.
            let id: u32 = result.try_read::<u32>(0).unwrap_or(0);
            let name: String = result.read(1);
            let normalized_name = normalized_realm_name_like_cpp(&name);
            let address: String = result.read(2);
            let local_address: String = result.read(3);
            let port: u16 = result.try_read::<u16>(4).unwrap_or(8085);
            let icon: u8 = normalize_realm_type_like_cpp(result.try_read::<u8>(5).unwrap_or(0));
            let flag: u8 = result.try_read::<u8>(6).unwrap_or(0);
            let timezone: u8 = result.try_read::<u8>(7).unwrap_or(0);
            let allowed_security_level: u8 =
                result.try_read::<u8>(8).unwrap_or(0).min(SEC_ADMINISTRATOR);
            let population: f32 = result.try_read::<f32>(9).unwrap_or(0.0);
            let build: u32 = result.try_read::<u32>(10).unwrap_or(0);
            let region: u8 = result.try_read::<u8>(11).unwrap_or(0);
            let battlegroup: u8 = result.try_read::<u8>(12).unwrap_or(0);

            let sub_region = realm_sub_region_address_like_cpp(region, battlegroup);
            if !sub_regions.contains(&sub_region) {
                sub_regions.push(sub_region);
            }

            realms.insert(
                id,
                Realm {
                    id,
                    name,
                    normalized_name,
                    external_address: address,
                    local_address,
                    port,
                    icon,
                    flag,
                    timezone,
                    allowed_security_level,
                    population,
                    build,
                    region,
                    battlegroup,
                },
            );

            if !result.next_row() {
                break;
            }
        }
    }

    let count = realms.len();
    let mut mgr = state.realm_mgr.write();
    mgr.realms = realms;
    mgr.sub_regions = sub_regions;
    tracing::debug!("Updated {count} realms");
    Ok(())
}

fn normalize_realm_type_like_cpp(icon: u8) -> u8 {
    if icon == REALM_TYPE_FFA_PVP {
        return REALM_TYPE_PVP;
    }
    if icon >= MAX_CLIENT_REALM_TYPE {
        return REALM_TYPE_NORMAL;
    }
    icon
}

fn normalized_realm_name_like_cpp(name: &str) -> String {
    name.chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect()
}

fn realm_config_id_like_cpp(realm_type: u8) -> u8 {
    normalize_realm_type_like_cpp(realm_type) + 1
}

pub(crate) fn realm_address_like_cpp(region: u8, battlegroup: u8, realm_id: u32) -> u32 {
    (u32::from(region) << 24) | (u32::from(battlegroup) << 16) | (realm_id & 0xFFFF)
}

fn realm_id_from_address_like_cpp(realm_address: u32) -> u32 {
    realm_address & 0xFFFF
}

pub(crate) fn realm_sub_region_address_like_cpp(region: u8, battlegroup: u8) -> String {
    format!("{region}-{battlegroup}-0")
}

fn parse_hex_seed(hex: &str) -> Option<Vec<u8>> {
    if hex.is_empty() || hex.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hex[i..i + 2], 16).ok()?);
    }
    Some(bytes)
}

fn zlib_compress(data: &[u8]) -> Vec<u8> {
    // Prepend 4-byte little-endian uncompressed size
    let uncompressed_len = data.len() as u32;
    let mut result = uncompressed_len.to_le_bytes().to_vec();

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).expect("zlib write failed");
    let compressed = encoder.finish().expect("zlib finish failed");
    result.extend_from_slice(&compressed);
    result
}

// ── JSON types for realm list (matching C# RealmList JSON structures) ───────

#[derive(Serialize)]
struct RealmListUpdates {
    updates: Vec<RealmListUpdate>,
}

#[derive(Serialize)]
struct RealmListUpdate {
    update: RealmEntry,
    deleting: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RealmEntry {
    wow_realm_address: i32,
    cfg_timezones_id: i32,
    population_state: i32,
    cfg_categories_id: i32,
    version: ClientVersion,
    cfg_realms_id: i32,
    flags: i32,
    name: String,
    cfg_configs_id: i32,
    cfg_languages_id: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientVersion {
    version_major: i32,
    version_build: i32,
    version_minor: i32,
    version_revision: i32,
}

#[derive(Serialize)]
struct RealmCharacterCountList {
    counts: Vec<RealmCharacterCountEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RealmCharacterCountEntry {
    wow_realm_address: i32,
    count: i32,
}

#[derive(Serialize)]
struct RealmListServerIpAddresses {
    families: Vec<AddressFamily>,
}

#[derive(Serialize)]
struct AddressFamily {
    family: i32,
    addresses: Vec<IpAddress>,
}

#[derive(Serialize)]
struct IpAddress {
    ip: String,
    port: i32,
}

#[cfg(test)]
mod tests {
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
            icon,
            flag: 0,
            timezone,
            allowed_security_level: 0,
            population: 2.0,
            build: 51943,
            region,
            battlegroup,
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

    #[test]
    fn realm_handle_address_matches_cpp_packing_and_lookup() {
        let realm_address = realm_address_like_cpp(5, 6, 9);
        assert_eq!(realm_address, 0x0506_0009);
        assert_eq!(realm_id_from_address_like_cpp(realm_address), 9);
        assert_eq!(realm_sub_region_address_like_cpp(5, 6), "5-6-0");

        let mut manager = RealmManager::new();
        manager.realms.insert(9, test_realm(9, 5, 6, 1, 1));
        assert_eq!(
            manager
                .get_realm_by_realm_address_like_cpp(realm_address)
                .map(|realm| realm.id),
            Some(9)
        );
    }

    #[test]
    fn realm_names_strip_ascii_whitespace_like_cpp() {
        assert_eq!(
            normalized_realm_name_like_cpp("Ice Crown\t Citadel\n"),
            "IceCrownCitadel"
        );

        let mut manager = RealmManager::new();
        let mut realm = test_realm(9, 5, 6, 1, 1);
        realm.name = "Ice Crown".to_string();
        realm.normalized_name = normalized_realm_name_like_cpp(&realm.name);
        manager.realms.insert(9, realm);

        assert_eq!(
            manager.get_realm_names_like_cpp(realm_address_like_cpp(5, 6, 9)),
            Some(("Ice Crown".to_string(), "IceCrown".to_string()))
        );
        assert_eq!(
            manager.get_realm_names_like_cpp(realm_address_like_cpp(5, 6, 10)),
            None
        );
    }

    #[test]
    fn minor_major_bugfix_version_uses_cpp_lower_bound_semantics() {
        let mut manager = RealmManager::new();
        manager.builds = vec![
            RealmBuildInfo {
                major_version: 3,
                minor_version: 4,
                bugfix_version: 2,
                hotfix_version: String::new(),
                build: 51800,
                win64_auth_seed: None,
                mac64_auth_seed: None,
            },
            RealmBuildInfo {
                major_version: 3,
                minor_version: 4,
                bugfix_version: 3,
                hotfix_version: String::new(),
                build: 51943,
                win64_auth_seed: None,
                mac64_auth_seed: None,
            },
        ];

        assert_eq!(
            manager.get_minor_major_bugfix_version_for_build_like_cpp(51800),
            30_402
        );
        assert_eq!(
            manager.get_minor_major_bugfix_version_for_build_like_cpp(51801),
            30_403
        );
        assert_eq!(
            manager.get_minor_major_bugfix_version_for_build_like_cpp(99999),
            0
        );
    }

    #[test]
    fn realm_list_json_filters_subregion_and_uses_cpp_fields() {
        let mut manager = RealmManager::new();
        manager.realms.insert(9, test_realm(9, 5, 6, 3, 1));
        manager.realms.insert(10, test_realm(10, 7, 8, 4, 6));
        manager.builds.push(RealmBuildInfo {
            major_version: 3,
            minor_version: 4,
            bugfix_version: 3,
            hotfix_version: String::new(),
            build: 51943,
            win64_auth_seed: None,
            mac64_auth_seed: None,
        });

        let mut counts = HashMap::new();
        counts.insert(realm_address_like_cpp(5, 6, 9), 2);

        let (realms, char_counts) = manager.get_realm_list_json(51943, "5-6-0", &counts);
        let realms = inflate_payload(&realms);
        let json = parse_enveloped_json(&realms, "JSONRealmListUpdates:");
        let updates = json["updates"].as_array().unwrap();
        assert_eq!(updates.len(), 1);

        let update = &updates[0]["update"];
        assert_eq!(update["wowRealmAddress"], 0x0506_0009);
        assert_eq!(update["cfgTimezonesId"], 1);
        assert_eq!(update["cfgCategoriesId"], 3);
        assert_eq!(update["cfgConfigsId"], 2);
        assert_eq!(update["cfgRealmsId"], 9);
        assert_eq!(update["version"]["versionMajor"], 3);
        assert_eq!(update["version"]["versionMinor"], 4);
        assert_eq!(update["version"]["versionRevision"], 3);
        assert_eq!(update["version"]["versionBuild"], 51943);

        let char_counts = inflate_payload(&char_counts);
        let json = parse_enveloped_json(&char_counts, "JSONRealmCharacterCountList:");
        assert_eq!(json["counts"][0]["wowRealmAddress"], 0x0506_0009);
        assert_eq!(json["counts"][0]["count"], 2);
    }

    #[test]
    fn realm_list_json_uses_cpp_fallback_version_and_type_normalization() {
        let mut manager = RealmManager::new();
        manager
            .realms
            .insert(9, test_realm(9, 5, 6, 3, REALM_TYPE_FFA_PVP));

        let (realms, _) = manager.get_realm_list_json(12340, "5-6-0", &HashMap::new());
        let realms = inflate_payload(&realms);
        let json = parse_enveloped_json(&realms, "JSONRealmListUpdates:");
        let update = &json["updates"][0]["update"];

        assert_eq!(update["flags"], REALM_FLAG_VERSION_MISMATCH);
        assert_eq!(update["cfgConfigsId"], 2);
        assert_eq!(update["version"]["versionMajor"], DEFAULT_VERSION_MAJOR);
        assert_eq!(update["version"]["versionMinor"], DEFAULT_VERSION_MINOR);
        assert_eq!(
            update["version"]["versionRevision"],
            DEFAULT_VERSION_REVISION
        );
    }
}
