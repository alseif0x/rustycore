// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact source grammar for finitely declared owner-specific `RegistryBuilder` registrars.

use std::collections::BTreeSet;
use std::path::Path;

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::visit::Visit;
use syn::{
    Attribute, Expr, FnArg, Item, ItemFn, Lit, Meta, Pat, Stmt, Type, TypeParamBound, UseTree,
    Visibility, WherePredicate,
};

use super::ident_is;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RegistrarFacadeContract {
    pub(crate) module: &'static str,
    pub(crate) child: &'static str,
    pub(crate) exports: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DirectRegistrarContract {
    pub(crate) owner: &'static str,
    pub(crate) package: &'static str,
    pub(crate) module: &'static str,
    pub(crate) registrar: &'static str,
    pub(crate) host_trait: &'static str,
    pub(crate) production_type_args: &'static [&'static str],
    pub(crate) facades: &'static [RegistrarFacadeContract],
}

const INVENTORY_ROOT_EXPORTS: &[&str] = &[
    "AuctionHandlerCxLikeCpp",
    "ItemTextQueryHandlerCxLikeCpp",
    "EquipmentSetsHandlerCxLikeCpp",
    "InventoryHandlerHostLikeCpp",
    "EquipmentSetsSaveCxLikeCpp",
    "register_inventory_handlers_like_cpp",
];
const INVENTORY_HANDLER_EXPORTS: &[&str] = &[
    "ItemTextQueryHandlerCxLikeCpp",
    "EquipmentSetsHandlerCxLikeCpp",
    "InventoryHandlerHostLikeCpp",
    "register_inventory_handlers_like_cpp",
];
const INVENTORY_FACADES: &[RegistrarFacadeContract] = &[
    RegistrarFacadeContract {
        module: "crate",
        child: "handlers",
        exports: INVENTORY_ROOT_EXPORTS,
    },
    RegistrarFacadeContract {
        module: "crate::handlers",
        child: "equipment_sets",
        exports: INVENTORY_HANDLER_EXPORTS,
    },
];

pub(crate) const INVENTORY_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "Inventory",
    package: "wow-world-inventory",
    module: "crate::handlers::equipment_sets",
    registrar: "register_inventory_handlers_like_cpp",
    host_trait: "InventoryHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: INVENTORY_FACADES,
};

const INSTANCES_ROOT_EXPORTS: &[&str] = &[
    "handle_instance_lock_response_like_cpp",
    "handle_request_raid_info_like_cpp",
    "handle_reset_instances_like_cpp",
    "handle_set_difficulty_id_like_cpp",
    "handle_set_dungeon_difficulty_like_cpp",
    "handle_set_raid_difficulty_like_cpp",
    "handle_set_saved_instance_extend_like_cpp",
    "handle_toggle_difficulty_like_cpp",
    "register_instance_handlers_like_cpp",
    "reset_represented_instances_like_cpp",
    "InstanceDifficultyHandlerCxLikeCpp",
    "InstanceLockOperationsHandlerCxLikeCpp",
    "InstanceLockResponseOutcomeLikeCpp",
    "InstanceRaidInfoHandlerCxLikeCpp",
    "InstanceResetMethodLikeCpp",
    "InstancesHandlerHostLikeCpp",
];
const INSTANCES_REGISTRATION_EXPORTS: &[&str] = &["register_instance_handlers_like_cpp"];
const INSTANCES_FACADES: &[RegistrarFacadeContract] = &[
    RegistrarFacadeContract {
        module: "crate",
        child: "instances",
        exports: INSTANCES_ROOT_EXPORTS,
    },
    RegistrarFacadeContract {
        module: "crate::instances",
        child: "registration",
        exports: INSTANCES_REGISTRATION_EXPORTS,
    },
];

pub(crate) const INSTANCES_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationInstances",
    package: "wow-world-application",
    module: "crate::instances::registration",
    registrar: "register_instance_handlers_like_cpp",
    host_trait: "InstancesHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: INSTANCES_FACADES,
};

const EQUIPMENT_SET_USE_ROOT_EXPORTS: &[&str] = &[
    "register_equipment_set_use_handler_like_cpp",
    "EquipmentSetUseContextLikeCpp",
    "EquipmentSetUseHandlerHostLikeCpp",
    "EquipmentSetUseItemModsStoresLikeCpp",
];
const EQUIPMENT_SET_USE_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "equipment_set_use",
    exports: EQUIPMENT_SET_USE_ROOT_EXPORTS,
}];

pub(crate) const EQUIPMENT_SET_USE_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationEquipmentSetUse",
    package: "wow-world-application",
    module: "crate::equipment_set_use",
    registrar: "register_equipment_set_use_handler_like_cpp",
    host_trait: "EquipmentSetUseHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: EQUIPMENT_SET_USE_FACADES,
};

const BANK_ROOT_EXPORTS: &[&str] = &[
    "BankSlotFlagApplicationCxLikeCpp",
    "BankHandlerHostLikeCpp",
    "can_use_current_bank_with_access_like_cpp",
    "register_bank_handlers_like_cpp",
];
const BANK_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "bank",
    exports: BANK_ROOT_EXPORTS,
}];

pub(crate) const BANK_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationBank",
    package: "wow-world-application",
    module: "crate::bank",
    registrar: "register_bank_handlers_like_cpp",
    host_trait: "BankHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: BANK_FACADES,
};

