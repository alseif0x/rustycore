//! Update-object block builder operations, part 2 of 2.
//!
//! The inherent `UpdateObject` impl is divided by responsibility under
//! #650; every method keeps its original body.

#[allow(unused_imports)]
use super::super::*;
use super::*;

impl UpdateObject {
    /// Populate C++ `Unit::m_movementInfo.transport` on a player CREATE.
    ///
    /// `Map::SendInitSelf` creates the player's current transport before the
    /// player block, and the player `MovementUpdate` references it through the
    /// nested `HasTransport` branch.
    pub fn set_player_movement_transport_like_cpp(&mut self, transport: TransportInfo) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                movement: Some(movement),
                ..
            } = block
            {
                movement.transport = Some(Box::new(transport));
                return;
            }
        }
    }
    /// Populate PlayerData::WowAccount and PlayerData::BnetAccount on the
    /// self CREATE block.
    ///
    /// C++ `Player::LoadFromDB` sets these from `WorldSession` before
    /// `PlayerData::WriteCreate`.
    pub fn set_player_account_guids_like_cpp(
        &mut self,
        wow_account: ObjectGuid,
        bnet_account: ObjectGuid,
    ) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.wow_account = wow_account;
                create_data.bnet_account = bnet_account;
                return;
            }
        }
    }
    /// Create a player VALUES update for changed inventory fields.
    ///
    /// Used when items are swapped/equipped/unequipped to update the client's
    /// InvSlots (ActivePlayerData) and VisibleItems (PlayerData) without
    /// recreating the entire player object.
    pub fn player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        visible_item_changes: Vec<(u8, i32, u16, u16)>,
        virtual_item_changes: Vec<(u8, i32, u16, u16)>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes,
                buyback_changes: Vec::new(),
                visible_item_changes,
                virtual_item_changes,
                stat_changes: None,
                coinage_change: None,
            }],
        }
    }
    /// Create a player VALUES update for changed inventory and buyback fields.
    pub fn player_values_buyback_update(
        guid: ObjectGuid,
        map_id: u16,
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        buyback_changes: Vec<(u8, u32, i64)>,
        coinage: Option<u64>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes,
                buyback_changes,
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: None,
                coinage_change: coinage,
            }],
        }
    }
    /// Create a VALUES update for player coinage + optional inv slot change.
    ///
    /// Used after buy/sell to update the client's displayed gold and inventory.
    pub fn player_money_update(
        guid: ObjectGuid,
        map_id: u16,
        coinage: u64,
        inv_slot_change: Option<(u8, ObjectGuid)>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes: inv_slot_change.map(|c| vec![c]).unwrap_or_default(),
                buyback_changes: Vec::new(),
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: None,
                coinage_change: Some(coinage),
            }],
        }
    }
    /// Create a VALUES update for player stats only (after equip/desequip).
    pub fn player_stat_update(guid: ObjectGuid, map_id: u16, changes: PlayerStatChanges) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes: Vec::new(),
                buyback_changes: Vec::new(),
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: Some(changes),
                coinage_change: None,
            }],
        }
    }
    /// Create a VALUES update for the base `UF::ObjectData` section.
    ///
    /// The mask follows TrinityCore `UF::ObjectData`: bit 0 is the parent bit,
    /// bits 1/2/3 are EntryID/DynamicFlags/Scale.
    pub fn object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ObjectValuesUpdate { guid, data }],
        }
    }
    /// Create a VALUES update for `UF::DynamicObjectData`.
    pub fn dynamic_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: DynamicObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::DynamicObjectValuesUpdate { guid, data }],
        }
    }
    /// Create a VALUES update for `UF::SceneObjectData`.
    pub fn scene_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: SceneObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::SceneObjectValuesUpdate { guid, data }],
        }
    }
    /// Create a VALUES update for `UF::ConversationData`.
    pub fn conversation_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ConversationDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ConversationValuesUpdate { guid, data }],
        }
    }
    /// Create a VALUES update for `UF::GameObjectData`.
    pub fn game_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: GameObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::GameObjectValuesUpdate { guid, data }],
        }
    }
    /// Create a VALUES update for `UF::CorpseData`.
    pub fn corpse_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: CorpseDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CorpseValuesUpdate { guid, data }],
        }
    }
    /// Create a VALUES update for `UF::AreaTriggerData`.
    pub fn area_trigger_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: AreaTriggerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::AreaTriggerValuesUpdate { guid, data }],
        }
    }
    /// Create a full VALUES update for `UF::ItemData`.
    pub fn full_item_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ItemDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullItemValuesUpdate { guid, data }],
        }
    }
    /// Create a full VALUES update for `UF::UnitData`.
    pub fn unit_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: UnitDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::UnitValuesUpdate { guid, data }],
        }
    }
    /// Create a full VALUES update for `UF::PlayerData`.
    pub fn full_player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: PlayerDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullPlayerValuesUpdate { guid, data }],
        }
    }
    /// Create a full VALUES update for `UF::ActivePlayerData`.
    pub fn full_active_player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ActivePlayerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullActivePlayerValuesUpdate { guid, data }],
        }
    }
    /// Create a VALUES update for `UF::ContainerData`, with optional `ItemData`.
    pub fn container_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ContainerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ContainerValuesUpdate { guid, data }],
        }
    }
    /// Create an UpdateObject with item CREATE blocks.
    ///
    /// Each item gets its own block. Sent BEFORE the player CREATE packet
    /// so the client has item objects when it processes InvSlots.
    pub fn create_items(items: Vec<ItemCreateData>, map_id: u16) -> Self {
        Self::create_items_with_update_type(items, map_id, UpdateType::CreateObject2)
    }
    /// Create inventory item blocks from C++ `Player::_StoreItem`.
    ///
    /// `_StoreItem` calls `Item::AddToWorld` directly and then
    /// `SendUpdateToPlayer`; unlike `Map::AddToMap`, that path never raises
    /// `Object::m_isNewObject`, so `BuildCreateUpdateBlockForPlayer` writes
    /// `CreateObject` rather than `CreateObject2`.
    pub fn create_stored_items(items: Vec<ItemCreateData>, map_id: u16) -> Self {
        Self::create_items_with_update_type(items, map_id, UpdateType::CreateObject)
    }
    pub(super) fn create_items_with_update_type(
        items: Vec<ItemCreateData>,
        map_id: u16,
        update_type: UpdateType,
    ) -> Self {
        let num = items.len() as u32;
        let blocks = items
            .into_iter()
            .map(|data| {
                let guid = data.item_guid;
                UpdateBlock::CreateItem {
                    update_type,
                    guid,
                    create_data: data,
                }
            })
            .collect();

        Self {
            map_id,
            num_updates: num,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }
    /// Create an item VALUES update for changed stack count.
    pub fn item_stack_count_update(guid: ObjectGuid, map_id: u16, stack_count: u32) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ItemValuesUpdate {
                guid,
                stack_count,
                dynamic_flags: None,
            }],
        }
    }
    /// Create the single ItemData VALUES update emitted by C++ `_StoreItem`
    /// when an existing stack changes both count and binding flags.
    pub fn item_stack_count_and_flags_update(
        guid: ObjectGuid,
        map_id: u16,
        stack_count: u32,
        dynamic_flags: u32,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ItemValuesUpdate {
                guid,
                stack_count,
                dynamic_flags: Some(dynamic_flags),
            }],
        }
    }
}
