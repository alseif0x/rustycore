use super::{BUYBACK_SLOT_START, Item, MAX_BAG_SIZE, ObjectGuid, PLAYER_SLOT_END, is_buyback_slot};

/// Persistent identity and template metadata for one Player-owned item.
///
/// C++ stores the concrete `Item*` directly in `Player::m_items`
/// (`Player.h:2935`). Rust keeps this small record alongside the concrete
/// [`Item`] so database identity and the effective inventory type travel with
/// the same canonical Player lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInventoryItem {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub db_guid: u64,
    pub inventory_type: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBagStorage {
    pub bag_guid: ObjectGuid,
    pub bag_size: u8,
    pub slots: [Option<ObjectGuid>; MAX_BAG_SIZE],
}

impl PlayerBagStorage {
    pub fn new(bag_guid: ObjectGuid, bag_size: u8) -> Self {
        assert!(bag_size as usize <= MAX_BAG_SIZE);
        Self {
            bag_guid,
            bag_size,
            slots: [None; MAX_BAG_SIZE],
        }
    }

    /// Bag records are held behind a pointer in `PlayerInventoryStorage::bags`; this keeps
    /// the call sites one line, matching the physical-source ceiling on their file.
    pub fn boxed(bag_guid: ObjectGuid, bag_size: u8) -> Box<Self> {
        Box::new(Self::new(bag_guid, bag_size))
    }

    pub fn item_by_pos(&self, slot: u8) -> Option<ObjectGuid> {
        if slot < self.bag_size {
            self.slots[slot as usize]
        } else {
            None
        }
    }

    pub fn set_item(&mut self, slot: u8, guid: Option<ObjectGuid>) {
        assert!((slot as usize) < MAX_BAG_SIZE);
        self.slots[slot as usize] = guid;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInventoryStorage {
    pub items: [Option<ObjectGuid>; PLAYER_SLOT_END],
    /// Only the slots `is_bag_storage_slot` accepts can ever hold a bag, so the record is
    /// reached through a pointer the way C++ reaches it through `Bag*` (`Player.h:1314`,
    /// backed by `Item* m_items[PLAYER_SLOTS_COUNT]` at `Player.h:2950`) rather than being
    /// inlined into all `PLAYER_SLOT_END` slots.
    pub bags: [Option<Box<PlayerBagStorage>>; PLAYER_SLOT_END],
    pub current_buyback_slot: u8,
}

impl PlayerInventoryStorage {
    /// Borrow the bag record at an absolute player slot, hiding the pointer indirection
    /// from callers so they read the same as they did when the record was inlined.
    pub fn bag_at(&self, slot: u8) -> Option<&PlayerBagStorage> {
        self.bags[slot as usize].as_deref()
    }

    pub fn get_item_by_guid_everywhere(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        self.items
            .iter()
            .enumerate()
            .filter(|(slot, _)| !is_buyback_slot(*slot as u8))
            .find_map(|(_, item_guid)| (*item_guid == Some(guid)).then_some(guid))
            .or_else(|| {
                self.bags
                    .iter()
                    .filter_map(Option::as_ref)
                    .flat_map(|bag| bag.slots.into_iter().take(bag.bag_size as usize))
                    .find_map(|item_guid| (item_guid == Some(guid)).then_some(guid))
            })
    }
}

impl Default for PlayerInventoryStorage {
    fn default() -> Self {
        Self {
            items: [None; PLAYER_SLOT_END],
            bags: [const { None }; PLAYER_SLOT_END],
            current_buyback_slot: BUYBACK_SLOT_START,
        }
    }
}
