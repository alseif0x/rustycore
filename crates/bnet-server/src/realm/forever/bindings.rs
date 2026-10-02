//! Explicit Forever routing, without changing the inherited Auth schema.
//! advocaite/TrinityCore 02245dcd: RealmList::{GetRealmIdForContentSet,
//! FillRealmEntry,GetRealmEntryJSON}, Realm.h legacy population conversion.
//! Exact field spelling is also present in the approved 70170 PE descriptor.

use super::ForeverCatalog;
use crate::realm::{RealmFlagsLikeCpp as Flags, RealmHandleLikeCpp, RealmManager};
use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RealmBinding {
    #[serde(rename = "realmAddress")]
    address: u32,
    #[serde(rename = "contentSetID")]
    content_set: i32,
    #[serde(rename = "superDistrictID")]
    district: i32,
}

impl ForeverCatalog {
    pub(crate) fn with_realm_bindings(mut self, config: &str) -> Result<Self> {
        if config.len() > 16 * 1024 {
            bail!("Forever.RealmBindings exceeds the 16 KiB configuration limit");
        }
        let bindings: Vec<RealmBinding> = serde_json::from_str(config)?;
        if bindings.len() > 64 {
            bail!("Forever.RealmBindings exceeds the 64-entry configuration limit");
        }
        let mut addresses = HashSet::new();
        let mut content_sets = HashSet::new();
        for binding in &bindings {
            let address = RealmHandleLikeCpp::from_address_like_cpp(binding.address);
            if address.region == 0
                || address.site == 0
                || address.realm == 0
                || binding.content_set <= 0
                || !addresses.insert(binding.address)
                || !content_sets.insert(binding.content_set)
                || !self.districts.iter().any(|d| d.id == binding.district)
            {
                bail!(
                    "Forever.RealmBindings requires unique valid realm/content IDs and a configured district"
                );
            }
        }
        self.bindings = bindings;
        Ok(self)
    }

    pub(crate) fn realm_for_content(&self, content_set: i32) -> Option<u32> {
        self.bindings
            .iter()
            .find(|b| b.content_set == content_set)
            .map(|b| b.address)
    }

    /// Read canonical realm state on every request; config never overrides its
    /// online/build/security gates. No fallback into an unrelated ruleset.
    pub(crate) fn realm_entry(
        &self,
        realms: &RealmManager,
        address: u32,
        build: u32,
        security: u8,
        now: u64,
    ) -> Option<Vec<u8>> {
        if build != 70170 {
            return None;
        }
        let binding = self.bindings.iter().find(|b| b.address == address)?;
        let district = self.districts.iter().find(|d| d.id == binding.district)?;
        if district.disallow_login || u64::from(district.hold_down_until_time) > now {
            return None;
        }
        let realm = realms.get_realm_by_realm_address_like_cpp(address)?;
        // The inherited lookup compares only the low realm ID. Bindings must
        // match the complete current region/site/realm, not silently alias it.
        let actual = RealmHandleLikeCpp::new_like_cpp(realm.region, realm.battlegroup, realm.id);
        if actual.get_address_like_cpp() != address
            || realm.id > u16::MAX as u32
            || realm.flag.contains(Flags::OFFLINE)
            || realm.build != build
            || security < realm.allowed_security_level
        {
            return None;
        }
        let version = realms.get_build_info(build)?;
        let entry = json!({
            "wowRealmAddress": address,
            "cfgTimezonesID": 1,
            "populationState": population(realm.flag, realm.population),
            "cfgCategoriesID": realm.timezone,
            "version": {
                "versionMajor": version.major_version, "versionMinor": version.minor_version,
                "versionRevision": version.bugfix_version, "versionBuild": build,
            },
            "cfgRealmsID": realm.id,
            "flags": realm.flag.bits() & Flags::VERSION_MISMATCH.bits(),
            "name": realm.name,
            "cfgConfigsID": realm.icon.get_config_id_like_cpp(),
            "cfgLanguagesID": 1,
            "cfgContentSetID": binding.content_set,
            "superDistrictID": binding.district,
            "useBleepChance": 0.0,
        });
        Some(crate::realm::zlib_compress(
            format!("JamJSONRealmEntry:{entry}\0").as_bytes(),
        ))
    }
}

fn population(flags: Flags, value: f32) -> u8 {
    if flags.contains(Flags::RECOMMENDED) {
        5
    } else if flags.contains(Flags::NEW) {
        4
    } else if flags.contains(Flags::FULL) || value > 0.95 {
        6
    } else if value > 0.66 {
        3
    } else if value > 0.33 {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests;
