use super::*;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{
        item_records::ItemRecords,
        item_specs::{ITEM_SPEC_HASH, ItemSpecRecord, ItemSpecRecords},
    },
    forever_hotfix::ITEM_TABLE_HASHES,
};

fn item_catalog(metadata: HotfixBlobCache) -> ForeverHotfixCatalog {
    let removals = Db2HotfixRemovalStoreLikeCpp::default();
    let items = ItemRecords::default()
        .finish(Default::default(), Default::default(), &removals)
        .unwrap();
    let specs = ItemSpecRecords {
        specs: vec![ItemSpecRecord {
            id: 7,
            min_level: 1,
            max_level: 255,
            item_type: 2,
            primary: 3,
            secondary: 40,
            specialization: 0x0102,
        }],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &removals)
    .unwrap();
    hotfix_catalog(metadata)
        .with_item_stores(Arc::new(items), Arc::new(specs))
        .unwrap()
}
fn metadata() -> HotfixBlobCache {
    let mut metadata = HotfixBlobCache::new();
    for hash in ITEM_TABLE_HASHES {
        metadata.register_typed_table(hash);
    }
    metadata
}

#[tokio::test]
async fn typed_item_bulk_uses_same_registry_and_source_reply_bytes_in_requested_order() {
    let mut session = session(Arc::new(Repository::good()));
    session.hotfixes = Arc::new(item_catalog(metadata()));
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    let output = session
        .dispatch(&catalog, bulk_request(ITEM_SPEC_HASH, &[7, 99, 7]))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(output.len(), 3);
    for (reply, id) in output.iter().zip([7u32, 99, 7]) {
        assert_eq!(reply.opcode(), 0x4A0000);
        let bytes = reply.payload();
        assert_eq!(&bytes[..4], &ITEM_SPEC_HASH.to_le_bytes());
        assert_eq!(&bytes[4..8], &id.to_le_bytes());
        if id == 99 {
            assert_eq!(&bytes[12..], &[0x60, 0, 0, 0, 0]);
        } else {
            assert_eq!(&bytes[12..], &[0x20, 7, 0, 0, 0, 1, 255, 2, 3, 40, 2, 1]);
        }
    }
    assert!(!session.is_closed());
}

#[tokio::test]
async fn typed_item_hotfix_preserves_requested_push_order_and_missing_record_downgrade() {
    let mut metadata = metadata();
    metadata.apply_hotfix_data_rows_like_cpp(
        [(1, 1, ITEM_SPEC_HASH, 7, 1), (2, 2, ITEM_SPEC_HASH, 99, 1)],
        "esES",
    );
    let mut session = session(Arc::new(Repository::good()));
    session.hotfixes = Arc::new(item_catalog(metadata));
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    let mut payload = 70170u32.to_le_bytes().repeat(2);
    payload.extend_from_slice(&2u32.to_le_bytes());
    for id in [2i32, 1] {
        payload.extend_from_slice(&id.to_le_bytes());
    }
    let output = session
        .dispatch(
            &catalog,
            Request {
                opcode: HOTFIX_REQUEST,
                payload,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(output.len(), 1);
    let bytes = output[0].payload();
    assert_eq!(output[0].opcode(), 0x4A0003);
    assert_eq!(&bytes[..4], &2u32.to_le_bytes());
    assert_eq!((bytes[24], bytes[45]), (0x40, 0x20));
    assert_eq!(&bytes[46..50], &7u32.to_le_bytes());
    assert_eq!(&bytes[50..], &[1, 255, 2, 3, 40, 2, 1]);
}
