//! Realm list management.
//!
//! Periodically polls the `realmlist` table and provides realm data to clients.

use anyhow::Result;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use crate::state::AppState;
use wow_database::LoginStatements;

/// A single realm entry from the `realmlist` table.
#[derive(Debug, Clone)]
pub struct Realm {
    pub id: u32,
    pub name: String,
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

    /// Get build info for a specific build number.
    pub fn get_build_info(&self, build: u32) -> Option<&RealmBuildInfo> {
        self.builds.iter().find(|b| b.build == build)
    }

    /// Generate compressed JSON realm list for a specific build and sub-region.
    ///
    /// Matches C# RealmManager.GetRealmList() logic:
    /// - All realms are included (not filtered by build)
    /// - VersionMismatch flag (0x01) added dynamically if build doesn't match
    /// - PopulationState = 0 if offline, else max(population_level, 1)
    pub fn get_realm_list_json(&self, build: u32, _sub_region: &str, char_counts: &HashMap<u32, u8>) -> (Vec<u8>, Vec<u8>) {
        const REALM_FLAG_VERSION_MISMATCH: u8 = 0x01;
        const REALM_FLAG_OFFLINE: u8 = 0x02;

        let updates: Vec<RealmListUpdate> = self.realms.values()
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
                        wow_realm_address: r.id as i32,
                        cfg_timezones_id: i32::from(r.timezone),
                        population_state,
                        cfg_categories_id: 1,
                        version: ClientVersion {
                            version_major: build_info.map_or(0, |b| b.major_version as i32),
                            version_build: r.build as i32,
                            version_minor: build_info.map_or(0, |b| b.minor_version as i32),
                            version_revision: build_info.map_or(0, |b| b.bugfix_version as i32),
                        },
                        cfg_realms_id: r.id as i32,
                        flags: i32::from(flags),
                        name: r.name.clone(),
                        cfg_configs_id: 1,
                        cfg_languages_id: 1,
                    },
                    deleting: false,
                }
            })
            .collect();

        let realm_list = RealmListUpdates { updates };
        let realm_json = format!("JSONRealmListUpdates:{}\0", serde_json::to_string(&realm_list).unwrap_or_default());
        let compressed_realms = zlib_compress(realm_json.as_bytes());

        let counts: Vec<RealmCharacterCountEntry> = char_counts.iter()
            .map(|(&realm_id, &count)| RealmCharacterCountEntry {
                wow_realm_address: realm_id as i32,
                count: i32::from(count),
            })
            .collect();
        let count_list = RealmCharacterCountList { counts };
        let count_json = format!("JSONRealmCharacterCountList:{}\0", serde_json::to_string(&count_list).unwrap_or_default());
        let compressed_counts = zlib_compress(count_json.as_bytes());

        (compressed_realms, compressed_counts)
    }

    /// Generate compressed JSON for server IP addresses of a realm.
    /// Selects local or external address based on the client's IP:
    /// - loopback (127.x) → local address
    /// - same /24 subnet as local address → local address
    /// - otherwise → external address
    pub fn get_realm_entry_json(&self, realm: &Realm, client_ip: Option<std::net::IpAddr>) -> Vec<u8> {
        let selected_ip = select_realm_ip_str(client_ip, &realm.external_address, &realm.local_address);
        let addresses = RealmListServerIpAddresses {
            families: vec![AddressFamily {
                family: 1,
                addresses: vec![
                    IpAddress { ip: selected_ip, port: i32::from(realm.port) },
                ],
            }],
        };
        let json = format!("JSONRealmListServerIPAddresses:{}\0", serde_json::to_string(&addresses).unwrap_or_default());
        zlib_compress(json.as_bytes())
    }
}

/// Pick the right realm IP for a given client address.
/// - loopback → local
/// - same /24 subnet as local → local
/// - otherwise → external
fn select_realm_ip_str(
    client_ip: Option<std::net::IpAddr>,
    external: &str,
    local: &str,
) -> String {
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
            tracing::debug!("select_realm_ip: client {}.{}.{}.{} on same /24 as local {} → local",
                client[0], client[1], client[2], client[3], local);
            return local.to_string();
        }
    }

    tracing::debug!("select_realm_ip: client is external → external ({})", external);
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
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(update_interval_secs));
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
            let address: String = result.read(2);
            let local_address: String = result.read(3);
            let port: u16 = result.try_read::<u16>(4).unwrap_or(8085);
            let icon: u8 = result.try_read::<u8>(5).unwrap_or(0);
            let flag: u8 = result.try_read::<u8>(6).unwrap_or(0);
            let timezone: u8 = result.try_read::<u8>(7).unwrap_or(0);
            let allowed_security_level: u8 = result.try_read::<u8>(8).unwrap_or(0);
            let population: f32 = result.try_read::<f32>(9).unwrap_or(0.0);
            let build: u32 = result.try_read::<u32>(10).unwrap_or(0);
            let region: u8 = result.try_read::<u8>(11).unwrap_or(0);
            let battlegroup: u8 = result.try_read::<u8>(12).unwrap_or(0);

            let sub_region = format!("{region}-{battlegroup}-0");
            if !sub_regions.contains(&sub_region) {
                sub_regions.push(sub_region);
            }

            realms.insert(id, Realm {
                id, name, external_address: address, local_address,
                port, icon, flag, timezone, allowed_security_level,
                population, build, region, battlegroup,
            });

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
