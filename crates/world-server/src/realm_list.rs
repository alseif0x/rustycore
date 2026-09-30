// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Realm-list normalization, snapshot construction, and refresh lifecycle.

use std::time::Duration;

use anyhow::{Context, Result};
use tracing::{debug, warn};
use wow_database::{LoginDatabase, LoginStatements, SqlResult};

use super::{
    MAX_CLIENT_REALM_TYPE_LIKE_CPP, REALM_TYPE_FFA_PVP_LIKE_CPP, REALM_TYPE_NORMAL_LIKE_CPP,
    REALM_TYPE_PVP_LIKE_CPP, REALM_TYPE_RPPVP_LIKE_CPP, RealmHandleLikeCpp, RealmListEntryLikeCpp,
    RealmListRawRowLikeCpp, RealmListRefreshSummaryLikeCpp, RealmListSnapshotLikeCpp,
    SEC_ADMINISTRATOR_LIKE_CPP, SharedRealmListLikeCpp,
};

impl RealmHandleLikeCpp {
    pub(super) fn new_like_cpp(region: u8, site: u8, realm: u32) -> Self {
        Self {
            region,
            site,
            realm,
        }
    }

    pub(super) fn address_like_cpp(self) -> u32 {
        (u32::from(self.region) << 24) | (u32::from(self.site) << 16) | (self.realm & 0xFFFF)
    }

    #[cfg(test)]
    pub(super) fn address_string_like_cpp(self) -> String {
        format!("{}-{}-{}", self.region, self.site, self.realm)
    }

    pub(super) fn sub_region_address_like_cpp(self) -> String {
        format!("{}-{}-0", self.region, self.site)
    }
}

impl PartialEq for RealmHandleLikeCpp {
    fn eq(&self, other: &Self) -> bool {
        self.realm == other.realm
    }
}

impl Eq for RealmHandleLikeCpp {}

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

impl RealmListSnapshotLikeCpp {
    pub(super) fn replace_like_cpp(&mut self, next: Self) -> RealmListRefreshSummaryLikeCpp {
        let added = next
            .realms
            .keys()
            .filter(|handle| !self.realms.contains_key(handle))
            .count();
        let updated = next
            .realms
            .keys()
            .filter(|handle| self.realms.contains_key(handle))
            .count();
        let removed = self
            .realms
            .keys()
            .filter(|handle| !next.realms.contains_key(handle))
            .count();
        let realms = next.realms.len();
        let sub_regions = next.sub_regions.len();

        *self = next;

        RealmListRefreshSummaryLikeCpp {
            realms,
            sub_regions,
            added,
            updated,
            removed,
        }
    }

    #[cfg(test)]
    pub(super) fn get_realm_like_cpp(
        &self,
        handle: RealmHandleLikeCpp,
    ) -> Option<&RealmListEntryLikeCpp> {
        self.realms.get(&handle)
    }

    pub(super) fn get_realm_by_id_like_cpp(&self, realm_id: u32) -> Option<&RealmListEntryLikeCpp> {
        self.realms
            .get(&RealmHandleLikeCpp::new_like_cpp(0, 0, realm_id))
    }
}

pub(super) fn realms_state_update_delay_secs_like_cpp() -> u32 {
    wow_config::get_value_default("RealmsStateUpdateDelay", 10i32).max(0) as u32
}

pub(super) fn normalize_realm_type_like_cpp(icon: u8) -> u8 {
    if icon == REALM_TYPE_FFA_PVP_LIKE_CPP {
        return REALM_TYPE_PVP_LIKE_CPP;
    }

    if icon >= MAX_CLIENT_REALM_TYPE_LIKE_CPP {
        return REALM_TYPE_NORMAL_LIKE_CPP;
    }

    icon
}

pub(super) fn is_pvp_realm_type_like_cpp(icon: u32) -> bool {
    matches!(
        icon,
        value if value == u32::from(REALM_TYPE_PVP_LIKE_CPP)
            || value == u32::from(REALM_TYPE_RPPVP_LIKE_CPP)
            || value == u32::from(REALM_TYPE_FFA_PVP_LIKE_CPP)
    )
}

pub(super) fn is_ffa_pvp_realm_type_like_cpp(icon: u32) -> bool {
    icon == u32::from(REALM_TYPE_FFA_PVP_LIKE_CPP)
}