const SOCIAL_INSPECT_ROOT_EXPORTS: &[&str] = &[
    "InspectHandlerCxLikeCpp",
    "SocialInspectHandlerHostLikeCpp",
    "register_social_inspect_handlers_like_cpp",
];
const SOCIAL_INSPECT_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "handlers",
    exports: SOCIAL_INSPECT_ROOT_EXPORTS,
}];

pub(crate) const SOCIAL_INSPECT_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "SocialInspect",
    package: "wow-world-social",
    module: "crate::handlers",
    registrar: "register_social_inspect_handlers_like_cpp",
    host_trait: "SocialInspectHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: SOCIAL_INSPECT_FACADES,
};

const ACCOUNT_DATA_ROOT_EXPORTS: &[&str] = &[
    "AccountDataHandlerCxLikeCpp",
    "AccountDataHandlerHostLikeCpp",
    "register_account_data_handlers_like_cpp",
];
const ACCOUNT_DATA_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "handlers",
    exports: ACCOUNT_DATA_ROOT_EXPORTS,
}];

pub(crate) const ACCOUNT_DATA_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "LifecycleAccountData",
    package: "wow-world-lifecycle",
    module: "crate::handlers",
    registrar: "register_account_data_handlers_like_cpp",
    host_trait: "AccountDataHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: ACCOUNT_DATA_FACADES,
};

const REPUTATION_ROOT_EXPORTS: &[&str] = &[
    "ReputationHandlerCxLikeCpp",
    "ReputationHandlerHostLikeCpp",
    "register_reputation_handlers_like_cpp",
];
const REPUTATION_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "reputation",
    exports: REPUTATION_ROOT_EXPORTS,
}];

pub(crate) const REPUTATION_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationReputation",
    package: "wow-world-application",
    module: "crate::reputation",
    registrar: "register_reputation_handlers_like_cpp",
    host_trait: "ReputationHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: REPUTATION_FACADES,
};

const SUPPORT_ROOT_EXPORTS: &[&str] = &[
    "SupportHandlerCxLikeCpp",
    "SupportHandlerHostLikeCpp",
    "register_support_handlers_like_cpp",
];
const SUPPORT_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "support",
    exports: SUPPORT_ROOT_EXPORTS,
}];

pub(crate) const SUPPORT_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "LifecycleSupport",
    package: "wow-world-lifecycle",
    module: "crate::support",
    registrar: "register_support_handlers_like_cpp",
    host_trait: "SupportHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: SUPPORT_FACADES,
};

const CLIENT_STATE_ROOT_EXPORTS: &[&str] = &[
    "ClientStateHandlerCxLikeCpp",
    "ClientStateHandlerHostLikeCpp",
    "register_client_state_handlers_like_cpp",
];
const CLIENT_STATE_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "client_state",
    exports: CLIENT_STATE_ROOT_EXPORTS,
}];

pub(crate) const CLIENT_STATE_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationClientState",
    package: "wow-world-application",
    module: "crate::client_state",
    registrar: "register_client_state_handlers_like_cpp",
    host_trait: "ClientStateHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: CLIENT_STATE_FACADES,
};

const CALENDAR_ROOT_EXPORTS: &[&str] = &[
    "CalendarHandlerCxLikeCpp",
    "CalendarHandlerHostLikeCpp",
    "register_calendar_handlers_like_cpp",
];
const CALENDAR_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "calendar_handlers",
    exports: CALENDAR_ROOT_EXPORTS,
}];

pub(crate) const CALENDAR_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "SocialCalendar",
    package: "wow-world-social",
    module: "crate::calendar_handlers",
    registrar: "register_calendar_handlers_like_cpp",
    host_trait: "CalendarHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: CALENDAR_FACADES,
};

const CHAT_ROOT_EXPORTS: &[&str] = &[
    "ChatHandlerCxLikeCpp",
    "ChatHandlerHostLikeCpp",
    "register_chat_handlers_like_cpp",
];
const CHAT_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "chat_handlers",
    exports: CHAT_ROOT_EXPORTS,
}];

pub(crate) const CHAT_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "SocialChat",
    package: "wow-world-social",
    module: "crate::chat_handlers",
    registrar: "register_chat_handlers_like_cpp",
    host_trait: "ChatHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: CHAT_FACADES,
};

const SOCIAL_CONTACTS_ROOT_EXPORTS: &[&str] = &[
    "SocialContactsHandlerCxLikeCpp",
    "SocialContactsHandlerHostLikeCpp",
    "register_social_contacts_handlers_like_cpp",
];
const SOCIAL_CONTACTS_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "social_contacts_handlers",
    exports: SOCIAL_CONTACTS_ROOT_EXPORTS,
}];

pub(crate) const SOCIAL_CONTACTS_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "SocialContacts",
    package: "wow-world-social",
    module: "crate::social_contacts_handlers",
    registrar: "register_social_contacts_handlers_like_cpp",
    host_trait: "SocialContactsHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: SOCIAL_CONTACTS_FACADES,
};

const ARENA_TEAM_ROOT_EXPORTS: &[&str] = &[
    "ArenaTeamHandlerCxLikeCpp",
    "ArenaTeamHandlerHostLikeCpp",
    "register_arena_team_handlers_like_cpp",
];
const ARENA_TEAM_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "arena_team_handlers",
    exports: ARENA_TEAM_ROOT_EXPORTS,
}];

