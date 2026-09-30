//! Process-wide identifier allocator initialization under exclusive DB ownership.

use std::sync::Arc;

use anyhow::Context;
use tracing::info;
use wow_core::{
    EquipmentSetGuidGeneratorLikeCpp, ObjectGuidGenerator, VoidStorageItemIdGeneratorLikeCpp,
    guid::HighGuid,
};
use wow_database::{CharStatements, CharacterDatabase, ItemGuidAllocatorAdvisoryLockLikeCpp};
use wow_world::session::SessionIdGeneratorsLikeCpp;

use crate::{
    item_guid_reference_cleanup_transaction_like_cpp,
    next_equipment_set_guid_allocator_start_like_cpp, next_item_guid_allocator_start_like_cpp,
    next_void_storage_item_id_allocator_start_like_cpp,
};

pub(super) async fn initialize_guid_allocators(
    char_db: &CharacterDatabase,
) -> anyhow::Result<(
    SessionIdGeneratorsLikeCpp,
    ItemGuidAllocatorAdvisoryLockLikeCpp,
)> {
    // Initialize GUID generator from MAX(guid) in characters table
    let max_guid = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_GUID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    1i64
                } else {
                    let max_val: u32 = result.try_read(0).unwrap_or(0);
                    (max_val as i64) + 1
                }
            }
            Err(_) => 1i64,
        }
    };

    let guid_generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Player, max_guid));
    info!("GUID generator initialized, next counter: {max_guid}");

    // A process-local atomic generator is safe only while one world-server can
    // allocate for this character database. Hold a connection-scoped MySQL
    // advisory lock for the complete server lifetime, failing startup if a
    // rolling/duplicate process already owns that allocation domain.
    let item_guid_allocator_advisory_lock =
        ItemGuidAllocatorAdvisoryLockLikeCpp::acquire_like_cpp(char_db.pool())
            .await
            .context("failed to acquire the character DB item GUID allocator lock")?;

    // C++ `ObjectMgr::SetHighestGuids` initializes one process-wide item
    // generator from `MAX(item_instance.guid) + 1`.  Sharing the atomic Rust
    // mirror across every session prevents concurrent loot grants from
    // selecting the same database GUID.
    let next_item_guid = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    next_item_guid_allocator_start_like_cpp(None)?
                } else {
                    let max_val: u64 = result
                        .try_read(0)
                        .context("failed to decode MAX(item_instance.guid)")?;
                    next_item_guid_allocator_start_like_cpp(Some(max_val))?
                }
            }
            Err(error) => {
                return Err(error)
                    .context("failed to initialize item GUID allocator from item_instance");
            }
        }
    };
    let next_item_guid_u64 = u64::try_from(next_item_guid)
        .context("item GUID allocator start must be a positive database counter")?;
    char_db
        .commit_transaction(item_guid_reference_cleanup_transaction_like_cpp(
            char_db,
            next_item_guid_u64,
        ))
        .await
        .context("failed to clean dangling item GUID references before allocator publication")?;
    let item_guid_generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Item, next_item_guid));
    info!("Item GUID generator initialized, next counter: {next_item_guid}");

    // C++ `ObjectMgr::SetHighestGuids` owns one raw uint64 namespace for both
    // equipment sets and transmog outfits. It must be initialized only after
    // the process has exclusive ownership of this character database's GUID
    // allocation domain (the advisory lock above).
    let next_equipment_set_guid = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_EQUIPMENT_SET_GUID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    next_equipment_set_guid_allocator_start_like_cpp(None)?
                } else {
                    let max_val: u64 = result
                        .try_read(0)
                        .context("failed to decode the equipment/transmog set GUID maximum")?;
                    next_equipment_set_guid_allocator_start_like_cpp(Some(max_val))?
                }
            }
            Err(error) => {
                return Err(error).context(
                    "failed to initialize the equipment-set GUID allocator from character_equipmentsets/character_transmog_outfits",
                );
            }
        }
    };
    let equipment_set_guid_generator = Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(
        next_equipment_set_guid,
    ));
    info!("Equipment-set GUID generator initialized, next counter: {next_equipment_set_guid}");

    // C++ `ObjectMgr::SetHighestGuids` initializes a second raw uint64
    // namespace for `character_void_storage.itemId`. Keep it under the same
    // process/CharacterDB allocator ownership lock as item and equipment IDs.
    let next_void_storage_item_id = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_VOID_STORAGE_ITEM_ID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    next_void_storage_item_id_allocator_start_like_cpp(None)?
                } else {
                    let max_val: u64 = result
                        .try_read(0)
                        .context("failed to decode the void-storage item ID maximum")?;
                    next_void_storage_item_id_allocator_start_like_cpp(Some(max_val))?
                }
            }
            Err(error) => {
                return Err(error).context(
                    "failed to initialize the void-storage item ID allocator from character_void_storage",
                );
            }
        }
    };
    let void_storage_item_id_generator = Arc::new(VoidStorageItemIdGeneratorLikeCpp::new(
        next_void_storage_item_id,
    ));
    info!("Void-storage item ID generator initialized, next counter: {next_void_storage_item_id}");

    Ok((
        SessionIdGeneratorsLikeCpp {
            player: guid_generator,
            item: item_guid_generator,
            equipment_set: equipment_set_guid_generator,
            void_storage_item: void_storage_item_id_generator,
        },
        item_guid_allocator_advisory_lock,
    ))
}