pub(super) fn normalize_realm_security_level_like_cpp(level: u8) -> u8 {
    level.min(SEC_ADMINISTRATOR_LIKE_CPP)
}

pub(super) fn normalized_realm_name_like_cpp(name: &str) -> String {
    name.chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect()
}

pub(super) fn realm_list_entry_from_row_like_cpp(
    row: RealmListRawRowLikeCpp,
) -> RealmListEntryLikeCpp {
    let id = RealmHandleLikeCpp::new_like_cpp(row.region, row.battlegroup, row.realm_id);
    let normalized_name = normalized_realm_name_like_cpp(&row.name);
    RealmListEntryLikeCpp {
        id,
        build: row.build,
        name: row.name,
        normalized_name,
        address: row.address,
        local_address: row.local_address,
        port: row.port,
        icon: normalize_realm_type_like_cpp(row.icon),
        flag: row.flag,
        timezone: row.timezone,
        allowed_security_level: normalize_realm_security_level_like_cpp(row.allowed_security_level),
        population: row.population,
    }
}

pub(super) fn realm_list_snapshot_from_result_like_cpp(
    result: &mut SqlResult,
) -> RealmListSnapshotLikeCpp {
    let mut snapshot = RealmListSnapshotLikeCpp::default();
    if result.is_empty() {
        return snapshot;
    }

    loop {
        let Some(fields) = result.fetch_like_cpp() else {
            break;
        };
        let entry = realm_list_entry_from_row_like_cpp(RealmListRawRowLikeCpp {
            realm_id: fields.try_read(0).unwrap_or(0),
            name: fields.read_string(1),
            address: fields.read_string(2),
            local_address: fields.read_string(3),
            port: fields.try_read(4).unwrap_or(0),
            icon: fields.try_read(5).unwrap_or(REALM_TYPE_NORMAL_LIKE_CPP),
            flag: fields.try_read(6).unwrap_or(0),
            timezone: fields.try_read(7).unwrap_or(0),
            allowed_security_level: fields.try_read(8).unwrap_or(SEC_ADMINISTRATOR_LIKE_CPP),
            population: fields.try_read(9).unwrap_or(0.0),
            build: fields.try_read(10).unwrap_or(0),
            region: fields.try_read(11).unwrap_or(0),
            battlegroup: fields.try_read(12).unwrap_or(0),
        });

        snapshot
            .sub_regions
            .insert(entry.id.sub_region_address_like_cpp());
        snapshot.realms.insert(entry.id, entry);

        if !result.next_row() {
            break;
        }
    }

    snapshot
}

pub(super) async fn update_realm_list_once_like_cpp(
    login_db: &LoginDatabase,
    realm_list: &SharedRealmListLikeCpp,
) -> Result<RealmListRefreshSummaryLikeCpp> {
    let stmt = login_db.prepare(LoginStatements::SEL_REALMLIST);
    let mut result = login_db
        .query(&stmt)
        .await
        .context("Failed to query C++ LOGIN_SEL_REALMLIST")?;
    let next_snapshot = realm_list_snapshot_from_result_like_cpp(&mut result);
    let mut realm_list = realm_list.lock().expect("realm list mutex poisoned");
    Ok(realm_list.replace_like_cpp(next_snapshot))
}

pub(super) fn spawn_realm_list_update_loop_like_cpp(
    login_db: LoginDatabase,
    realm_list: SharedRealmListLikeCpp,
    update_interval_secs: u32,
) -> Option<tokio::task::JoinHandle<()>> {
    if update_interval_secs == 0 {
        warn!("RealmsStateUpdateDelay is 0; RealmList background refresh disabled");
        return None;
    }

    Some(tokio::spawn(async move {
        let interval = Duration::from_secs(u64::from(update_interval_secs));
        loop {
            tokio::time::sleep(interval).await;
            match update_realm_list_once_like_cpp(&login_db, &realm_list).await {
                Ok(summary) => {
                    debug!(
                        realms = summary.realms,
                        sub_regions = summary.sub_regions,
                        added = summary.added,
                        updated = summary.updated,
                        removed = summary.removed,
                        "Updated RealmList from realmlist like C++"
                    );
                }
                Err(error) => {
                    warn!("RealmList background refresh failed: {error:#}");
                }
            }
        }
    }))
}