pub(crate) const ARENA_TEAM_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "SocialArenaTeam",
    package: "wow-world-social",
    module: "crate::arena_team_handlers",
    registrar: "register_arena_team_handlers_like_cpp",
    host_trait: "ArenaTeamHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: ARENA_TEAM_FACADES,
};

const BATTLENET_ROOT_EXPORTS: &[&str] = &[
    "BattlenetHandlerCxLikeCpp",
    "BattlenetHandlerHostLikeCpp",
    "register_battlenet_handlers_like_cpp",
];
const BATTLENET_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "battlenet_handlers",
    exports: BATTLENET_ROOT_EXPORTS,
}];

pub(crate) const BATTLENET_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "LifecycleBattlenet",
    package: "wow-world-lifecycle",
    module: "crate::battlenet_handlers",
    registrar: "register_battlenet_handlers_like_cpp",
    host_trait: "BattlenetHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: BATTLENET_FACADES,
};

const DATA_SERVICE_ROOT_EXPORTS: &[&str] = &[
    "DataServiceHandlerCxLikeCpp",
    "DataServiceHandlerHostLikeCpp",
    "register_data_service_handlers_like_cpp",
];
const DATA_SERVICE_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "data_service_handlers",
    exports: DATA_SERVICE_ROOT_EXPORTS,
}];

pub(crate) const DATA_SERVICE_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationDataService",
    package: "wow-world-application",
    module: "crate::data_service_handlers",
    registrar: "register_data_service_handlers_like_cpp",
    host_trait: "DataServiceHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: DATA_SERVICE_FACADES,
};

const SOCIAL_GROUP_ROOT_EXPORTS: &[&str] = &[
    "SocialGroupHandlerCxLikeCpp",
    "SocialGroupHandlerHostLikeCpp",
    "register_social_group_handlers_like_cpp",
];
const SOCIAL_GROUP_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "group_handlers",
    exports: SOCIAL_GROUP_ROOT_EXPORTS,
}];

pub(crate) const SOCIAL_GROUP_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "SocialGroup",
    package: "wow-world-social",
    module: "crate::group_handlers",
    registrar: "register_social_group_handlers_like_cpp",
    host_trait: "SocialGroupHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: SOCIAL_GROUP_FACADES,
};

const APPLICATION_GROUP_ROOT_EXPORTS: &[&str] = &[
    "GroupHandlerCxLikeCpp",
    "GroupHandlerHostLikeCpp",
    "register_group_handlers_like_cpp",
];
const APPLICATION_GROUP_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "group_handlers",
    exports: APPLICATION_GROUP_ROOT_EXPORTS,
}];

pub(crate) const APPLICATION_GROUP_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationGroup",
    package: "wow-world-application",
    module: "crate::group_handlers",
    registrar: "register_group_handlers_like_cpp",
    host_trait: "GroupHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: APPLICATION_GROUP_FACADES,
};

const GUILD_ROOT_EXPORTS: &[&str] = &[
    "GuildHandlerCxLikeCpp",
    "GuildHandlerHostLikeCpp",
    "register_guild_handlers_like_cpp",
];
const GUILD_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "guild_handlers",
    exports: GUILD_ROOT_EXPORTS,
}];

pub(crate) const GUILD_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "SocialGuild",
    package: "wow-world-social",
    module: "crate::guild_handlers",
    registrar: "register_guild_handlers_like_cpp",
    host_trait: "GuildHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: GUILD_FACADES,
};

const QUEST_QUERY_ROOT_EXPORTS: &[&str] = &[
    "QuestQueryHandlerCxLikeCpp",
    "QuestQueryHandlerHostLikeCpp",
    "register_quest_query_handlers_like_cpp",
];
const QUEST_QUERY_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "quest_query_handlers",
    exports: QUEST_QUERY_ROOT_EXPORTS,
}];

pub(crate) const QUEST_QUERY_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationQuestQuery",
    package: "wow-world-application",
    module: "crate::quest_query_handlers",
    registrar: "register_quest_query_handlers_like_cpp",
    host_trait: "QuestQueryHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: QUEST_QUERY_FACADES,
};

const COMBAT_ROOT_EXPORTS: &[&str] = &[
    "CombatHandlerCxLikeCpp",
    "CombatHandlerHostLikeCpp",
    "register_combat_handlers_like_cpp",
];
const COMBAT_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "combat_handlers",
    exports: COMBAT_ROOT_EXPORTS,
}];

pub(crate) const COMBAT_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationCombat",
    package: "wow-world-application",
    module: "crate::combat_handlers",
    registrar: "register_combat_handlers_like_cpp",
    host_trait: "CombatHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: COMBAT_FACADES,
};

const PLAYER_ROOT_EXPORTS: &[&str] = &[
    "PlayerHandlerCxLikeCpp",
    "PlayerHandlerHostLikeCpp",
    "register_player_handlers_like_cpp",
];
const PLAYER_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "player_handlers",
    exports: PLAYER_ROOT_EXPORTS,
}];

pub(crate) const PLAYER_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationPlayer",
    package: "wow-world-application",
    module: "crate::player_handlers",
    registrar: "register_player_handlers_like_cpp",
    host_trait: "PlayerHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: PLAYER_FACADES,
};

