//! Shared, hermetic discovery fixture; no database or runtime mutation.
use super::*;
use crate::realm::{
    Realm, RealmBuildInfo, RealmFlagsLikeCpp, RealmHandleLikeCpp, RealmManager, RealmTypeLikeCpp,
};

pub(crate) const ADDRESS: u32 = 0x02010001;
pub(crate) const DISTRICTS: &str =
    r#"[{"superDistrictID":1,"disallowLogin":false,"holdDownUntilTime":0}]"#;
pub(crate) const BINDINGS: &str =
    r#"[{"realmAddress":33619969,"contentSetID":136,"superDistrictID":1}]"#;

pub(crate) fn fixture() -> (ForeverCatalog, RealmManager) {
    let catalog = ForeverCatalog::parse(DISTRICTS)
        .unwrap()
        .with_realm_bindings(BINDINGS)
        .unwrap();
    let mut realms = RealmManager::new();
    realms.builds.push(RealmBuildInfo {
        major_version: 1,
        minor_version: 60,
        bugfix_version: 1,
        hotfix_version: [0; 4],
        build: 70170,
        win64_auth_seed: [0; 16],
        mac64_auth_seed: [0; 16],
    });
    realms.realms.insert(
        RealmHandleLikeCpp::from_address_like_cpp(ADDRESS),
        Realm {
            id: 1,
            name: "Forever test".into(),
            normalized_name: "Forevertest".into(),
            external_address: "127.0.0.1".into(),
            local_address: "127.0.0.1".into(),
            port: 18085,
            icon: RealmTypeLikeCpp::PVP,
            flag: RealmFlagsLikeCpp::NONE,
            timezone: 1,
            allowed_security_level: 0,
            population: 0.0,
            build: 70170,
            region: 2,
            battlegroup: 1,
        },
    );
    (catalog, realms)
}

pub(crate) fn inflate(blob: &[u8]) -> String {
    use std::io::Read;
    let mut output = Vec::new();
    flate2::read::ZlibDecoder::new(&blob[4..])
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(
        u32::from_le_bytes(blob[..4].try_into().unwrap()) as usize,
        output.len()
    );
    assert_eq!(output.last(), Some(&0));
    String::from_utf8(output).unwrap()
}
