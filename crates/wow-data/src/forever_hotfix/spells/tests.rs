mod core;
mod costs;
mod custom_sources;
mod dependencies;
mod value_inputs;
use super::super::{ForeverHotfixCatalog, ForeverTactKeys, RecordError, TACT_KEY_TABLE_HASH};
use crate::forever_spells::*;
use crate::{Db2HotfixRemovalStoreLikeCpp, HotfixBlobCache};
use std::{collections::BTreeMap, sync::Arc};

fn inputs() -> Arc<SpellCatalog> {
    Arc::new(
        SpellRecords {
            spell_cast_times: vec![SpellCastTimesRecord {
                id: 7,
                base: -1,
                minimum: -2,
            }],
            ..Default::default()
        }
        .finish(
            Default::default(),
            Default::default(),
            6,
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap(),
    )
}
fn catalog(all: bool) -> ForeverHotfixCatalog {
    let mut metadata = HotfixBlobCache::new();
    metadata.register_typed_table(TACT_KEY_TABLE_HASH);
    metadata.register_typed_table(0x12345678);
    if all {
        for hash in SPELL_TABLE_HASHES {
            metadata.register_typed_table(hash);
        }
    }
    ForeverHotfixCatalog::new(
        ForeverTactKeys {
            records: BTreeMap::new(),
        },
        metadata,
    )
    .unwrap()
}
#[test]
fn delivery_requires_registered_stores_and_shares_the_effective_allocation() {
    let data = inputs();
    assert!(catalog(false).with_spell_stores(data.clone()).is_err());
    let before = catalog(true);
    assert!(!before.has_serializer(SPELL_TABLE_HASHES[25]));
    assert_eq!(
        before
            .write_record(SPELL_TABLE_HASHES[25], 7, 6)
            .unwrap_err(),
        RecordError::UnportedStore
    );
    let delivered = before.with_spell_stores(data.clone()).unwrap();
    assert!(Arc::ptr_eq(
        &delivered.spell_stores.as_ref().unwrap().data,
        &data
    ));
    for hash in SPELL_TABLE_HASHES {
        assert!(delivered.has_serializer(hash));
    }
    assert_eq!(
        delivered
            .write_record(SPELL_TABLE_HASHES[25], 7, 6)
            .unwrap()
            .unwrap()
            .as_ref(),
        &[255, 255, 255, 255, 254, 255, 255, 255]
    );
    assert!(
        delivered
            .write_record(SPELL_TABLE_HASHES[25], 8, 6)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        delivered.write_record(0x12345678, 7, 6).unwrap_err(),
        RecordError::UnportedStore
    );
    assert_eq!(
        delivered
            .write_record(SPELL_TABLE_HASHES[25], 7, 9)
            .unwrap_err(),
        RecordError::InvalidLocale
    );
    assert!(delivered.write_record(0x76543210, 7, 6).unwrap().is_none());
}

#[test]
fn spell_text_write_has_no_en_us_fallback_and_uses_the_source_c_string_prefix() {
    let value = SpellText::from_locale(0, vec![0xFF, 0, b'a']).unwrap();
    let mut bytes = Vec::new();
    super::text(&mut bytes, &value, 6);
    assert_eq!(bytes, [0]);
    bytes.clear();
    super::text(&mut bytes, &value, 0);
    assert_eq!(bytes, [0xFF, 0]);
}