const COLLECTIONS_ROOT_EXPORTS: &[&str] = &[
    "CollectionsHandlerCxLikeCpp",
    "CollectionsHandlerHostLikeCpp",
    "register_collections_handlers_like_cpp",
];
const COLLECTIONS_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "collections_handlers",
    exports: COLLECTIONS_ROOT_EXPORTS,
}];

pub(crate) const COLLECTIONS_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationCollections",
    package: "wow-world-application",
    module: "crate::collections_handlers",
    registrar: "register_collections_handlers_like_cpp",
    host_trait: "CollectionsHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: COLLECTIONS_FACADES,
};

const TRAVEL_ROOT_EXPORTS: &[&str] = &[
    "TravelHandlerCxLikeCpp",
    "TravelHandlerHostLikeCpp",
    "register_travel_handlers_like_cpp",
];
const TRAVEL_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "travel_handlers",
    exports: TRAVEL_ROOT_EXPORTS,
}];

pub(crate) const TRAVEL_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationTravel",
    package: "wow-world-application",
    module: "crate::travel_handlers",
    registrar: "register_travel_handlers_like_cpp",
    host_trait: "TravelHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: TRAVEL_FACADES,
};

const BATTLEGROUND_ROOT_EXPORTS: &[&str] = &[
    "BattlegroundHandlerCxLikeCpp",
    "BattlegroundHandlerHostLikeCpp",
    "register_battleground_handlers_like_cpp",
];
const BATTLEGROUND_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "battleground_handlers",
    exports: BATTLEGROUND_ROOT_EXPORTS,
}];

pub(crate) const BATTLEGROUND_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationBattleground",
    package: "wow-world-application",
    module: "crate::battleground_handlers",
    registrar: "register_battleground_handlers_like_cpp",
    host_trait: "BattlegroundHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: BATTLEGROUND_FACADES,
};

const DUNGEON_FINDING_ROOT_EXPORTS: &[&str] = &[
    "DungeonFindingHandlerCxLikeCpp",
    "DungeonFindingHandlerHostLikeCpp",
    "register_dungeon_finding_handlers_like_cpp",
];
const DUNGEON_FINDING_FACADES: &[RegistrarFacadeContract] = &[RegistrarFacadeContract {
    module: "crate",
    child: "dungeon_finding_handlers",
    exports: DUNGEON_FINDING_ROOT_EXPORTS,
}];

pub(crate) const DUNGEON_FINDING_REGISTRAR: DirectRegistrarContract = DirectRegistrarContract {
    owner: "ApplicationDungeonFinding",
    package: "wow-world-application",
    module: "crate::dungeon_finding_handlers",
    registrar: "register_dungeon_finding_handlers_like_cpp",
    host_trait: "DungeonFindingHandlerHostLikeCpp",
    production_type_args: &["WorldSession", "SessionHandlerCatalogsLikeCpp"],
    facades: DUNGEON_FINDING_FACADES,
};

/// Exact direct registrars which exist in the current source tree.
pub(crate) const DIRECT_REGISTRAR_CONTRACTS: &[DirectRegistrarContract] = &[
    INVENTORY_REGISTRAR,
    INSTANCES_REGISTRAR,
    EQUIPMENT_SET_USE_REGISTRAR,
    BANK_REGISTRAR,
    SOCIAL_INSPECT_REGISTRAR,
    ACCOUNT_DATA_REGISTRAR,
    REPUTATION_REGISTRAR,
    SUPPORT_REGISTRAR,
    CLIENT_STATE_REGISTRAR,
    CALENDAR_REGISTRAR,
    CHAT_REGISTRAR,
    SOCIAL_CONTACTS_REGISTRAR,
    ARENA_TEAM_REGISTRAR,
    BATTLENET_REGISTRAR,
    DATA_SERVICE_REGISTRAR,
    SOCIAL_GROUP_REGISTRAR,
    APPLICATION_GROUP_REGISTRAR,
    GUILD_REGISTRAR,
    QUEST_QUERY_REGISTRAR,
    COMBAT_REGISTRAR,
    PLAYER_REGISTRAR,
    COLLECTIONS_REGISTRAR,
    TRAVEL_REGISTRAR,
    BATTLEGROUND_REGISTRAR,
    DUNGEON_FINDING_REGISTRAR,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RegistrarReport {
    pub(crate) entries: usize,
    pub(crate) registrar_count: usize,
    pub(crate) contract: Option<DirectRegistrarContract>,
}

#[derive(Default)]
struct ImportBindings {
    packet_handler_entry: usize,
    registry_builder: usize,
    duplicate_error: usize,
    errors: Vec<String>,
}

fn collect_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    leaves: &mut Vec<(Vec<String>, String, bool)>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree(&path.tree, prefix, leaves);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            leaves.push((path, name.ident.to_string(), false));
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            path.push(rename.ident.to_string());
            leaves.push((path, rename.rename.to_string(), true));
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, leaves);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn token_stream_mentions_ident(tokens: &TokenStream, expected: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident_is(&ident, expected),
        TokenTree::Group(group) => token_stream_mentions_ident(&group.stream(), expected),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn inspect_imports(items: &[Item]) -> Result<ImportBindings, String> {
    let mut bindings = ImportBindings::default();
    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        let mut leaves = Vec::new();
        collect_use_tree(&item_use.tree, &mut Vec::new(), &mut leaves);
        for (path, local, renamed) in leaves {
            let Some(provider_type) = path.last() else {
                continue;
            };
            let binding = match provider_type.as_str() {
                "PacketHandlerEntry" => {
                    Some(("PacketHandlerEntry", &mut bindings.packet_handler_entry))
                }
                "RegistryBuilder" => Some(("RegistryBuilder", &mut bindings.registry_builder)),
                "DuplicateHandlerRegistrationLikeCpp" => Some((
                    "DuplicateHandlerRegistrationLikeCpp",
                    &mut bindings.duplicate_error,
                )),
                _ => None,
            };
            let Some((expected, count)) = binding else {
                continue;
            };
            *count += 1;
            let canonical = path.len() == 2
                && path[0] == "wow_handler"
                && path[1] == expected
                && local == expected
                && !renamed
                && matches!(&item_use.vis, Visibility::Inherited)
                && item_use.attrs.is_empty();
            if !canonical {
                bindings.errors.push(format!(
                    "direct handler registration must import {expected} unrenamed from wow_handler"
                ));
            }
        }
    }
    for (name, count) in [
        ("PacketHandlerEntry", bindings.packet_handler_entry),
        ("RegistryBuilder", bindings.registry_builder),
        (
            "DuplicateHandlerRegistrationLikeCpp",
            bindings.duplicate_error,
        ),
    ] {
        if count != 1 {
            bindings.errors.push(format!(
                "direct handler registrar requires exactly one canonical wow_handler::{name} import; found {count}"
            ));
        }
    }
    if bindings.errors.is_empty() {
        Ok(bindings)
    } else {
        Err(bindings.errors.join("; "))
    }
}

