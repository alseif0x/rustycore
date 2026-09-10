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
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use crate::state::AppState;
use wow_database::LoginStatements;
use wow_proto::bgs::protocol::Variant;

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
    pub hotfix_version: [u8; 4],
    pub build: u32,
    pub win64_auth_seed: [u8; 16],
    pub mac64_auth_seed: [u8; 16],
}

pub struct JoinRealmPreparedLikeCpp {
    pub server_addresses: Vec<u8>,
    pub realm_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinRealmPrepareErrorLikeCpp {
    UnknownRealm,
    UserServerNotPermittedOnRealm,
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
    /// Selects local or external address based on the client's IP using the
    /// shared C++-like priority helper and scanned local IPv4 interfaces.
    pub fn get_realm_server_addresses_json_like_cpp(
        &self,
        realm: &Realm,
        client_ip: Option<std::net::IpAddr>,
    ) -> Vec<u8> {
        let scanned_networks = wow_core::scan_local_ipv4_networks_like_cpp();
        self.get_realm_server_addresses_json_with_local_networks_like_cpp(
            realm,
            client_ip,
            &scanned_networks,
        )
    }

    fn get_realm_server_addresses_json_with_local_networks_like_cpp(
        &self,
        realm: &Realm,
        client_ip: Option<std::net::IpAddr>,
        local_networks: &[wow_core::Ipv4NetworkLikeCpp],
    ) -> Vec<u8> {
        let selected_ip = select_realm_ip_str_with_local_networks(
            client_ip,
            &realm.external_address,
            &realm.local_address,
            local_networks,
        );
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

    /// Prepare the realm-owned part of C++ RealmList::JoinRealm.
    pub fn prepare_join_realm_like_cpp(
        &self,
        realm_address: u32,
        build: u32,
        client_ip: Option<std::net::IpAddr>,
    ) -> Result<JoinRealmPreparedLikeCpp, JoinRealmPrepareErrorLikeCpp> {
        let realm = self
            .get_realm_by_realm_address_like_cpp(realm_address)
            .ok_or(JoinRealmPrepareErrorLikeCpp::UnknownRealm)?;

        if realm.flag.contains(RealmFlagsLikeCpp::OFFLINE) || realm.build != build {
            return Err(JoinRealmPrepareErrorLikeCpp::UserServerNotPermittedOnRealm);
        }

        Ok(JoinRealmPreparedLikeCpp {
            server_addresses: self.get_realm_server_addresses_json_like_cpp(realm, client_ip),
            realm_name: realm.name.clone(),
        })
    }

    /// Write sub-region values like C++ RealmList::WriteSubRegions.
    pub fn write_sub_regions_like_cpp(&self) -> Vec<Variant> {
        self.sub_regions
            .iter()
            .map(|sub_region| Variant {
                string_value: Some(sub_region.clone()),
                ..Default::default()
            })
            .collect()
    }
}

fn select_realm_ip_str_with_local_networks(
    client_ip: Option<std::net::IpAddr>,
    external: &str,
    local: &str,
    scanned_networks: &[wow_core::Ipv4NetworkLikeCpp],
) -> String {
    let Ok(external_v4) = external.parse::<std::net::Ipv4Addr>() else {
        return external.to_string();
    };
    let Ok(local_v4) = local.parse::<std::net::Ipv4Addr>() else {
        return external.to_string();
    };
    let client_v4 = match client_ip {
        Some(std::net::IpAddr::V4(v4)) => Some(v4),
        _ => None,
    };
    let fallback_networks = [wow_core::Ipv4NetworkLikeCpp::new(local_v4, 24)];
    let local_networks = if scanned_networks.is_empty() {
        fallback_networks.as_slice()
    } else {
        scanned_networks
    };
    let selected = wow_core::realm_ipv4_address_for_client_like_cpp(
        client_v4,
        external_v4,
        local_v4,
        local_networks,
    );

    if selected == local_v4 {
        tracing::debug!("select_realm_ip: client selected local ({})", local);
        return local.to_string();
    }

    tracing::debug!("select_realm_ip: client selected external ({})", external);
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
            let win_seed: String = result.try_read::<String>(5).unwrap_or_default();
            let mac_seed: String = result.try_read::<String>(6).unwrap_or_default();

            builds.push(RealmBuildInfo {
                major_version: major,
                minor_version: minor,
                bugfix_version: bugfix,
                hotfix_version: parse_hotfix_version_like_cpp(&hotfix),
                build,
                win64_auth_seed: parse_auth_seed_like_cpp(&win_seed),
                mac64_auth_seed: parse_auth_seed_like_cpp(&mac_seed),
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
            let Some(external_address) =
                resolve_realm_address_like_cpp("address", &address, &name, id).await?
            else {
                if !result.next_row() {
                    break;
                }
                continue;
            };
            let Some(local_address) =
                resolve_realm_address_like_cpp("localAddress", &local_address, &name, id).await?
            else {
                if !result.next_row() {
                    break;
                }
                continue;
            };
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
                    external_address,
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

async fn resolve_realm_address_like_cpp(
    field_name: &str,
    hostname: &str,
    realm_name: &str,
    realm_id: u32,
) -> Result<Option<String>> {
    let endpoints = match tokio::net::lookup_host((hostname, 0)).await {
        Ok(endpoints) => endpoints,
        Err(error) => {
            tracing::error!(
                %error,
                "Could not resolve {field_name} {hostname} for realm \"{realm_name}\" id {realm_id}"
            );
            return Ok(None);
        }
    };

    let Some(address) = first_ipv4_address_like_cpp(endpoints) else {
        tracing::error!(
            "Could not resolve {field_name} {hostname} for realm \"{realm_name}\" id {realm_id} to an IPv4 address"
        );
        return Ok(None);
    };

    Ok(Some(address.to_string()))
}

fn first_ipv4_address_like_cpp(
    endpoints: impl IntoIterator<Item = SocketAddr>,
) -> Option<Ipv4Addr> {
    endpoints
        .into_iter()
        .find_map(|endpoint| match endpoint.ip() {
            IpAddr::V4(address) => Some(address),
            IpAddr::V6(_) => None,
        })
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

fn parse_hotfix_version_like_cpp(hotfix: &str) -> [u8; 4] {
    let mut bytes = [0; 4];
    let hotfix_bytes = hotfix.as_bytes();
    if hotfix_bytes.len() < bytes.len() {
        bytes[..hotfix_bytes.len()].copy_from_slice(hotfix_bytes);
    }
    bytes
}

fn parse_auth_seed_like_cpp(hex: &str) -> [u8; 16] {
    let mut bytes = [0; 16];
    if hex.len() != bytes.len() * 2 {
        return bytes;
    }

    for (idx, byte) in bytes.iter_mut().enumerate() {
        let start = idx * 2;
        let Some(parsed) = u8::from_str_radix(&hex[start..start + 2], 16).ok() else {
            return [0; 16];
        };
        *byte = parsed;
    }
    bytes
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
#[path = "tests/mod.rs"]
mod tests;
