//! Realm list management.
//!
//! Periodically polls the `realmlist` table and provides realm data to clients.

use anyhow::Result;
use bitflags::bitflags;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use serde::Serialize;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::Arc;

use crate::state::AppState;
use wow_database::LoginStatements;

const SEC_ADMINISTRATOR: u8 = 3;
const DEFAULT_VERSION_MAJOR: u32 = 6;
const DEFAULT_VERSION_MINOR: u32 = 2;
const DEFAULT_VERSION_REVISION: u32 = 4;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RealmFlagsLikeCpp: u8 {
        const NONE = 0x00;
        const VERSION_MISMATCH = 0x01;
        const OFFLINE = 0x02;
        const SPECIFYBUILD = 0x04;
        const UNK1 = 0x08;
        const UNK2 = 0x10;
        const RECOMMENDED = 0x20;
        const NEW = 0x40;
        const FULL = 0x80;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealmTypeLikeCpp(u8);

impl RealmTypeLikeCpp {
    #[allow(dead_code)]
    pub const NORMAL: Self = Self(0);
    #[allow(dead_code)]
    pub const PVP: Self = Self(1);
    #[allow(dead_code)]
    pub const NORMAL2: Self = Self(4);
    #[allow(dead_code)]
    pub const RP: Self = Self(6);
    #[allow(dead_code)]
    pub const RPPVP: Self = Self(8);
    #[allow(dead_code)]
    pub const MAX_CLIENT_REALM_TYPE: u8 = 14;
    #[allow(dead_code)]
    pub const FFA_PVP: Self = Self(16);

    pub fn from_db_like_cpp(icon: u8) -> Self {
        if icon == Self::FFA_PVP.0 {
            return Self::PVP;
        }
        if icon >= Self::MAX_CLIENT_REALM_TYPE {
            return Self::NORMAL;
        }
        Self(icon)
    }

    #[allow(dead_code)]
    pub fn as_u8(self) -> u8 {
        self.0
    }

    pub fn get_config_id_like_cpp(self) -> u8 {
        self.0 + 1
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RealmHandleLikeCpp {
    pub region: u8,
    pub site: u8,
    pub realm: u32,
}

impl RealmHandleLikeCpp {
    pub fn new_like_cpp(region: u8, battlegroup: u8, realm: u32) -> Self {
        Self {
            region,
            site: battlegroup,
            realm,
        }
    }

    pub fn from_address_like_cpp(realm_address: u32) -> Self {
        Self {
            region: ((realm_address >> 24) & 0xFF) as u8,
            site: ((realm_address >> 16) & 0xFF) as u8,
            realm: realm_address & 0xFFFF,
        }
    }

    pub fn get_address_like_cpp(self) -> u32 {
        (u32::from(self.region) << 24) | (u32::from(self.site) << 16) | (self.realm & 0xFFFF)
    }

    #[allow(dead_code)]
    pub fn get_address_string_like_cpp(self) -> String {
        format!("{}-{}-{}", self.region, self.site, self.realm)
    }

    pub fn get_sub_region_address_like_cpp(self) -> String {
        format!("{}-{}-0", self.region, self.site)
    }
}

impl PartialEq for RealmHandleLikeCpp {
    fn eq(&self, other: &Self) -> bool {
        self.realm == other.realm
    }
}

impl Eq for RealmHandleLikeCpp {}

impl Hash for RealmHandleLikeCpp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.realm.hash(state);
    }
}

impl PartialOrd for RealmHandleLikeCpp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RealmHandleLikeCpp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.realm.cmp(&other.realm)
    }
}

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
    pub icon: RealmTypeLikeCpp,
    pub flag: RealmFlagsLikeCpp,
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
    pub(crate) realms: HashMap<RealmHandleLikeCpp, Realm>,
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
            .get(&RealmHandleLikeCpp::from_address_like_cpp(realm_address))
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
            .filter(|r| {
                RealmHandleLikeCpp::new_like_cpp(r.region, r.battlegroup, r.id)
                    .get_sub_region_address_like_cpp()
                    == _sub_region
            })
            .map(|r| {
                let build_info = self.get_build_info(r.build);

                // Dynamically add VersionMismatch if client build != realm build
                let mut flags = r.flag;
                if r.build != build {
                    flags.insert(RealmFlagsLikeCpp::VERSION_MISMATCH);
                }

                // Population: 0 if offline, else max(population_level, 1)
                let is_offline = flags.contains(RealmFlagsLikeCpp::OFFLINE);
                let population_state = if is_offline {
                    0
                } else {
                    (r.population as i32).max(1)
                };

                RealmListUpdate {
                    update: RealmEntry {
                        wow_realm_address: RealmHandleLikeCpp::new_like_cpp(
                            r.region,
                            r.battlegroup,
                            r.id,
                        )
                        .get_address_like_cpp() as i32,
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
                        flags: i32::from(flags.bits()),
                        name: r.name.clone(),
                        cfg_configs_id: i32::from(r.icon.get_config_id_like_cpp()),
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

    /// Generate `JamJSONRealmEntry` like C++ RealmList::GetRealmEntryJSON.
    pub fn get_realm_entry_json_like_cpp(&self, realm_address: u32, build: u32) -> Vec<u8> {
        let Some(realm) = self.get_realm_by_realm_address_like_cpp(realm_address) else {
            return Vec::new();
        };

        if realm.flag.contains(RealmFlagsLikeCpp::OFFLINE) || realm.build != build {
            return Vec::new();
        }

        let build_info = self.get_build_info(realm.build);
        let realm_entry = RealmEntry {
            wow_realm_address: RealmHandleLikeCpp::new_like_cpp(
                realm.region,
                realm.battlegroup,
                realm.id,
            )
            .get_address_like_cpp() as i32,
            cfg_timezones_id: 1,
            population_state: (realm.population as i32).max(1),
            cfg_categories_id: i32::from(realm.timezone),
            version: ClientVersion {
                version_major: build_info
                    .map_or(DEFAULT_VERSION_MAJOR as i32, |b| b.major_version as i32),
                version_build: realm.build as i32,
                version_minor: build_info
                    .map_or(DEFAULT_VERSION_MINOR as i32, |b| b.minor_version as i32),
                version_revision: build_info
                    .map_or(DEFAULT_VERSION_REVISION as i32, |b| b.bugfix_version as i32),
            },
            cfg_realms_id: realm.id as i32,
            flags: i32::from(realm.flag.bits()),
            name: realm.name.clone(),
            cfg_configs_id: i32::from(realm.icon.get_config_id_like_cpp()),
            cfg_languages_id: 1,
        };
        let json = format!(
            "JamJSONRealmEntry:{}\0",
            serde_json::to_string(&realm_entry).unwrap_or_default()
        );
        zlib_compress(json.as_bytes())
    }

    /// Generate compressed JSON for server IP addresses of a realm.
    /// Selects local or external address based on the client's IP:
    /// - loopback (127.x) → local address
    /// - same /24 subnet as local address → local address
    /// - otherwise → external address
    pub fn get_realm_server_addresses_json_like_cpp(
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
            let icon = RealmTypeLikeCpp::from_db_like_cpp(result.try_read::<u8>(5).unwrap_or(0));
            let flag = RealmFlagsLikeCpp::from_bits_retain(result.try_read::<u8>(6).unwrap_or(0));
            let timezone: u8 = result.try_read::<u8>(7).unwrap_or(0);
            let allowed_security_level: u8 =
                result.try_read::<u8>(8).unwrap_or(0).min(SEC_ADMINISTRATOR);
            let population: f32 = result.try_read::<f32>(9).unwrap_or(0.0);
            let build: u32 = result.try_read::<u32>(10).unwrap_or(0);
            let region: u8 = result.try_read::<u8>(11).unwrap_or(0);
            let battlegroup: u8 = result.try_read::<u8>(12).unwrap_or(0);

            let sub_region = RealmHandleLikeCpp::new_like_cpp(region, battlegroup, 0)
                .get_sub_region_address_like_cpp();
            if !sub_regions.contains(&sub_region) {
                sub_regions.push(sub_region);
            }

            let handle = RealmHandleLikeCpp::new_like_cpp(region, battlegroup, id);
            realms.insert(
                handle,
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

fn normalized_realm_name_like_cpp(name: &str) -> String {
    name.chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect()
}

pub(crate) fn realm_address_like_cpp(region: u8, battlegroup: u8, realm_id: u32) -> u32 {
    RealmHandleLikeCpp::new_like_cpp(region, battlegroup, realm_id).get_address_like_cpp()
}

pub(crate) fn realm_sub_region_address_like_cpp(region: u8, battlegroup: u8) -> String {
    RealmHandleLikeCpp::new_like_cpp(region, battlegroup, 0).get_sub_region_address_like_cpp()
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
        let handle = RealmHandleLikeCpp::new_like_cpp(5, 6, 9);
        let realm_address = handle.get_address_like_cpp();
        assert_eq!(realm_address, 0x0506_0009);
        assert_eq!(
            RealmHandleLikeCpp::from_address_like_cpp(realm_address),
            handle
        );
        assert_eq!(handle.get_address_string_like_cpp(), "5-6-9");
        assert_eq!(handle.get_sub_region_address_like_cpp(), "5-6-0");

        let same_realm_different_region = RealmHandleLikeCpp::new_like_cpp(1, 2, 9);
        let other_realm = RealmHandleLikeCpp::new_like_cpp(5, 6, 10);
        assert_eq!(handle, same_realm_different_region);
        assert_eq!(
            handle.cmp(&same_realm_different_region),
            std::cmp::Ordering::Equal
        );
        assert!(handle < other_realm);

        let mut manager = RealmManager::new();
        manager.realms.insert(
            RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
            test_realm(9, 5, 6, 1, 1),
        );
        assert_eq!(
            manager
                .get_realm_by_realm_address_like_cpp(realm_address)
                .map(|realm| realm.id),
            Some(9)
        );
    }

    #[test]
    fn realm_manager_storage_key_matches_cpp_realm_only_ordering() {
        let mut manager = RealmManager::new();
        manager.realms.insert(
            RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
            test_realm(9, 5, 6, 1, 1),
        );
        let mut replacement = test_realm(9, 1, 2, 3, 1);
        replacement.name = "Replacement".to_string();
        manager
            .realms
            .insert(RealmHandleLikeCpp::new_like_cpp(1, 2, 9), replacement);

        assert_eq!(manager.realms.len(), 1);
        assert_eq!(
            manager
                .get_realm_by_realm_address_like_cpp(realm_address_like_cpp(5, 6, 9))
                .map(|realm| realm.name.as_str()),
            Some("Replacement")
        );
        assert_eq!(
            manager
                .get_realm_by_realm_address_like_cpp(realm_address_like_cpp(1, 2, 9))
                .map(|realm| realm.name.as_str()),
            Some("Replacement")
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
        manager
            .realms
            .insert(RealmHandleLikeCpp::new_like_cpp(5, 6, 9), realm);

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
    fn realm_flags_match_cpp_bits() {
        assert_eq!(RealmFlagsLikeCpp::NONE.bits(), 0x00);
        assert_eq!(RealmFlagsLikeCpp::VERSION_MISMATCH.bits(), 0x01);
        assert_eq!(RealmFlagsLikeCpp::OFFLINE.bits(), 0x02);
        assert_eq!(RealmFlagsLikeCpp::SPECIFYBUILD.bits(), 0x04);
        assert_eq!(RealmFlagsLikeCpp::UNK1.bits(), 0x08);
        assert_eq!(RealmFlagsLikeCpp::UNK2.bits(), 0x10);
        assert_eq!(RealmFlagsLikeCpp::RECOMMENDED.bits(), 0x20);
        assert_eq!(RealmFlagsLikeCpp::NEW.bits(), 0x40);
        assert_eq!(RealmFlagsLikeCpp::FULL.bits(), 0x80);
    }

    #[test]
    fn realm_type_normalization_matches_cpp() {
        assert_eq!(RealmTypeLikeCpp::NORMAL.as_u8(), 0);
        assert_eq!(RealmTypeLikeCpp::PVP.as_u8(), 1);
        assert_eq!(RealmTypeLikeCpp::NORMAL2.as_u8(), 4);
        assert_eq!(RealmTypeLikeCpp::RP.as_u8(), 6);
        assert_eq!(RealmTypeLikeCpp::RPPVP.as_u8(), 8);
        assert_eq!(RealmTypeLikeCpp::MAX_CLIENT_REALM_TYPE, 14);
        assert_eq!(RealmTypeLikeCpp::FFA_PVP.as_u8(), 16);

        assert_eq!(
            RealmTypeLikeCpp::from_db_like_cpp(RealmTypeLikeCpp::FFA_PVP.as_u8()).as_u8(),
            RealmTypeLikeCpp::PVP.as_u8()
        );
        assert_eq!(
            RealmTypeLikeCpp::from_db_like_cpp(RealmTypeLikeCpp::MAX_CLIENT_REALM_TYPE).as_u8(),
            RealmTypeLikeCpp::NORMAL.as_u8()
        );
        assert_eq!(RealmTypeLikeCpp::from_db_like_cpp(13).as_u8(), 13);
        assert_eq!(
            RealmTypeLikeCpp::from_db_like_cpp(13).get_config_id_like_cpp(),
            14
        );
    }

    #[test]
    fn realm_list_json_filters_subregion_and_uses_cpp_fields() {
        let mut manager = RealmManager::new();
        manager.realms.insert(
            RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
            test_realm(9, 5, 6, 3, 1),
        );
        manager.realms.insert(
            RealmHandleLikeCpp::new_like_cpp(7, 8, 10),
            test_realm(10, 7, 8, 4, 6),
        );
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
        manager.realms.insert(
            RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
            test_realm(9, 5, 6, 3, RealmTypeLikeCpp::FFA_PVP.as_u8()),
        );

        let (realms, _) = manager.get_realm_list_json(12340, "5-6-0", &HashMap::new());
        let realms = inflate_payload(&realms);
        let json = parse_enveloped_json(&realms, "JSONRealmListUpdates:");
        let update = &json["updates"][0]["update"];

        assert_eq!(update["flags"], RealmFlagsLikeCpp::VERSION_MISMATCH.bits());
        assert_eq!(update["cfgConfigsId"], 2);
        assert_eq!(update["version"]["versionMajor"], DEFAULT_VERSION_MAJOR);
        assert_eq!(update["version"]["versionMinor"], DEFAULT_VERSION_MINOR);
        assert_eq!(
            update["version"]["versionRevision"],
            DEFAULT_VERSION_REVISION
        );
    }

    #[test]
    fn realm_list_json_offline_realm_has_zero_population_like_cpp() {
        let mut manager = RealmManager::new();
        let mut realm = test_realm(9, 5, 6, 3, 1);
        realm.flag = RealmFlagsLikeCpp::OFFLINE;
        manager
            .realms
            .insert(RealmHandleLikeCpp::new_like_cpp(5, 6, 9), realm);

        let (realms, _) = manager.get_realm_list_json(51943, "5-6-0", &HashMap::new());
        let realms = inflate_payload(&realms);
        let json = parse_enveloped_json(&realms, "JSONRealmListUpdates:");
        let update = &json["updates"][0]["update"];

        assert_eq!(update["populationState"], 0);
        assert_eq!(update["flags"], RealmFlagsLikeCpp::OFFLINE.bits());
    }

    #[test]
    fn realm_entry_json_matches_cpp_envelope_and_empty_gates() {
        let mut manager = RealmManager::new();
        manager.realms.insert(
            RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
            test_realm(9, 5, 6, 3, 1),
        );
        manager.builds.push(RealmBuildInfo {
            major_version: 3,
            minor_version: 4,
            bugfix_version: 3,
            hotfix_version: String::new(),
            build: 51943,
            win64_auth_seed: None,
            mac64_auth_seed: None,
        });

        let packed = realm_address_like_cpp(5, 6, 9);
        let entry = manager.get_realm_entry_json_like_cpp(packed, 51943);
        let entry = inflate_payload(&entry);
        let json = parse_enveloped_json(&entry, "JamJSONRealmEntry:");
        assert_eq!(json["wowRealmAddress"], 0x0506_0009);
        assert_eq!(json["cfgTimezonesId"], 1);
        assert_eq!(json["cfgCategoriesId"], 3);
        assert_eq!(json["populationState"], 2);
        assert_eq!(json["version"]["versionBuild"], 51943);

        assert!(
            manager
                .get_realm_entry_json_like_cpp(packed, 12340)
                .is_empty()
        );

        manager
            .realms
            .get_mut(&RealmHandleLikeCpp::new_like_cpp(5, 6, 9))
            .unwrap()
            .flag = RealmFlagsLikeCpp::OFFLINE;
        assert!(
            manager
                .get_realm_entry_json_like_cpp(packed, 51943)
                .is_empty()
        );
    }

    #[test]
    fn server_addresses_json_selects_local_or_external_like_cpp() {
        let manager = RealmManager::new();
        let realm = test_realm(9, 5, 6, 3, 1);

        assert_eq!(
            select_realm_ip_str(
                Some(std::net::IpAddr::V4("127.0.0.1".parse().unwrap())),
                &realm.external_address,
                &realm.local_address,
            ),
            realm.local_address
        );
        assert_eq!(
            select_realm_ip_str(
                Some(std::net::IpAddr::V4("10.0.0.42".parse().unwrap())),
                &realm.external_address,
                &realm.local_address,
            ),
            realm.local_address
        );
        assert_eq!(
            select_realm_ip_str(
                Some(std::net::IpAddr::V4("198.51.100.42".parse().unwrap())),
                &realm.external_address,
                &realm.local_address,
            ),
            realm.external_address
        );

        let addresses = manager
            .get_realm_server_addresses_json_like_cpp(&realm, Some("127.0.0.1".parse().unwrap()));
        let addresses = inflate_payload(&addresses);
        let json = parse_enveloped_json(&addresses, "JSONRealmListServerIPAddresses:");
        assert_eq!(json["families"][0]["family"], 1);
        assert_eq!(json["families"][0]["addresses"][0]["ip"], "10.0.0.10");
        assert_eq!(json["families"][0]["addresses"][0]["port"], 8085);
    }
}