struct EntryAndRegisterVisitor {
    entry_names: BTreeSet<String>,
    entry_literals: usize,
    register_calls: usize,
    all_register_calls: usize,
    cfg: Vec<String>,
}

impl EntryAndRegisterVisitor {
    fn new(entry_names: BTreeSet<String>) -> Self {
        Self {
            entry_names,
            entry_literals: 0,
            register_calls: 0,
            all_register_calls: 0,
            cfg: Vec::new(),
        }
    }
}

struct EntryLiteralFinder<'a> {
    entry_names: &'a BTreeSet<String>,
    found: bool,
}

impl<'ast> Visit<'ast> for EntryLiteralFinder<'_> {
    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if expression
            .path
            .segments
            .last()
            .is_some_and(|segment| self.entry_names.contains(&segment.ident.to_string()))
        {
            self.found = true;
        }
        syn::visit::visit_expr_struct(self, expression);
    }
}

/// Whether an item can exist in a production build.
///
/// A `#[path]` child arrives spliced inside its parent and keeps its own cfg; a
/// test-only module cannot register a production handler, so its entries are
/// fixture data and stay outside the registrar grammar.
pub(crate) fn attributes_are_production(attrs: &[syn::Attribute], parent_cfg: &[String]) -> bool {
    crate::ownership::cfg_context_allows_production(parent_cfg, attrs).unwrap_or(true)
}

fn item_attributes(item: &Item) -> &[syn::Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

impl<'ast> Visit<'ast> for EntryAndRegisterVisitor {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !attributes_are_production(&item.attrs, &self.cfg) {
            return;
        }
        let previous = self.cfg.len();
        self.cfg = crate::ownership::extend_cfg_context(&self.cfg, &item.attrs);
        syn::visit::visit_item_mod(self, item);
        self.cfg.truncate(previous);
    }

    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if expression
            .path
            .segments
            .last()
            .is_some_and(|segment| self.entry_names.contains(&segment.ident.to_string()))
        {
            self.entry_literals += 1;
        }
        syn::visit::visit_expr_struct(self, expression);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if ident_is(&call.method, "register") {
            self.all_register_calls += 1;
        }
        let mut argument_contains_entry = EntryLiteralFinder {
            entry_names: &self.entry_names,
            found: false,
        };
        for argument in &call.args {
            argument_contains_entry.visit_expr(argument);
        }
        if ident_is(&call.method, "register") && argument_contains_entry.found {
            self.register_calls += 1;
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

fn entry_type_bindings(items: &[Item]) -> BTreeSet<String> {
    let mut names = BTreeSet::from(["PacketHandlerEntry".to_owned()]);
    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        let mut leaves = Vec::new();
        collect_use_tree(&item_use.tree, &mut Vec::new(), &mut leaves);
        for (path, local, _) in leaves {
            if path.last().is_some_and(|name| name == "PacketHandlerEntry") {
                names.insert(local);
            }
        }
    }

    loop {
        let mut added = false;
        for item in items {
            let Item::Type(alias) = item else {
                continue;
            };
            struct ReferencedEntry<'a> {
                names: &'a BTreeSet<String>,
                found: bool,
            }
            impl<'ast> Visit<'ast> for ReferencedEntry<'_> {
                fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
                    if path
                        .path
                        .segments
                        .last()
                        .is_some_and(|segment| self.names.contains(&segment.ident.to_string()))
                    {
                        self.found = true;
                    }
                    syn::visit::visit_type_path(self, path);
                }
            }
            let mut visitor = ReferencedEntry {
                names: &names,
                found: false,
            };
            visitor.visit_type(&alias.ty);
            if visitor.found {
                added |= names.insert(alias.ident.to_string());
            }
        }
        if !added {
            break;
        }
    }
    names
}

