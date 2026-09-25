// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Persistence capabilities: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, MAX_POWERS_PER_CLASS, ObjectGuid, Position};

pub(crate) type CharacterPowerSnapshotLikeCpp = [Option<i32>; MAX_POWERS_PER_CLASS];

#[cfg(test)]
pub(in crate::session) fn empty_character_power_snapshot_like_cpp() -> CharacterPowerSnapshotLikeCpp
{
    [None; MAX_POWERS_PER_CLASS]
}

pub(in crate::session) fn loaded_character_power_snapshot_like_cpp(
    powers: [i32; MAX_POWERS_PER_CLASS],
) -> CharacterPowerSnapshotLikeCpp {
    powers.map(|power| Some(power.max(0)))
}

pub(in crate::session) fn character_power_snapshot_values_like_cpp(
    powers: &CharacterPowerSnapshotLikeCpp,
) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
    let mut values = [0; MAX_POWERS_PER_CLASS];
    for (index, power) in powers.iter().copied().enumerate() {
        values[index] = power?;
    }
    Some(values)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlayerSaveToDbSnapshotLikeCpp {
    pub guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub level: u8,
    pub xp: u32,
    pub money: u64,
    pub health: u32,
    pub max_health: u32,
    pub powers: CharacterPowerSnapshotLikeCpp,
}

/// Per-player session on the world server.
///
/// Receives deserialized packets from the socket layer via a channel,
/// dispatches them to registered handlers, and sends responses back.
#[derive(Clone, Default)]
pub struct SessionAdmissionPersistenceLikeCpp {
    pub(crate) character_administration:
        Option<Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>>,
    pub(crate) character_enumeration:
        Option<Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>>,
    pub(crate) session_account_state:
        Option<Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp>>,
    pub(crate) packet_spoof_ban:
        Option<Arc<dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp>>,
    pub(crate) player_name_query:
        Option<Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>>,
    pub(crate) support_bug_report:
        Option<Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct PlayerPersistenceCapabilitiesLikeCpp {
    pub(crate) player_lifecycle: Option<Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>>,
    pub(crate) void_storage: Option<Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>>,
    pub(crate) social: Option<Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>>,
    pub(crate) stored_item_money:
        Option<Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>>,
    pub(crate) stored_item: Option<Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>>,
    pub(crate) player_inventory:
        Option<Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>>,
    pub(crate) player_quest: Option<Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>>,
    /// Commits one complete quest-reward operation as a single character
    /// transaction, the way C++ closes `Player::RewardQuest` with
    /// `SaveToDB(false)` (Player.cpp:14867).
    pub(crate) player_quest_reward:
        Option<Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>>,
    pub(crate) vendor_trade: Option<Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>>,
    pub(crate) player_spell_acquisition:
        Option<Arc<dyn wow_persistence::PlayerSpellAcquisitionPersistencePortLikeCpp>>,
    pub(crate) instance_lock: Option<Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>>,
    pub(crate) battle_pet_purchase:
        Option<Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct WorldPersistenceCapabilitiesLikeCpp {
    pub(crate) map_corpse: Option<Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>>,
    pub(crate) group_loot_money:
        Option<Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>>,
    pub(crate) represented_group:
        Option<Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct CatalogPersistenceCapabilitiesLikeCpp {
    pub(crate) quest_poi: Option<Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>>,
    pub(crate) item_template_addon_catalog:
        Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>>,
    pub(crate) loot_template_catalog:
        Option<Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>>,
    pub(crate) vendor_catalog:
        Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>>,
    pub(crate) visibility_spawn_catalog:
        Option<Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>>,
    pub(crate) gossip_catalog:
        Option<Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct SessionPersistencePortsLikeCpp {
    pub(crate) admission: SessionAdmissionPersistenceLikeCpp,
    pub(crate) player: PlayerPersistenceCapabilitiesLikeCpp,
    pub(crate) world: WorldPersistenceCapabilitiesLikeCpp,
    pub(crate) catalogs: CatalogPersistenceCapabilitiesLikeCpp,
}
