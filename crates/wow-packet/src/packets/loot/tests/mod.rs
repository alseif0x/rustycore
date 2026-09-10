//! Loot packet regressions.
//!
//! Separated from loot.rs under #685.

use crate::{ClientPacket, ServerPacket};

use super::{
    AELootTargets, AELootTargetsAck, CoinRemoved, LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP, LootAllPassed, LootCurrencyData, LootItemData,
    LootList, LootMoney, LootMoneyNotify, LootReleaseAll, LootRemoved, LootResponse, LootRoll,
    LootRollBroadcast, LootRollWon, LootRollsComplete, MasterLootCandidateList, MasterLootItem,
    SLootRelease, SetLootSpecialization, StartLootRoll,
};
use crate::packets::item::ItemInstance;
use crate::world_packet::WorldPacket;

fn roll_test_item() -> LootItemData {
    LootItemData {
        item_type: 0,
        ui_type: 5,
        can_trade_to_tap_list: false,
        loot: ItemInstance {
            item_id: 25,
            ..ItemInstance::default()
        },
        loot_list_id: 9,
        quantity: 1,
        loot_item_type: 0,
    }
}

mod scenarios;