fn path_has_only_segment(path: &syn::Path, expected: &str) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path
            .segments
            .first()
            .is_some_and(|segment| ident_is(&segment.ident, expected))
}

fn type_arguments(ty: &Type) -> Option<Vec<&syn::GenericArgument>> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    Some(arguments.args.iter().collect())
}

fn is_builder_parameter(argument: &FnArg) -> bool {
    let FnArg::Typed(argument) = argument else {
        return false;
    };
    if !argument.attrs.is_empty() {
        return false;
    }
    let Pat::Ident(pattern) = &*argument.pat else {
        return false;
    };
    if !pattern.attrs.is_empty()
        || !ident_is(&pattern.ident, "builder")
        || pattern.by_ref.is_some()
        || pattern.subpat.is_some()
    {
        return false;
    }
    let Type::Reference(reference) = &*argument.ty else {
        return false;
    };
    if reference.mutability.is_none() || reference.lifetime.is_some() {
        return false;
    }
    let Type::Path(builder_path) = &*reference.elem else {
        return false;
    };
    if builder_path.qself.is_some() || !path_has_only_segment(&builder_path.path, "RegistryBuilder")
    {
        return false;
    }
    let Some(arguments) = type_arguments(&reference.elem) else {
        return false;
    };
    arguments.len() == 2
        && matches!(&arguments[0], syn::GenericArgument::Type(Type::Path(path)) if path.qself.is_none() && path.path.is_ident("S"))
        && matches!(&arguments[1], syn::GenericArgument::Type(Type::Path(path)) if path.qself.is_none() && path.path.is_ident("C"))
}

fn registrar_doc_attributes_only(attributes: &[Attribute]) -> bool {
    attributes.iter().all(|attribute| {
        attribute.path().is_ident("doc")
            && matches!(
                &attribute.meta,
                Meta::NameValue(name_value)
                    if name_value.path.is_ident("doc")
                        && matches!(&name_value.value, Expr::Lit(value) if matches!(&value.lit, Lit::Str(_)))
            )
    })
}

fn is_registrar_signature(function: &ItemFn, contract: DirectRegistrarContract) -> bool {
    registrar_doc_attributes_only(&function.attrs)
        && matches!(&function.vis, Visibility::Public(_))
        && ident_is(&function.sig.ident, contract.registrar)
        && function.sig.constness.is_none()
        && function.sig.asyncness.is_none()
        && function.sig.unsafety.is_none()
        && function.sig.abi.is_none()
        && function.sig.variadic.is_none()
        && function.sig.generics.params.len() == 2
        && function
            .sig
            .generics
            .params
            .iter()
            .enumerate()
            .all(|(index, parameter)| {
                matches!(parameter, syn::GenericParam::Type(parameter)
                if parameter.attrs.is_empty()
                    && parameter.bounds.is_empty()
                    && parameter.default.is_none()
                    && ident_is(&parameter.ident, if index == 0 { "S" } else { "C" }))
            })
        && function.sig.generics.where_clause.is_some()
        && is_host_where_clause(
            function.sig.generics.where_clause.as_ref(),
            contract.host_trait,
        )
        && function.sig.inputs.len() == 1
        && is_builder_parameter(&function.sig.inputs[0])
        && is_registrar_output(&function.sig.output)
}

fn is_plain_type_path(ty: &Type, expected: &str) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none() && path_has_only_segment(&path.path, expected)
}

fn where_type_name(predicate: &syn::PredicateType, expected: &str) -> bool {
    predicate.lifetimes.is_none() && is_plain_type_path(&predicate.bounded_ty, expected)
}

fn trait_bound_name(bound: &TypeParamBound, expected: &str) -> bool {
    matches!(bound, TypeParamBound::Trait(bound)
        if bound.paren_token.is_none()
            && matches!(&bound.modifier, syn::TraitBoundModifier::None)
            && bound.lifetimes.is_none()
            && bound.path.leading_colon.is_none()
            && bound.path.segments.len() == 1
            && bound.path.segments.first().is_some_and(|segment| ident_is(&segment.ident, expected)))
}

