//! Build-70170 realm-discovery projection. The client metadata and decoder
//! anchors are recorded in docs/operations/forever-login.md. This immutable
//! operator catalog is not an inferred mapping from 3.4.3 realm IDs.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

mod bindings;
#[cfg(test)]
pub(crate) mod test_fixture;

const MAX_DISTRICTS: usize = 64;

#[derive(Debug, Default)]
pub(crate) struct ForeverCatalog {
    districts: Vec<SuperDistrict>,
    bindings: Vec<bindings::RealmBinding>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SuperDistrict {
    #[serde(rename = "superDistrictID")]
    id: i32,
    #[serde(rename = "disallowLogin")]
    disallow_login: bool,
    #[serde(rename = "holdDownUntilTime")]
    hold_down_until_time: u32,
}

impl ForeverCatalog {
    pub(crate) fn parse(config: &str) -> Result<Self> {
        // Bounded operator input. No implicit district IDs or mutable cache.
        if config.len() > 16 * 1024 {
            bail!("Forever.SuperDistricts exceeds the 16 KiB configuration limit");
        }
        let districts: Vec<SuperDistrict> = serde_json::from_str(config)?;
        if districts.len() > MAX_DISTRICTS {
            bail!("Forever.SuperDistricts exceeds the 64-entry configuration limit");
        }
        let mut ids = HashSet::new();
        for district in &districts {
            if district.id <= 0 || !ids.insert(district.id) {
                bail!("Forever.SuperDistricts requires positive, unique district IDs");
            }
        }
        Ok(Self {
            districts,
            bindings: Vec::new(),
        })
    }

    pub(crate) fn compressed_list(&self) -> Result<Vec<u8>> {
        #[derive(Serialize)]
        struct List<'a> {
            #[serde(rename = "superDistricts")]
            districts: &'a [SuperDistrict],
        }
        let json = serde_json::to_string(&List {
            districts: &self.districts,
        })?;
        // Jam JSON blob: u32 LE length including NUL, followed by zlib data.
        // Do not change the inherited V1 realm-list compressor's callers.
        Ok(super::zlib_compress(
            format!("JSONSuperDistrictList:{json}\0").as_bytes(),
        ))
    }
}

pub(crate) fn compressed_empty_bleep_proxies() -> Vec<u8> {
    // TC 6ebe044c Shared::GameUtilities::GetBleepProxies and
    // RealmList.proto::BleepProxyList. No proxy endpoint is advertised.
    super::zlib_compress(b"JSONBleepProxyList:{\"proxies\":[]}\0")
}

pub(crate) fn compressed_utility_info() -> Vec<u8> {
    // Match Forever GetLastCharPlayed's minimal account realm permissions.
    // No login licenses are advertised by this projection.
    super::zlib_compress(b"JSONUtilityInfo:{\"realmPermissions\":512}\0")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn inflate(blob: &[u8]) -> String {
        let mut data = Vec::new();
        flate2::read::ZlibDecoder::new(&blob[4..])
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(
            u32::from_le_bytes(blob[..4].try_into().unwrap()) as usize,
            data.len()
        );
        String::from_utf8(data).unwrap()
    }

    #[test]
    fn configured_values_and_order_are_preserved_in_the_exact_json_envelope() {
        let config = r#"[{"superDistrictID":7,"disallowLogin":true,"holdDownUntilTime":4294967295},{"superDistrictID":3,"disallowLogin":false,"holdDownUntilTime":0}]"#;
        let blob = ForeverCatalog::parse(config)
            .unwrap()
            .compressed_list()
            .unwrap();
        assert_eq!(
            inflate(&blob),
            format!("JSONSuperDistrictList:{{\"superDistricts\":{config}}}\0")
        );
        assert_eq!(
            inflate(&ForeverCatalog::default().compressed_list().unwrap()),
            "JSONSuperDistrictList:{\"superDistricts\":[]}\0"
        );
        assert_eq!(
            inflate(&compressed_empty_bleep_proxies()),
            "JSONBleepProxyList:{\"proxies\":[]}\0"
        );
    }

    #[test]
    fn invalid_operator_catalog_fails_closed() {
        for value in [
            "{}",
            r#"[{"superDistrictID":0,"disallowLogin":false,"holdDownUntilTime":0}]"#,
            r#"[{"superDistrictID":-1,"disallowLogin":false,"holdDownUntilTime":0}]"#,
            r#"[{"superDistrictID":2147483648,"disallowLogin":false,"holdDownUntilTime":0}]"#,
            r#"[{"superDistrictID":1,"disallowLogin":false,"holdDownUntilTime":-1}]"#,
            r#"[{"superDistrictID":1,"disallowLogin":false,"holdDownUntilTime":4294967296}]"#,
            r#"[{"superDistrictID":1,"disallowLogin":false}]"#,
            r#"[{"superDistrictID":1,"disallowLogin":false,"holdDownUntilTime":0,"typo":1}]"#,
            r#"[{"superDistrictID":1,"disallowLogin":false,"holdDownUntilTime":0},{"superDistrictID":1,"disallowLogin":true,"holdDownUntilTime":0}]"#,
        ] {
            assert!(ForeverCatalog::parse(value).is_err(), "accepted {value}");
        }
        assert!(ForeverCatalog::parse(&" ".repeat(16385)).is_err());
        let large = (1..=65)
            .map(|id| {
                format!(r#"{{"superDistrictID":{id},"disallowLogin":false,"holdDownUntilTime":0}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        assert!(ForeverCatalog::parse(&format!("[{large}]")).is_err());
    }
}