fn is_host_trait_bound(bound: &TypeParamBound, host_trait: &str) -> bool {
    let TypeParamBound::Trait(bound) = bound else {
        return false;
    };
    if bound.paren_token.is_some()
        || !matches!(&bound.modifier, syn::TraitBoundModifier::None)
        || bound.lifetimes.is_some()
        || bound.path.leading_colon.is_some()
        || bound.path.segments.len() != 1
    {
        return false;
    }
    let segment = &bound.path.segments[0];
    if !ident_is(&segment.ident, host_trait) {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    arguments.args.len() == 1
        && matches!(arguments.args.first(), Some(syn::GenericArgument::Type(Type::Path(path)))
            if path.qself.is_none() && path.path.is_ident("C"))
}

fn is_host_where_clause(clause: Option<&syn::WhereClause>, host_trait: &str) -> bool {
    let Some(clause) = clause else {
        return false;
    };
    let predicates: Vec<_> = clause.predicates.iter().collect();
    if predicates.len() != 2 {
        return false;
    }
    let WherePredicate::Type(session) = predicates[0] else {
        return false;
    };
    let WherePredicate::Type(catalogs) = predicates[1] else {
        return false;
    };
    where_type_name(session, "S")
        && session.bounds.len() == 2
        && session
            .bounds
            .first()
            .is_some_and(|bound| is_host_trait_bound(bound, host_trait))
        && session
            .bounds
            .iter()
            .nth(1)
            .is_some_and(|bound| trait_bound_name(bound, "Send"))
        && where_type_name(catalogs, "C")
        && catalogs.bounds.len() == 1
        && catalogs
            .bounds
            .iter()
            .all(|bound| trait_bound_name(bound, "Sync"))
}

fn is_registrar_output(output: &syn::ReturnType) -> bool {
    let syn::ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(result) = &**ty else {
        return false;
    };
    if result.qself.is_some() || !path_has_only_segment(&result.path, "Result") {
        return false;
    }
    let Some(arguments) = type_arguments(ty) else {
        return false;
    };
    arguments.len() == 2
        && matches!(&arguments[0], syn::GenericArgument::Type(Type::Tuple(tuple)) if tuple.elems.is_empty())
        && matches!(&arguments[1], syn::GenericArgument::Type(Type::Path(path))
            if path.qself.is_none()
                && path.path.leading_colon.is_none()
                && path.path.is_ident("DuplicateHandlerRegistrationLikeCpp"))
}

fn is_builder_receiver(expression: &Expr) -> bool {
    matches!(expression, Expr::Path(path)
        if path.attrs.is_empty()
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
            && path.path.segments.first().is_some_and(|segment| ident_is(&segment.ident, "builder")))
}

fn is_entry_literal(expression: &Expr) -> bool {
    let Expr::Struct(entry) = expression else {
        return false;
    };
    let fields = ["opcode", "status", "processing", "handler_name", "handler"];
    entry.attrs.is_empty()
        && entry.qself.is_none()
        && entry.path.leading_colon.is_none()
        && entry.path.segments.len() == 1
        && entry.path.segments.first().is_some_and(|segment| {
            ident_is(&segment.ident, "PacketHandlerEntry")
                && matches!(&segment.arguments, syn::PathArguments::None)
        })
        && entry.rest.is_none()
        && entry.fields.len() == fields.len()
        && entry.fields.iter().zip(fields).all(|(field, expected)| {
            matches!(&field.member, syn::Member::Named(name) if ident_is(name, expected))
                && field.attrs.is_empty()
        })
}

fn registration_entry(statement: &Stmt) -> Option<&syn::ExprStruct> {
    let Stmt::Expr(expression, Some(_)) = statement else {
        return None;
    };
    let Expr::Try(try_expression) = expression else {
        return None;
    };
    if !try_expression.attrs.is_empty() {
        return None;
    }
    let Expr::MethodCall(call) = &*try_expression.expr else {
        return None;
    };
    if !ident_is(&call.method, "register")
        || call.turbofish.is_some()
        || !call.attrs.is_empty()
        || !is_builder_receiver(&call.receiver)
        || call.args.len() != 1
        || !is_entry_literal(&call.args[0])
    {
        return None;
    }
    let Expr::Struct(entry) = &call.args[0] else {
        return None;
    };
    Some(entry)
}

fn is_ok_unit_tail(statement: &Stmt) -> bool {
    let Stmt::Expr(expression, None) = statement else {
        return false;
    };
    matches!(expression, Expr::Call(call)
        if call.attrs.is_empty()
            && matches!(&*call.func, Expr::Path(path)
            if path.qself.is_none()
                && path.path.leading_colon.is_none()
                && path.path.is_ident("Ok")
                && path.attrs.is_empty())
            && call.args.len() == 1
            && matches!(call.args.first(), Some(Expr::Tuple(tuple)) if tuple.elems.is_empty() && tuple.attrs.is_empty()))
}

fn opcode_key(entry: &syn::ExprStruct) -> Option<String> {
    entry.fields.iter().find_map(|field| {
        if !matches!(&field.member, syn::Member::Named(name) if ident_is(name, "opcode")) {
            return None;
        }
        let Expr::Path(path) = &field.expr else {
            return None;
        };
        (path.attrs.is_empty()
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 2
            && path
                .path
                .segments
                .first()
                .is_some_and(|segment| ident_is(&segment.ident, "ClientOpcodes")))
        .then(|| path.to_token_stream().to_string())
    })
}

fn analyze_registrar(
    function: &ItemFn,
    contract: DirectRegistrarContract,
) -> Result<usize, String> {
    if !is_registrar_signature(function, contract) {
        return Err(format!(
            "{} handler registrar has an unexpected signature or attributes",
            contract.owner
        ));
    }
    let Some((tail, registrations)) = function.block.stmts.split_last() else {
        return Err(format!("{} handler registrar is empty", contract.owner));
    };
    if !is_ok_unit_tail(tail) {
        return Err(format!(
            "{} handler registrar must end with Ok(())",
            contract.owner
        ));
    }
    let mut opcodes = BTreeSet::new();
    for statement in registrations {
        let Some(entry) = registration_entry(statement) else {
            return Err(format!(
                "{} handler registrar permits only direct builder.register(PacketHandlerEntry {{ ... }})? statements before Ok(())",
                contract.owner
            ));
        };
        let opcode = opcode_key(entry).ok_or_else(|| {
            format!(
                "{} PacketHandlerEntry opcode must be a ClientOpcodes path",
                contract.owner
            )
        })?;
        if !opcodes.insert(opcode.clone()) {
            return Err(format!(
                "duplicate {} handler opcode entry {opcode}",
                contract.owner
            ));
        }
    }
    if opcodes.is_empty() {
        return Err(format!(
            "{} handler registrar contains no direct entries",
            contract.owner
        ));
    }
    Ok(opcodes.len())
}

/// Analyze a registrar only under its exact finite owner contract.
pub(crate) fn analyze_contract_source(
    contract: DirectRegistrarContract,
    package: &str,
    logical_module: &str,
    source_path: &Path,
    source: &str,
) -> Result<RegistrarReport, String> {
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("cannot parse {}: {error}", source_path.display()))?;
    let registrar_items: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if ident_is(&function.sig.ident, contract.registrar) => {
                Some(function)
            }
            _ => None,
        })
        .collect();
    let entry_names = entry_type_bindings(&syntax.items);
    let mut occurrences = EntryAndRegisterVisitor::new(entry_names);
    occurrences.visit_file(&syntax);
    if registrar_items.is_empty() {
        if occurrences.entry_literals != 0 || occurrences.register_calls != 0 {
            return Err(format!(
                "direct PacketHandlerEntry or builder.register source is outside the exact {} registrar in {} ({logical_module})",
                contract.owner,
                source_path.display()
            ));
        }
        return Ok(RegistrarReport::default());
    }
    if syntax
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
    {
        return Err(format!(
            "{} direct registrar source file must not be conditionally compiled",
            contract.owner
        ));
    }
    if package != contract.package || logical_module != contract.module {
        return Err(format!(
            "direct {} handler registrar is outside {}::{}: {} ({logical_module})",
            contract.owner,
            contract.package,
            contract.module,
            source_path.display()
        ));
    }
    if registrar_items.len() != 1 {
        return Err(format!(
            "{} handler registrar must be defined exactly once",
            contract.owner
        ));
    }
    inspect_imports(&syntax.items)?;
    for item in &syntax.items {
        if matches!(item, Item::Use(_))
            || matches!(item, Item::Fn(function) if ident_is(&function.sig.ident, contract.registrar))
            || !attributes_are_production(item_attributes(item), &[])
        {
            continue;
        }
        let tokens = item.to_token_stream();
        if token_stream_mentions_ident(&tokens, "PacketHandlerEntry")
            || token_stream_mentions_ident(&tokens, "RegistryBuilder")
        {
            return Err(format!(
                "{} handler entry types may appear only in the exact top-level registrar",
                contract.owner
            ));
        }
    }
    let entries = analyze_registrar(registrar_items[0], contract)?;
    if occurrences.entry_literals != entries
        || occurrences.register_calls != entries
        || occurrences.all_register_calls != entries
    {
        return Err(format!(
            "{} source contains {} PacketHandlerEntry literals, {} entry-bearing register calls, and {} total register calls, but its exact registrar accounts for {entries}",
            contract.owner,
            occurrences.entry_literals,
            occurrences.register_calls,
            occurrences.all_register_calls
        ));
    }
    Ok(RegistrarReport {
        entries,
        registrar_count: 1,
        contract: Some(contract),
    })
}

/// Compatibility entry point for existing Inventory callers/tests.
pub(crate) fn analyze_owner_source(
    package: &str,
    logical_module: &str,
    source_path: &Path,
    source: &str,
) -> Result<RegistrarReport, String> {
    analyze_contract_source(
        INVENTORY_REGISTRAR,
        package,
        logical_module,
        source_path,
        source,
    )
}

pub(crate) fn analyze_owner_source_with_contracts(
    package: &str,
    logical_module: &str,
    source_path: &Path,
    source: &str,
    contracts: &[DirectRegistrarContract],
) -> Result<RegistrarReport, String> {
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("cannot parse {}: {error}", source_path.display()))?;
    let declared: Vec<_> = contracts
        .iter()
        .filter(|contract| {
            syntax.items.iter().any(|item| {
                matches!(item, Item::Fn(function)
                    if ident_is(&function.sig.ident, contract.registrar))
            })
        })
        .copied()
        .collect();
    if declared.len() > 1 {
        return Err(format!(
            "{} declares more than one finite direct registrar in the same source module",
            source_path.display()
        ));
    }
    if let Some(contract) = declared.first() {
        return analyze_contract_source(*contract, package, logical_module, source_path, source);
    }
    if let Some(violation) = unowned_entry_literal_violation(source)? {
        return Err(format!("{}: {violation}", source_path.display()));
    }
    Ok(RegistrarReport::default())
}

/// Reject a direct generic handler entry constructed outside an authorized registrar.
pub(crate) fn unowned_entry_literal_violation(source: &str) -> Result<Option<String>, String> {
    let syntax =
        syn::parse_file(source).map_err(|error| format!("cannot parse source: {error}"))?;
    let entry_names = entry_type_bindings(&syntax.items);
    let mut occurrences = EntryAndRegisterVisitor::new(entry_names);
    occurrences.visit_file(&syntax);
    if occurrences.entry_literals == 0 && occurrences.register_calls == 0 {
        return Ok(None);
    }
    Ok(Some(format!(
        "{} PacketHandlerEntry struct literal(s) and {} generic register call(s) appear outside an authorized direct registrar",
        occurrences.entry_literals, occurrences.register_calls
    )))
}
