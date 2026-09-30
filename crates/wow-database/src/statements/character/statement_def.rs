//! Character statements keep one exhaustive dispatch owner; SQL values live in private semantic families.
//!
//! Separated from the character.rs root under #652. Behaviour is preserved.

use super::*;

mod character;
mod gm_support;
mod items;
mod pets_pvp;
mod progression;
mod social;
mod world;

impl StatementDef for CharStatements {
    fn database() -> crate::persistence_trace::LogicalDatabase {
        crate::persistence_trace::LogicalDatabase::Character
    }

    fn trace_identity(self) -> String {
        // The default derives identity from `Debug`, which for this data
        // carrying variant would embed the SQL text.
        match self {
            Self::GENERATED_CPP { name, .. } => name.to_owned(),
            other => format!("{other:?}"),
        }
    }

    fn sql(self) -> &'static str {
        match self {
            Self::DEL_POOL_QUEST_SAVE => progression::DEL_POOL_QUEST_SAVE,
            Self::INS_POOL_QUEST_SAVE => progression::INS_POOL_QUEST_SAVE,
            Self::DEL_NONEXISTENT_GUILD_BANK_ITEM => social::DEL_NONEXISTENT_GUILD_BANK_ITEM,
            Self::DEL_EXPIRED_BANS => character::DEL_EXPIRED_BANS,
            Self::SEL_ENUM => character::SEL_ENUM,
            Self::SEL_ENUM_DECLINED_NAME => character::SEL_ENUM_DECLINED_NAME,
            Self::SEL_ENUM_CUSTOMIZATIONS => character::SEL_ENUM_CUSTOMIZATIONS,
            Self::SEL_UNDELETE_ENUM => character::SEL_UNDELETE_ENUM,
            Self::SEL_UNDELETE_ENUM_DECLINED_NAME => character::SEL_UNDELETE_ENUM_DECLINED_NAME,
            Self::SEL_UNDELETE_ENUM_CUSTOMIZATIONS => character::SEL_UNDELETE_ENUM_CUSTOMIZATIONS,
            Self::SEL_CHECK_NAME => character::SEL_CHECK_NAME,
            Self::SEL_RESERVED_NAMES => character::SEL_RESERVED_NAMES,
            Self::SEL_CHECK_GUID => character::SEL_CHECK_GUID,
            Self::SEL_SUM_CHARS => character::SEL_SUM_CHARS,
            Self::SEL_CHAR_CREATE_INFO => character::SEL_CHAR_CREATE_INFO,
            Self::INS_CHARACTER_BAN => character::INS_CHARACTER_BAN,
            Self::UPD_CHARACTER_BAN => character::UPD_CHARACTER_BAN,
            Self::DEL_CHARACTER_BAN => character::DEL_CHARACTER_BAN,
            Self::SEL_BANINFO => character::SEL_BANINFO,
            Self::SEL_GUID_BY_NAME_FILTER => character::SEL_GUID_BY_NAME_FILTER,
            Self::SEL_BANINFO_LIST => character::SEL_BANINFO_LIST,
            Self::SEL_BANNED_NAME => character::SEL_BANNED_NAME,
            Self::SEL_MAIL_LIST_COUNT => items::SEL_MAIL_LIST_COUNT,
            Self::SEL_MAIL_LIST_INFO => items::SEL_MAIL_LIST_INFO,
            Self::SEL_MAIL_LIST_ITEMS => items::SEL_MAIL_LIST_ITEMS,
            Self::SEL_FREE_NAME => character::SEL_FREE_NAME,
            Self::SEL_CHAR_ZONE => world::SEL_CHAR_ZONE,
            Self::SEL_CHAR_POSITION_XYZ => world::SEL_CHAR_POSITION_XYZ,
            Self::SEL_CHAR_POSITION => world::SEL_CHAR_POSITION,
            Self::DEL_BATTLEGROUND_RANDOM_ALL => pets_pvp::DEL_BATTLEGROUND_RANDOM_ALL,
            Self::DEL_BATTLEGROUND_RANDOM => pets_pvp::DEL_BATTLEGROUND_RANDOM,
            Self::INS_BATTLEGROUND_RANDOM => pets_pvp::INS_BATTLEGROUND_RANDOM,
            Self::INS_CHARACTER => character::INS_CHARACTER,
            Self::UPD_CHARACTER => character::UPD_CHARACTER,
            Self::INS_CHAR_CUSTOMIZATION => character::INS_CHAR_CUSTOMIZATION,
            Self::INS_CHARACTER_CUSTOMIZATION => character::INS_CHARACTER_CUSTOMIZATION,
            Self::UPD_ADD_AT_LOGIN_FLAG => character::UPD_ADD_AT_LOGIN_FLAG,
            Self::UPD_REM_AT_LOGIN_FLAG => character::UPD_REM_AT_LOGIN_FLAG,
            Self::UPD_ALL_AT_LOGIN_FLAGS => character::UPD_ALL_AT_LOGIN_FLAGS,
            Self::INS_BUG_REPORT => gm_support::INS_BUG_REPORT,
            Self::UPD_PETITION_NAME => social::UPD_PETITION_NAME,
            Self::INS_PETITION_SIGNATURE => social::INS_PETITION_SIGNATURE,
            Self::UPD_ACCOUNT_ONLINE => character::UPD_ACCOUNT_ONLINE,
            Self::DEL_CHARACTER_CUSTOMIZATIONS => character::DEL_CHARACTER_CUSTOMIZATIONS,
            Self::DEL_CHARACTER => character::DEL_CHARACTER,
            Self::DEL_CHAR_REPUTATION_BY_FACTION => progression::DEL_CHAR_REPUTATION_BY_FACTION,
            Self::INS_CHAR_REPUTATION_BY_FACTION => progression::INS_CHAR_REPUTATION_BY_FACTION,
            Self::DEL_CHAR_REPUTATION => progression::DEL_CHAR_REPUTATION,
            Self::SEL_CHARACTER => character::SEL_CHARACTER,
            Self::SEL_CHARACTER_IDENTITY_CACHE => character::SEL_CHARACTER_IDENTITY_CACHE,
            Self::SEL_CHARACTER_CUSTOMIZATIONS => character::SEL_CHARACTER_CUSTOMIZATIONS,
            Self::SEL_GROUP_MEMBER => social::SEL_GROUP_MEMBER,
            Self::SEL_CHARACTER_AURAS => progression::SEL_CHARACTER_AURAS,
            Self::SEL_CHARACTER_AURA_EFFECTS => progression::SEL_CHARACTER_AURA_EFFECTS,
            Self::UPD_CHAR_ONLINE => character::UPD_CHAR_ONLINE,
            Self::UPD_CHAR_OFFLINE => character::UPD_CHAR_OFFLINE,
            Self::SEL_CHAR_DEL_CHECK => character::SEL_CHAR_DEL_CHECK,
            Self::SEL_MAX_GUID => character::SEL_MAX_GUID,
            Self::SEL_CHAR_EQUIPMENT => items::SEL_CHAR_EQUIPMENT,
            Self::UPD_CHAR_INVENTORY_SLOT => items::UPD_CHAR_INVENTORY_SLOT,
            Self::DEL_CHAR_INVENTORY_ITEM => items::DEL_CHAR_INVENTORY_ITEM,
            Self::DEL_CHAR_INVENTORY_ITEM_BY_OWNER => items::DEL_CHAR_INVENTORY_ITEM_BY_OWNER,
            Self::SEL_CHARACTER_SKILLS => progression::SEL_CHARACTER_SKILLS,
            Self::SEL_CHARACTER_SPELL => progression::SEL_CHARACTER_SPELL,
            Self::SEL_CHARACTER_SPELL_FAVORITES => progression::SEL_CHARACTER_SPELL_FAVORITES,
            Self::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA => {
                progression::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA
            }
            Self::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS => {
                progression::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS
            }
            Self::SEL_CHARACTER_QUESTSTATUS_DAILY => progression::SEL_CHARACTER_QUESTSTATUS_DAILY,
            Self::SEL_CHARACTER_QUESTSTATUS_WEEKLY => progression::SEL_CHARACTER_QUESTSTATUS_WEEKLY,
            Self::SEL_CHARACTER_QUESTSTATUS_MONTHLY => {
                progression::SEL_CHARACTER_QUESTSTATUS_MONTHLY
            }
            Self::SEL_CHARACTER_QUESTSTATUS_SEASONAL => {
                progression::SEL_CHARACTER_QUESTSTATUS_SEASONAL
            }
            Self::SEL_CHARACTER_REPUTATION => progression::SEL_CHARACTER_REPUTATION,
            Self::SEL_MAIL_COUNT => items::SEL_MAIL_COUNT,
            Self::SEL_CHARACTER_SOCIALLIST => social::SEL_CHARACTER_SOCIALLIST,
            Self::SEL_CHARACTER_HOMEBIND => world::SEL_CHARACTER_HOMEBIND,
            Self::SEL_CHARACTER_SPELLCOOLDOWNS => progression::SEL_CHARACTER_SPELLCOOLDOWNS,
            Self::SEL_CHARACTER_SPELL_CHARGES => progression::SEL_CHARACTER_SPELL_CHARGES,
            Self::SEL_CHARACTER_DECLINEDNAMES => character::SEL_CHARACTER_DECLINEDNAMES,
            Self::SEL_GUILD_MEMBER => social::SEL_GUILD_MEMBER,
            Self::SEL_GUILD_MEMBER_EXTENDED => social::SEL_GUILD_MEMBER_EXTENDED,
            Self::SEL_CHARACTER_ACHIEVEMENTS => progression::SEL_CHARACTER_ACHIEVEMENTS,
            Self::SEL_CHARACTER_CRITERIAPROGRESS => progression::SEL_CHARACTER_CRITERIAPROGRESS,
            Self::SEL_CHARACTER_EQUIPMENTSETS => items::SEL_CHARACTER_EQUIPMENTSETS,
            Self::SEL_CHARACTER_TRANSMOG_OUTFITS => items::SEL_CHARACTER_TRANSMOG_OUTFITS,
            Self::SEL_CHARACTER_BGDATA => pets_pvp::SEL_CHARACTER_BGDATA,
            Self::SEL_CHARACTER_GLYPHS => progression::SEL_CHARACTER_GLYPHS,
            Self::SEL_CHARACTER_TALENTS => progression::SEL_CHARACTER_TALENTS,
            Self::SEL_CHARACTER_RANDOMBG => pets_pvp::SEL_CHARACTER_RANDOMBG,
            Self::SEL_CHARACTER_BANNED => character::SEL_CHARACTER_BANNED,
            Self::SEL_CHARACTER_QUESTSTATUSREW => progression::SEL_CHARACTER_QUESTSTATUSREW,
            Self::SEL_CHARACTER_FAVORITE_AUCTIONS => items::SEL_CHARACTER_FAVORITE_AUCTIONS,
            Self::INS_CHARACTER_FAVORITE_AUCTION => items::INS_CHARACTER_FAVORITE_AUCTION,
            Self::DEL_CHARACTER_FAVORITE_AUCTION => items::DEL_CHARACTER_FAVORITE_AUCTION,
            Self::DEL_CHARACTER_FAVORITE_AUCTIONS_BY_CHAR => {
                items::DEL_CHARACTER_FAVORITE_AUCTIONS_BY_CHAR
            }
            Self::SEL_PLAYER_CURRENCY => progression::SEL_PLAYER_CURRENCY,
            Self::UPD_PLAYER_CURRENCY => progression::UPD_PLAYER_CURRENCY,
            Self::REP_PLAYER_CURRENCY => progression::REP_PLAYER_CURRENCY,
            Self::DEL_PLAYER_CURRENCY => progression::DEL_PLAYER_CURRENCY,
            Self::SEL_CHARACTER_ACTIONS_SPEC => progression::SEL_CHARACTER_ACTIONS_SPEC,
            Self::INS_CHARACTER_ACTION => progression::INS_CHARACTER_ACTION,
            Self::UPD_GROUP_TYPE => social::UPD_GROUP_TYPE,
            Self::UPD_GROUP_LEADER => social::UPD_GROUP_LEADER,
            Self::INS_GROUP => social::INS_GROUP,
            Self::INS_GROUP_MEMBER => social::INS_GROUP_MEMBER,
            Self::UPD_GROUP_MEMBER_SUBGROUP => social::UPD_GROUP_MEMBER_SUBGROUP,
            Self::UPD_GROUP_MEMBER_FLAG => social::UPD_GROUP_MEMBER_FLAG,
            Self::UPD_GROUP_DIFFICULTY => social::UPD_GROUP_DIFFICULTY,
            Self::UPD_GROUP_RAID_DIFFICULTY => social::UPD_GROUP_RAID_DIFFICULTY,
            Self::UPD_GROUP_LEGACY_RAID_DIFFICULTY => social::UPD_GROUP_LEGACY_RAID_DIFFICULTY,
            Self::DEL_GROUP_MEMBER => social::DEL_GROUP_MEMBER,
            Self::DEL_GROUP => social::DEL_GROUP,
            Self::DEL_GROUP_MEMBER_ALL => social::DEL_GROUP_MEMBER_ALL,
            Self::DEL_LFG_DATA => social::DEL_LFG_DATA,
            Self::DEL_GROUP_MEMBERS_WITHOUT_CHARACTER => {
                social::DEL_GROUP_MEMBERS_WITHOUT_CHARACTER
            }
            Self::DEL_GROUPS_WITHOUT_LEADER => social::DEL_GROUPS_WITHOUT_LEADER,
            Self::DEL_GROUPS_WITH_FEWER_THAN_TWO_MEMBERS => {
                social::DEL_GROUPS_WITH_FEWER_THAN_TWO_MEMBERS
            }
            Self::DEL_GROUP_MEMBERS_WITHOUT_GROUP => social::DEL_GROUP_MEMBERS_WITHOUT_GROUP,
            Self::SEL_GROUPS => social::SEL_GROUPS,
            Self::SEL_GROUP_MEMBERS => social::SEL_GROUP_MEMBERS,
            Self::SEL_GROUP_MEMBER_CHARACTER_CACHE => social::SEL_GROUP_MEMBER_CHARACTER_CACHE,
            Self::UPD_CHAR_PLAYED_TIME => progression::UPD_CHAR_PLAYED_TIME,
            Self::SEL_ACCOUNT_INSTANCELOCKTIMES => world::SEL_ACCOUNT_INSTANCELOCKTIMES,
            Self::SEL_AUCTIONS => items::SEL_AUCTIONS,
            Self::INS_AUCTION_ITEMS => items::INS_AUCTION_ITEMS,
            Self::DEL_AUCTION_ITEMS_BY_ITEM => items::DEL_AUCTION_ITEMS_BY_ITEM,
            Self::SEL_AUCTION_BIDDERS => items::SEL_AUCTION_BIDDERS,
            Self::INS_AUCTION_BIDDER => items::INS_AUCTION_BIDDER,
            Self::DEL_AUCTION_BIDDER_BY_PLAYER => items::DEL_AUCTION_BIDDER_BY_PLAYER,
            Self::INS_AUCTION => items::INS_AUCTION,
            Self::DEL_AUCTION => items::DEL_AUCTION,
            Self::UPD_AUCTION_BID => items::UPD_AUCTION_BID,
            Self::UPD_AUCTION_EXPIRATION => items::UPD_AUCTION_EXPIRATION,
            Self::INS_MAIL => items::INS_MAIL,
            Self::DEL_MAIL_BY_ID => items::DEL_MAIL_BY_ID,
            Self::INS_MAIL_ITEM => items::INS_MAIL_ITEM,
            Self::DEL_MAIL_ITEM => items::DEL_MAIL_ITEM,
            Self::DEL_INVALID_MAIL_ITEM => items::DEL_INVALID_MAIL_ITEM,
            Self::DEL_EMPTY_EXPIRED_MAIL => items::DEL_EMPTY_EXPIRED_MAIL,
            Self::SEL_EXPIRED_MAIL => items::SEL_EXPIRED_MAIL,
            Self::SEL_EXPIRED_MAIL_ITEMS => items::SEL_EXPIRED_MAIL_ITEMS,
            Self::UPD_MAIL_RETURNED => items::UPD_MAIL_RETURNED,
            Self::UPD_MAIL_ITEM_RECEIVER => items::UPD_MAIL_ITEM_RECEIVER,
            Self::UPD_ITEM_OWNER => items::UPD_ITEM_OWNER,
            Self::DEL_ACCOUNT_INSTANCE_LOCK_TIMES => world::DEL_ACCOUNT_INSTANCE_LOCK_TIMES,
            Self::INS_ACCOUNT_INSTANCE_LOCK_TIMES => world::INS_ACCOUNT_INSTANCE_LOCK_TIMES,
            Self::SEL_INSTANCE => world::SEL_INSTANCE,
            Self::SEL_CHARACTER_INSTANCE_LOCK => world::SEL_CHARACTER_INSTANCE_LOCK,
            Self::DEL_CHARACTER_INSTANCE_LOCK => world::DEL_CHARACTER_INSTANCE_LOCK,
            Self::DEL_CHARACTER_INSTANCE_LOCK_BY_GUID => world::DEL_CHARACTER_INSTANCE_LOCK_BY_GUID,
            Self::INS_CHARACTER_INSTANCE_LOCK => world::INS_CHARACTER_INSTANCE_LOCK,
            Self::UPD_CHARACTER_INSTANCE_LOCK_EXTENSION => {
                world::UPD_CHARACTER_INSTANCE_LOCK_EXTENSION
            }
            Self::UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE => {
                world::UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE
            }
            Self::DEL_INSTANCE => world::DEL_INSTANCE,
            Self::INS_INSTANCE => world::INS_INSTANCE,
            Self::SEL_RESPAWNS => world::SEL_RESPAWNS,
            Self::SEL_ALL_RESPAWNS => world::SEL_ALL_RESPAWNS,
            Self::REP_RESPAWN => world::REP_RESPAWN,
            Self::DEL_RESPAWN => world::DEL_RESPAWN,
            Self::DEL_ALL_RESPAWNS => world::DEL_ALL_RESPAWNS,
            Self::SEL_GM_BUGS => gm_support::SEL_GM_BUGS,
            Self::REP_GM_BUG => gm_support::REP_GM_BUG,
            Self::DEL_GM_BUG => gm_support::DEL_GM_BUG,
            Self::DEL_ALL_GM_BUGS => gm_support::DEL_ALL_GM_BUGS,
            Self::SEL_GM_COMPLAINTS => gm_support::SEL_GM_COMPLAINTS,
            Self::REP_GM_COMPLAINT => gm_support::REP_GM_COMPLAINT,
            Self::DEL_GM_COMPLAINT => gm_support::DEL_GM_COMPLAINT,
            Self::SEL_GM_COMPLAINT_CHATLINES => gm_support::SEL_GM_COMPLAINT_CHATLINES,
            Self::INS_GM_COMPLAINT_CHATLINE => gm_support::INS_GM_COMPLAINT_CHATLINE,
            Self::DEL_GM_COMPLAINT_CHATLOG => gm_support::DEL_GM_COMPLAINT_CHATLOG,
            Self::DEL_ALL_GM_COMPLAINTS => gm_support::DEL_ALL_GM_COMPLAINTS,
            Self::DEL_ALL_GM_COMPLAINT_CHATLOGS => gm_support::DEL_ALL_GM_COMPLAINT_CHATLOGS,
            Self::SEL_GM_SUGGESTIONS => gm_support::SEL_GM_SUGGESTIONS,
            Self::REP_GM_SUGGESTION => gm_support::REP_GM_SUGGESTION,
            Self::DEL_GM_SUGGESTION => gm_support::DEL_GM_SUGGESTION,
            Self::DEL_ALL_GM_SUGGESTIONS => gm_support::DEL_ALL_GM_SUGGESTIONS,
            Self::INS_LFG_DATA => social::INS_LFG_DATA,
            Self::DEL_GAME_EVENT_SAVE => world::DEL_GAME_EVENT_SAVE,
            Self::INS_GAME_EVENT_SAVE => world::INS_GAME_EVENT_SAVE,
            Self::SEL_GAME_EVENT_CONDITION_SAVES => world::SEL_GAME_EVENT_CONDITION_SAVES,
            Self::DEL_ALL_GAME_EVENT_CONDITION_SAVE => world::DEL_ALL_GAME_EVENT_CONDITION_SAVE,
            Self::DEL_GAME_EVENT_CONDITION_SAVE => world::DEL_GAME_EVENT_CONDITION_SAVE,
            Self::INS_GAME_EVENT_CONDITION_SAVE => world::INS_GAME_EVENT_CONDITION_SAVE,
            Self::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT => {
                progression::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT
            }
            Self::DEL_CHARACTER_QUESTSTATUS_DAILY => progression::DEL_CHARACTER_QUESTSTATUS_DAILY,
            Self::DEL_CHARACTER_QUESTSTATUS_WEEKLY => progression::DEL_CHARACTER_QUESTSTATUS_WEEKLY,
            Self::DEL_CHARACTER_QUESTSTATUS_MONTHLY => {
                progression::DEL_CHARACTER_QUESTSTATUS_MONTHLY
            }
            Self::DEL_CHARACTER_QUESTSTATUS_SEASONAL => {
                progression::DEL_CHARACTER_QUESTSTATUS_SEASONAL
            }
            Self::INS_CHARACTER_QUESTSTATUS_DAILY => progression::INS_CHARACTER_QUESTSTATUS_DAILY,
            Self::INS_CHARACTER_QUESTSTATUS_WEEKLY => progression::INS_CHARACTER_QUESTSTATUS_WEEKLY,
            Self::INS_CHARACTER_QUESTSTATUS_MONTHLY => {
                progression::INS_CHARACTER_QUESTSTATUS_MONTHLY
            }
            Self::INS_CHARACTER_QUESTSTATUS_SEASONAL => {
                progression::INS_CHARACTER_QUESTSTATUS_SEASONAL
            }
            Self::SEL_WORLD_STATE_VALUES => world::SEL_WORLD_STATE_VALUES,
            Self::REP_WORLD_STATE => world::REP_WORLD_STATE,
            Self::REP_WORLD_VARIABLE => world::REP_WORLD_VARIABLE,
            Self::DEL_INVALID_SPELL_SPELLS => progression::DEL_INVALID_SPELL_SPELLS,
            Self::UPD_DELETE_INFO => character::UPD_DELETE_INFO,
            Self::UPD_RESTORE_DELETE_INFO => character::UPD_RESTORE_DELETE_INFO,
            Self::UPD_ZONE => world::UPD_ZONE,
            Self::UPD_LEVEL => progression::UPD_LEVEL,
            Self::DEL_INVALID_ACHIEV_PROGRESS_CRITERIA => {
                progression::DEL_INVALID_ACHIEV_PROGRESS_CRITERIA
            }
            Self::DEL_INVALID_ACHIEV_PROGRESS_CRITERIA_GUILD => {
                social::DEL_INVALID_ACHIEV_PROGRESS_CRITERIA_GUILD
            }
            Self::DEL_INVALID_ACHIEVMENT => progression::DEL_INVALID_ACHIEVMENT,
            Self::DEL_INVALID_PET_SPELL => pets_pvp::DEL_INVALID_PET_SPELL,
            Self::UPD_CHAR_NAME_AT_LOGIN => character::UPD_CHAR_NAME_AT_LOGIN,
            Self::DEL_CHARACTER_SKILL => progression::DEL_CHARACTER_SKILL,
            Self::UPD_CHARACTER_SOCIAL_FLAGS => social::UPD_CHARACTER_SOCIAL_FLAGS,
            Self::INS_CHARACTER_SOCIAL => social::INS_CHARACTER_SOCIAL,
            Self::DEL_CHARACTER_SOCIAL => social::DEL_CHARACTER_SOCIAL,
            Self::UPD_CHARACTER_SOCIAL_NOTE => social::UPD_CHARACTER_SOCIAL_NOTE,
            Self::UPD_CHARACTER_POSITION => world::UPD_CHARACTER_POSITION,
            Self::UPD_CHARACTER_POSITION_BY_MAPID => world::UPD_CHARACTER_POSITION_BY_MAPID,
            Self::UPD_CHARACTER_POSITION_PRESERVE_TRAVEL => {
                world::UPD_CHARACTER_POSITION_PRESERVE_TRAVEL
            }
            Self::SEL_CHARACTER_AURA_FROZEN => progression::SEL_CHARACTER_AURA_FROZEN,
            Self::SEL_CHARACTER_ONLINE => character::SEL_CHARACTER_ONLINE,
            Self::SEL_CHAR_DEL_INFO_BY_GUID => character::SEL_CHAR_DEL_INFO_BY_GUID,
            Self::SEL_CHAR_DEL_INFO_BY_NAME => character::SEL_CHAR_DEL_INFO_BY_NAME,
            Self::SEL_CHAR_DEL_INFO => character::SEL_CHAR_DEL_INFO,
            Self::SEL_CHARS_BY_ACCOUNT_ID => character::SEL_CHARS_BY_ACCOUNT_ID,
            Self::SEL_CHAR_PINFO => character::SEL_CHAR_PINFO,
            Self::SEL_PINFO_BANS => character::SEL_PINFO_BANS,
            Self::SEL_PINFO_MAILS => items::SEL_PINFO_MAILS,
            Self::SEL_PINFO_XP => progression::SEL_PINFO_XP,
            Self::SEL_CHAR_HOMEBIND => world::SEL_CHAR_HOMEBIND,
            Self::SEL_CHAR_GUID_NAME_BY_ACC => character::SEL_CHAR_GUID_NAME_BY_ACC,
            Self::SEL_CHAR_CUSTOMIZE_INFO => character::SEL_CHAR_CUSTOMIZE_INFO,
            Self::SEL_CHAR_RACE_OR_FACTION_CHANGE_INFOS => {
                character::SEL_CHAR_RACE_OR_FACTION_CHANGE_INFOS
            }
            Self::SEL_CHAR_COD_ITEM_MAIL => items::SEL_CHAR_COD_ITEM_MAIL,
            Self::SEL_CHAR_SOCIAL => social::SEL_CHAR_SOCIAL,
            Self::SEL_CHAR_OLD_CHARS => character::SEL_CHAR_OLD_CHARS,
            Self::SEL_MAIL => items::SEL_MAIL,
            Self::DEL_CHAR_AURA_FROZEN => progression::DEL_CHAR_AURA_FROZEN,
            Self::SEL_CHAR_INVENTORY_COUNT_ITEM => items::SEL_CHAR_INVENTORY_COUNT_ITEM,
            Self::SEL_MAIL_COUNT_ITEM => items::SEL_MAIL_COUNT_ITEM,
            Self::SEL_AUCTIONHOUSE_COUNT_ITEM => items::SEL_AUCTIONHOUSE_COUNT_ITEM,
            Self::SEL_GUILD_BANK_COUNT_ITEM => social::SEL_GUILD_BANK_COUNT_ITEM,
            Self::SEL_CHAR_INVENTORY_ITEM_BY_ENTRY => items::SEL_CHAR_INVENTORY_ITEM_BY_ENTRY,
            Self::SEL_MAIL_ITEMS_BY_ENTRY => items::SEL_MAIL_ITEMS_BY_ENTRY,
            Self::SEL_AUCTIONHOUSE_ITEM_BY_ENTRY => items::SEL_AUCTIONHOUSE_ITEM_BY_ENTRY,
            Self::SEL_GUILD_BANK_ITEM_BY_ENTRY => social::SEL_GUILD_BANK_ITEM_BY_ENTRY,
            Self::DEL_CHAR_ACHIEVEMENT => progression::DEL_CHAR_ACHIEVEMENT,
            Self::DEL_CHAR_ACHIEVEMENT_PROGRESS => progression::DEL_CHAR_ACHIEVEMENT_PROGRESS,
            Self::INS_CHAR_ACHIEVEMENT => progression::INS_CHAR_ACHIEVEMENT,
            Self::DEL_CHAR_ACHIEVEMENT_PROGRESS_BY_CRITERIA => {
                progression::DEL_CHAR_ACHIEVEMENT_PROGRESS_BY_CRITERIA
            }
            Self::INS_CHAR_ACHIEVEMENT_PROGRESS => progression::INS_CHAR_ACHIEVEMENT_PROGRESS,
            Self::INS_CHAR_GIFT => items::INS_CHAR_GIFT,
            Self::DEL_MAIL_ITEM_BY_ID => items::DEL_MAIL_ITEM_BY_ID,
            Self::INS_PETITION => social::INS_PETITION,
            Self::DEL_PETITION_BY_GUID => social::DEL_PETITION_BY_GUID,
            Self::DEL_PETITION_SIGNATURE_BY_GUID => social::DEL_PETITION_SIGNATURE_BY_GUID,
            Self::DEL_CHAR_DECLINED_NAME => character::DEL_CHAR_DECLINED_NAME,
            Self::INS_CHAR_DECLINED_NAME => character::INS_CHAR_DECLINED_NAME,
            Self::UPD_CHAR_RACE => character::UPD_CHAR_RACE,
            Self::DEL_CHAR_SKILL_LANGUAGES => progression::DEL_CHAR_SKILL_LANGUAGES,
            Self::INS_CHAR_SKILL_LANGUAGE => progression::INS_CHAR_SKILL_LANGUAGE,
            Self::UPD_CHAR_TAXI_PATH => world::UPD_CHAR_TAXI_PATH,
            Self::UPD_CHAR_TAXIMASK => world::UPD_CHAR_TAXIMASK,
            Self::DEL_CHAR_QUESTSTATUS => progression::DEL_CHAR_QUESTSTATUS,
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES => progression::DEL_CHAR_QUESTSTATUS_OBJECTIVES,
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA => {
                progression::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA
            }
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS => {
                progression::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS
            }
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS_BY_CRITERIA => {
                progression::DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS_BY_CRITERIA
            }
            Self::DEL_CHAR_SOCIAL_BY_GUID => social::DEL_CHAR_SOCIAL_BY_GUID,
            Self::DEL_CHAR_SOCIAL_BY_FRIEND => social::DEL_CHAR_SOCIAL_BY_FRIEND,
            Self::DEL_CHAR_ACHIEVEMENT_BY_ACHIEVEMENT => {
                progression::DEL_CHAR_ACHIEVEMENT_BY_ACHIEVEMENT
            }
            Self::UPD_CHAR_ACHIEVEMENT => progression::UPD_CHAR_ACHIEVEMENT,
            Self::UPD_CHAR_INVENTORY_FACTION_CHANGE => items::UPD_CHAR_INVENTORY_FACTION_CHANGE,
            Self::DEL_CHAR_SPELL_BY_SPELL => progression::DEL_CHAR_SPELL_BY_SPELL,
            Self::UPD_CHAR_SPELL_FACTION_CHANGE => progression::UPD_CHAR_SPELL_FACTION_CHANGE,
            Self::SEL_CHAR_REP_BY_FACTION => progression::SEL_CHAR_REP_BY_FACTION,
            Self::DEL_CHAR_REP_BY_FACTION => progression::DEL_CHAR_REP_BY_FACTION,
            Self::UPD_CHAR_REP_FACTION_CHANGE => progression::UPD_CHAR_REP_FACTION_CHANGE,
            Self::UPD_CHAR_TITLES_FACTION_CHANGE => progression::UPD_CHAR_TITLES_FACTION_CHANGE,
            Self::RES_CHAR_TITLES_FACTION_CHANGE => progression::RES_CHAR_TITLES_FACTION_CHANGE,
            Self::DEL_CHAR_SPELL_COOLDOWNS => progression::DEL_CHAR_SPELL_COOLDOWNS,
            Self::INS_CHAR_SPELL_COOLDOWN => progression::INS_CHAR_SPELL_COOLDOWN,
            Self::DEL_CHAR_SPELL_CHARGES => progression::DEL_CHAR_SPELL_CHARGES,
            Self::INS_CHAR_SPELL_CHARGES => progression::INS_CHAR_SPELL_CHARGES,
            Self::DEL_CHAR_ACTION => progression::DEL_CHAR_ACTION,
            Self::DEL_CHAR_AURA => progression::DEL_CHAR_AURA,
            Self::DEL_CHAR_AURA_EFFECT => progression::DEL_CHAR_AURA_EFFECT,
            Self::DEL_CHAR_GIFT => items::DEL_CHAR_GIFT,
            Self::DEL_CHAR_INVENTORY => items::DEL_CHAR_INVENTORY,
            Self::DEL_CHAR_QUESTSTATUS_REWARDED => progression::DEL_CHAR_QUESTSTATUS_REWARDED,
            Self::DEL_CHAR_SPELL => progression::DEL_CHAR_SPELL,
            Self::DEL_MAIL => items::DEL_MAIL,
            Self::DEL_MAIL_ITEMS => items::DEL_MAIL_ITEMS,
            Self::DEL_CHAR_ACHIEVEMENTS => progression::DEL_CHAR_ACHIEVEMENTS,
            Self::DEL_CHAR_EQUIPMENTSETS => items::DEL_CHAR_EQUIPMENTSETS,
            Self::DEL_CHAR_TRANSMOG_OUTFITS => items::DEL_CHAR_TRANSMOG_OUTFITS,
            Self::DEL_GUILD_EVENTLOG_BY_PLAYER => social::DEL_GUILD_EVENTLOG_BY_PLAYER,
            Self::DEL_GUILD_BANK_EVENTLOG_BY_PLAYER => social::DEL_GUILD_BANK_EVENTLOG_BY_PLAYER,
            Self::DEL_CHAR_GLYPHS => progression::DEL_CHAR_GLYPHS,
            Self::DEL_CHAR_TALENT => progression::DEL_CHAR_TALENT,
            Self::DEL_CHAR_SKILLS => progression::DEL_CHAR_SKILLS,
            Self::INS_CHAR_ACTION => progression::INS_CHAR_ACTION,
            Self::UPD_CHAR_ACTION => progression::UPD_CHAR_ACTION,
            Self::DEL_CHAR_ACTION_BY_BUTTON_SPEC => progression::DEL_CHAR_ACTION_BY_BUTTON_SPEC,
            Self::DEL_CHAR_ACTION_BY_SPEC => progression::DEL_CHAR_ACTION_BY_SPEC,
            Self::DEL_CHAR_ACTION_BY_TRAIT_CONFIG => progression::DEL_CHAR_ACTION_BY_TRAIT_CONFIG,
            Self::DEL_CHAR_INVENTORY_BY_ITEM => items::DEL_CHAR_INVENTORY_BY_ITEM,
            Self::DEL_CHAR_INVENTORY_BY_BAG_SLOT => items::DEL_CHAR_INVENTORY_BY_BAG_SLOT,
            Self::UPD_MAIL => items::UPD_MAIL,
            Self::REP_CHAR_QUESTSTATUS => progression::REP_CHAR_QUESTSTATUS,
            Self::DEL_CHAR_QUESTSTATUS_BY_QUEST => progression::DEL_CHAR_QUESTSTATUS_BY_QUEST,
            Self::INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA => {
                progression::INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA
            }
            Self::INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS => {
                progression::INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA_PROGRESS
            }
            Self::INS_CHAR_QUESTSTATUS_REWARDED => progression::INS_CHAR_QUESTSTATUS_REWARDED,
            Self::DEL_CHAR_QUESTSTATUS_REWARDED_BY_QUEST => {
                progression::DEL_CHAR_QUESTSTATUS_REWARDED_BY_QUEST
            }
            Self::UPD_CHAR_QUESTSTATUS_REWARDED_FACTION_CHANGE => {
                progression::UPD_CHAR_QUESTSTATUS_REWARDED_FACTION_CHANGE
            }
            Self::UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE => {
                progression::UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE
            }
            Self::UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE_BY_QUEST => {
                progression::UPD_CHAR_QUESTSTATUS_REWARDED_ACTIVE_BY_QUEST
            }
            Self::DEL_INVALID_QUEST_PROGRESS_CRITERIA => {
                progression::DEL_INVALID_QUEST_PROGRESS_CRITERIA
            }
            Self::DEL_CHAR_SKILL_BY_SKILL => progression::DEL_CHAR_SKILL_BY_SKILL,
            Self::INS_CHAR_SKILLS => progression::INS_CHAR_SKILLS,
            Self::UPD_CHAR_SKILLS => progression::UPD_CHAR_SKILLS,
            Self::INS_CHAR_SPELL => progression::INS_CHAR_SPELL,
            Self::UPSERT_CHAR_SPELL_LEARN_FALLBACK => progression::UPSERT_CHAR_SPELL_LEARN_FALLBACK,
            Self::DEL_CHAR_SPELL_FAVORITE => progression::DEL_CHAR_SPELL_FAVORITE,
            Self::DEL_CHAR_SPELL_FAVORITE_BY_CHAR => progression::DEL_CHAR_SPELL_FAVORITE_BY_CHAR,
            Self::INS_CHAR_SPELL_FAVORITE => progression::INS_CHAR_SPELL_FAVORITE,
            Self::DEL_CHAR_STATS => progression::DEL_CHAR_STATS,
            Self::INS_CHAR_STATS => progression::INS_CHAR_STATS,
            Self::DEL_PETITION_BY_OWNER => social::DEL_PETITION_BY_OWNER,
            Self::DEL_PETITION_SIGNATURE_BY_OWNER => social::DEL_PETITION_SIGNATURE_BY_OWNER,
            Self::INS_CHAR_GLYPHS => progression::INS_CHAR_GLYPHS,
            Self::INS_CHAR_TALENT => progression::INS_CHAR_TALENT,
            Self::UPD_CHAR_LIST_SLOT => character::UPD_CHAR_LIST_SLOT,
            Self::INS_CHAR_FISHINGSTEPS => progression::INS_CHAR_FISHINGSTEPS,
            Self::DEL_CHAR_FISHINGSTEPS => progression::DEL_CHAR_FISHINGSTEPS,
            Self::SEL_CHAR_TRAIT_ENTRIES => progression::SEL_CHAR_TRAIT_ENTRIES,
            Self::INS_CHAR_TRAIT_ENTRIES => progression::INS_CHAR_TRAIT_ENTRIES,
            Self::DEL_CHAR_TRAIT_ENTRIES => progression::DEL_CHAR_TRAIT_ENTRIES,
            Self::DEL_CHAR_TRAIT_ENTRIES_BY_CHAR => progression::DEL_CHAR_TRAIT_ENTRIES_BY_CHAR,
            Self::SEL_CHAR_TRAIT_CONFIGS => progression::SEL_CHAR_TRAIT_CONFIGS,
            Self::INS_CHAR_TRAIT_CONFIGS => progression::INS_CHAR_TRAIT_CONFIGS,
            Self::DEL_CHAR_TRAIT_CONFIGS => progression::DEL_CHAR_TRAIT_CONFIGS,
            Self::DEL_CHAR_TRAIT_CONFIGS_BY_CHAR => progression::DEL_CHAR_TRAIT_CONFIGS_BY_CHAR,
            Self::DEL_RESET_CHARACTER_QUESTSTATUS_DAILY => {
                progression::DEL_RESET_CHARACTER_QUESTSTATUS_DAILY
            }
            Self::DEL_RESET_CHARACTER_QUESTSTATUS_WEEKLY => {
                progression::DEL_RESET_CHARACTER_QUESTSTATUS_WEEKLY
            }
            Self::DEL_RESET_CHARACTER_QUESTSTATUS_MONTHLY => {
                progression::DEL_RESET_CHARACTER_QUESTSTATUS_MONTHLY
            }
            Self::SEL_CHAR_VOID_STORAGE => items::SEL_CHAR_VOID_STORAGE,
            Self::REP_CHAR_VOID_STORAGE_ITEM => items::REP_CHAR_VOID_STORAGE_ITEM,
            Self::DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID => {
                items::DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID
            }
            Self::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT => items::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT,
            Self::SEL_CHAR_CUF_PROFILES => character::SEL_CHAR_CUF_PROFILES,
            Self::REP_CHAR_CUF_PROFILES => character::REP_CHAR_CUF_PROFILES,
            Self::DEL_CHAR_CUF_PROFILES_BY_ID => character::DEL_CHAR_CUF_PROFILES_BY_ID,
            Self::DEL_CHAR_CUF_PROFILES => character::DEL_CHAR_CUF_PROFILES,
            Self::REP_CALENDAR_EVENT => social::REP_CALENDAR_EVENT,
            Self::DEL_CALENDAR_EVENT => social::DEL_CALENDAR_EVENT,
            Self::REP_CALENDAR_INVITE => social::REP_CALENDAR_INVITE,
            Self::DEL_CALENDAR_INVITE => social::DEL_CALENDAR_INVITE,
            Self::SEL_CHAR_PET_IDS => pets_pvp::SEL_CHAR_PET_IDS,
            Self::DEL_CHAR_PET_DECLINEDNAME_BY_OWNER => {
                pets_pvp::DEL_CHAR_PET_DECLINEDNAME_BY_OWNER
            }
            Self::DEL_CHAR_PET_DECLINEDNAME => pets_pvp::DEL_CHAR_PET_DECLINEDNAME,
            Self::INS_CHAR_PET_DECLINEDNAME => pets_pvp::INS_CHAR_PET_DECLINEDNAME,
            Self::SEL_PET_AURA => pets_pvp::SEL_PET_AURA,
            Self::SEL_PET_AURA_EFFECT => pets_pvp::SEL_PET_AURA_EFFECT,
            Self::SEL_PET_SPELL => pets_pvp::SEL_PET_SPELL,
            Self::SEL_PET_SPELL_COOLDOWN => pets_pvp::SEL_PET_SPELL_COOLDOWN,
            Self::SEL_PET_DECLINED_NAME => pets_pvp::SEL_PET_DECLINED_NAME,
            Self::DEL_PET_AURAS => pets_pvp::DEL_PET_AURAS,
            Self::DEL_PET_AURA_EFFECTS => pets_pvp::DEL_PET_AURA_EFFECTS,
            Self::DEL_PET_SPELLS => pets_pvp::DEL_PET_SPELLS,
            Self::DEL_PET_SPELL_COOLDOWNS => pets_pvp::DEL_PET_SPELL_COOLDOWNS,
            Self::INS_PET_SPELL_COOLDOWN => pets_pvp::INS_PET_SPELL_COOLDOWN,
            Self::SEL_PET_SPELL_CHARGES => pets_pvp::SEL_PET_SPELL_CHARGES,
            Self::DEL_PET_SPELL_CHARGES => pets_pvp::DEL_PET_SPELL_CHARGES,
            Self::INS_PET_SPELL_CHARGES => pets_pvp::INS_PET_SPELL_CHARGES,
            Self::DEL_PET_SPELL_BY_SPELL => pets_pvp::DEL_PET_SPELL_BY_SPELL,
            Self::INS_PET_SPELL => pets_pvp::INS_PET_SPELL,
            Self::INS_PET_AURA => pets_pvp::INS_PET_AURA,
            Self::INS_PET_AURA_EFFECT => pets_pvp::INS_PET_AURA_EFFECT,
            Self::SEL_CHAR_PETS => pets_pvp::SEL_CHAR_PETS,
            Self::SEL_CHARACTER_INVENTORY => items::SEL_CHARACTER_INVENTORY,
            Self::SEL_MAILITEMS => items::SEL_MAILITEMS,
            Self::SEL_AUCTION_ITEMS => items::SEL_AUCTION_ITEMS,
            Self::SEL_GUILD_BANK_ITEMS => social::SEL_GUILD_BANK_ITEMS,
            Self::DEL_CHAR_PET_BY_OWNER => pets_pvp::DEL_CHAR_PET_BY_OWNER,
            Self::UPD_CHAR_PET_NAME => pets_pvp::UPD_CHAR_PET_NAME,
            Self::UPD_CHAR_PET_SLOT_BY_ID => pets_pvp::UPD_CHAR_PET_SLOT_BY_ID,
            Self::DEL_CHAR_PET_BY_ID => pets_pvp::DEL_CHAR_PET_BY_ID,
            Self::DEL_ALL_PET_SPELLS_BY_OWNER => pets_pvp::DEL_ALL_PET_SPELLS_BY_OWNER,
            Self::UPD_PET_SPECS_BY_OWNER => pets_pvp::UPD_PET_SPECS_BY_OWNER,
            Self::INS_PET => pets_pvp::INS_PET,
            Self::SEL_PVPSTATS_MAXID => pets_pvp::SEL_PVPSTATS_MAXID,
            Self::INS_PVPSTATS_BATTLEGROUND => pets_pvp::INS_PVPSTATS_BATTLEGROUND,
            Self::INS_PVPSTATS_PLAYER => pets_pvp::INS_PVPSTATS_PLAYER,
            Self::SEL_PVPSTATS_FACTIONS_OVERALL => pets_pvp::SEL_PVPSTATS_FACTIONS_OVERALL,
            Self::INS_QUEST_TRACK => progression::INS_QUEST_TRACK,
            Self::UPD_QUEST_TRACK_GM_COMPLETE => progression::UPD_QUEST_TRACK_GM_COMPLETE,
            Self::UPD_QUEST_TRACK_COMPLETE_TIME => progression::UPD_QUEST_TRACK_COMPLETE_TIME,
            Self::UPD_QUEST_TRACK_ABANDON_TIME => progression::UPD_QUEST_TRACK_ABANDON_TIME,
            Self::SEL_CHARACTER_AURA_STORED_LOCATIONS => {
                progression::SEL_CHARACTER_AURA_STORED_LOCATIONS
            }
            Self::DEL_CHARACTER_AURA_STORED_LOCATIONS_BY_GUID => {
                progression::DEL_CHARACTER_AURA_STORED_LOCATIONS_BY_GUID
            }
            Self::DEL_CHARACTER_AURA_STORED_LOCATION => {
                progression::DEL_CHARACTER_AURA_STORED_LOCATION
            }
            Self::INS_CHARACTER_AURA_STORED_LOCATION => {
                progression::INS_CHARACTER_AURA_STORED_LOCATION
            }
            Self::SEL_WAR_MODE_TUNING => progression::SEL_WAR_MODE_TUNING,
            Self::UPD_CHAR_XP => progression::UPD_CHAR_XP,
            Self::UPD_CHAR_LEVEL => progression::UPD_CHAR_LEVEL,
            Self::UPD_CHAR_MONEY => progression::UPD_CHAR_MONEY,
            Self::UPD_CHAR_PLAYER_FLAGS => character::UPD_CHAR_PLAYER_FLAGS,
            Self::SEL_CHAR_MONEY_FOR_UPDATE => progression::SEL_CHAR_MONEY_FOR_UPDATE,
            Self::SEL_CHAR_MONEY => progression::SEL_CHAR_MONEY,
            Self::UPD_CHAR_HEALTH => progression::UPD_CHAR_HEALTH,
            Self::UPD_CHAR_POWERS => progression::UPD_CHAR_POWERS,
            Self::UPD_CHAR_REST_STATE => progression::UPD_CHAR_REST_STATE,
            Self::UPD_CHAR_ONLINE_REST_STATE => progression::UPD_CHAR_ONLINE_REST_STATE,
            Self::UPD_CHAR_TALENT_RESET_STATE => progression::UPD_CHAR_TALENT_RESET_STATE,
            Self::UPD_CHAR_DIFFICULTIES => world::UPD_CHAR_DIFFICULTIES,
            Self::UPD_CHAR_EXPLORED_ZONES => world::UPD_CHAR_EXPLORED_ZONES,
            Self::SEL_MAX_ITEM_GUID => items::SEL_MAX_ITEM_GUID,
            Self::SEL_MAX_EQUIPMENT_SET_GUID => items::SEL_MAX_EQUIPMENT_SET_GUID,
            Self::SEL_MAX_VOID_STORAGE_ITEM_ID => items::SEL_MAX_VOID_STORAGE_ITEM_ID,
            Self::DEL_INVALID_CHAR_INVENTORY_ITEM_GUIDS => {
                items::DEL_INVALID_CHAR_INVENTORY_ITEM_GUIDS
            }
            Self::DEL_INVALID_MAIL_ITEM_GUIDS => items::DEL_INVALID_MAIL_ITEM_GUIDS,
            Self::DEL_INVALID_AUCTION_ITEM_GUIDS => items::DEL_INVALID_AUCTION_ITEM_GUIDS,
            Self::DEL_INVALID_GUILD_BANK_ITEM_GUIDS => social::DEL_INVALID_GUILD_BANK_ITEM_GUIDS,
            Self::DEL_INVALID_ITEM_LOOT_ITEMS_GUIDS => items::DEL_INVALID_ITEM_LOOT_ITEMS_GUIDS,
            Self::DEL_INVALID_ITEM_LOOT_MONEY_GUIDS => items::DEL_INVALID_ITEM_LOOT_MONEY_GUIDS,
            Self::INS_ITEM_INSTANCE => items::INS_ITEM_INSTANCE,
            Self::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT => {
                items::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT
            }
            Self::INS_ITEM_INSTANCE_CLONE => items::INS_ITEM_INSTANCE_CLONE,
            Self::UPD_ITEM_INSTANCE_COUNT => items::UPD_ITEM_INSTANCE_COUNT,
            Self::UPD_ITEM_INSTANCE_DURABILITY => items::UPD_ITEM_INSTANCE_DURABILITY,
            Self::UPD_ITEM_INSTANCE_FLAGS => items::UPD_ITEM_INSTANCE_FLAGS,
            Self::UPD_ITEM_INSTANCE_ENCHANTMENTS => items::UPD_ITEM_INSTANCE_ENCHANTMENTS,
            Self::UPD_ITEM_INSTANCE_STORAGE_MUTABLE => items::UPD_ITEM_INSTANCE_STORAGE_MUTABLE,
            Self::SEL_CHARACTER_GIFT_BY_ITEM => items::SEL_CHARACTER_GIFT_BY_ITEM,
            Self::DEL_GIFT => items::DEL_GIFT,
            Self::UPD_ITEM_INSTANCE_OPEN_GIFT => items::UPD_ITEM_INSTANCE_OPEN_GIFT,
            Self::INS_CHAR_INVENTORY => items::INS_CHAR_INVENTORY,
            Self::REP_CHAR_INVENTORY_ITEM => items::REP_CHAR_INVENTORY_ITEM,
            Self::DEL_ITEM_INSTANCE => items::DEL_ITEM_INSTANCE,
            Self::DEL_ITEM_INSTANCE_BY_GUID_AND_OWNER => items::DEL_ITEM_INSTANCE_BY_GUID_AND_OWNER,
            Self::SEL_UNCAGE_ITEM_STATE => items::SEL_UNCAGE_ITEM_STATE,
            Self::INS_BATTLE_PET_PURCHASE => pets_pvp::INS_BATTLE_PET_PURCHASE,
            Self::SEL_BATTLE_PET_PURCHASE_BY_KEY => pets_pvp::SEL_BATTLE_PET_PURCHASE_BY_KEY,
            Self::SEL_BATTLE_PET_PURCHASE_PENDING => pets_pvp::SEL_BATTLE_PET_PURCHASE_PENDING,
            Self::UPD_BATTLE_PET_PURCHASE_PUBLISHED => pets_pvp::UPD_BATTLE_PET_PURCHASE_PUBLISHED,
            Self::UPD_BATTLE_PET_PURCHASE_COMPLETED => pets_pvp::UPD_BATTLE_PET_PURCHASE_COMPLETED,
            Self::UPD_BATTLE_PET_PURCHASE_COMPENSATION_PENDING => {
                pets_pvp::UPD_BATTLE_PET_PURCHASE_COMPENSATION_PENDING
            }
            Self::UPD_BATTLE_PET_PURCHASE_COMPENSATED => {
                pets_pvp::UPD_BATTLE_PET_PURCHASE_COMPENSATED
            }
            Self::UPD_BATTLE_PET_PURCHASE_TERMINAL_FAILURE => {
                pets_pvp::UPD_BATTLE_PET_PURCHASE_TERMINAL_FAILURE
            }
            Self::UPD_CHARACTER_MONEY_GUARDED => progression::UPD_CHARACTER_MONEY_GUARDED,
            Self::UPD_CHARACTER_MONEY_REFUND => items::UPD_CHARACTER_MONEY_REFUND,
            Self::SEL_ITEM_REFUNDS => items::SEL_ITEM_REFUNDS,
            Self::SEL_ITEM_BOP_TRADE => items::SEL_ITEM_BOP_TRADE,
            Self::DEL_ITEM_BOP_TRADE => items::DEL_ITEM_BOP_TRADE,
            Self::INS_ITEM_BOP_TRADE => items::INS_ITEM_BOP_TRADE,
            Self::REP_INVENTORY_ITEM => items::REP_INVENTORY_ITEM,
            Self::REP_ITEM_INSTANCE => items::REP_ITEM_INSTANCE,
            Self::UPD_ITEM_INSTANCE => items::UPD_ITEM_INSTANCE,
            Self::UPD_ITEM_INSTANCE_ON_LOAD => items::UPD_ITEM_INSTANCE_ON_LOAD,
            Self::DEL_ITEM_INSTANCE_BY_OWNER => items::DEL_ITEM_INSTANCE_BY_OWNER,
            Self::INS_ITEM_INSTANCE_GEMS => items::INS_ITEM_INSTANCE_GEMS,
            Self::DEL_ITEM_INSTANCE_GEMS => items::DEL_ITEM_INSTANCE_GEMS,
            Self::DEL_ITEM_INSTANCE_GEMS_BY_OWNER => items::DEL_ITEM_INSTANCE_GEMS_BY_OWNER,
            Self::INS_ITEM_INSTANCE_TRANSMOG => items::INS_ITEM_INSTANCE_TRANSMOG,
            Self::DEL_ITEM_INSTANCE_TRANSMOG => items::DEL_ITEM_INSTANCE_TRANSMOG,
            Self::DEL_ITEM_INSTANCE_TRANSMOG_BY_OWNER => items::DEL_ITEM_INSTANCE_TRANSMOG_BY_OWNER,
            Self::UPD_GIFT_OWNER => items::UPD_GIFT_OWNER,
            Self::SEL_ACCOUNT_BY_NAME => character::SEL_ACCOUNT_BY_NAME,
            Self::UPD_ACCOUNT_BY_GUID => character::UPD_ACCOUNT_BY_GUID,
            Self::SEL_MATCH_MAKER_RATING => pets_pvp::SEL_MATCH_MAKER_RATING,
            Self::SEL_CHARACTER_COUNT => character::SEL_CHARACTER_COUNT,
            Self::UPD_NAME_BY_GUID => character::UPD_NAME_BY_GUID,
            Self::INS_GUILD => social::INS_GUILD,
            Self::DEL_GUILD => social::DEL_GUILD,
            Self::UPD_GUILD_NAME => social::UPD_GUILD_NAME,
            Self::INS_GUILD_MEMBER => social::INS_GUILD_MEMBER,
            Self::DEL_GUILD_MEMBER => social::DEL_GUILD_MEMBER,
            Self::DEL_GUILD_MEMBERS => social::DEL_GUILD_MEMBERS,
            Self::INS_GUILD_RANK => social::INS_GUILD_RANK,
            Self::DEL_GUILD_RANKS => social::DEL_GUILD_RANKS,
            Self::DEL_GUILD_RANK => social::DEL_GUILD_RANK,
            Self::INS_GUILD_BANK_TAB => social::INS_GUILD_BANK_TAB,
            Self::DEL_GUILD_BANK_TAB => social::DEL_GUILD_BANK_TAB,
            Self::DEL_GUILD_BANK_TABS => social::DEL_GUILD_BANK_TABS,
            Self::INS_GUILD_BANK_ITEM => social::INS_GUILD_BANK_ITEM,
            Self::DEL_GUILD_BANK_ITEM => social::DEL_GUILD_BANK_ITEM,
            Self::DEL_GUILD_BANK_ITEMS => social::DEL_GUILD_BANK_ITEMS,
            Self::INS_GUILD_BANK_RIGHT => social::INS_GUILD_BANK_RIGHT,
            Self::DEL_GUILD_BANK_RIGHTS => social::DEL_GUILD_BANK_RIGHTS,
            Self::DEL_GUILD_BANK_RIGHTS_FOR_RANK => social::DEL_GUILD_BANK_RIGHTS_FOR_RANK,
            Self::INS_GUILD_BANK_EVENTLOG => social::INS_GUILD_BANK_EVENTLOG,
            Self::DEL_GUILD_BANK_EVENTLOG => social::DEL_GUILD_BANK_EVENTLOG,
            Self::DEL_GUILD_BANK_EVENTLOGS => social::DEL_GUILD_BANK_EVENTLOGS,
            Self::INS_GUILD_EVENTLOG => social::INS_GUILD_EVENTLOG,
            Self::DEL_GUILD_EVENTLOG => social::DEL_GUILD_EVENTLOG,
            Self::DEL_GUILD_EVENTLOGS => social::DEL_GUILD_EVENTLOGS,
            Self::UPD_GUILD_MEMBER_PNOTE => social::UPD_GUILD_MEMBER_PNOTE,
            Self::UPD_GUILD_MEMBER_OFFNOTE => social::UPD_GUILD_MEMBER_OFFNOTE,
            Self::UPD_GUILD_MEMBER_RANK => social::UPD_GUILD_MEMBER_RANK,
            Self::UPD_GUILD_MOTD => social::UPD_GUILD_MOTD,
            Self::UPD_GUILD_INFO => social::UPD_GUILD_INFO,
            Self::UPD_GUILD_LEADER => social::UPD_GUILD_LEADER,
            Self::UPD_GUILD_RANK_ORDER => social::UPD_GUILD_RANK_ORDER,
            Self::UPD_GUILD_RANK_NAME => social::UPD_GUILD_RANK_NAME,
            Self::UPD_GUILD_RANK_RIGHTS => social::UPD_GUILD_RANK_RIGHTS,
            Self::UPD_GUILD_EMBLEM_INFO => social::UPD_GUILD_EMBLEM_INFO,
            Self::UPD_GUILD_BANK_TAB_INFO => social::UPD_GUILD_BANK_TAB_INFO,
            Self::UPD_GUILD_BANK_MONEY => social::UPD_GUILD_BANK_MONEY,
            Self::UPD_GUILD_RANK_BANK_MONEY => social::UPD_GUILD_RANK_BANK_MONEY,
            Self::UPD_GUILD_BANK_TAB_TEXT => social::UPD_GUILD_BANK_TAB_TEXT,
            Self::INS_GUILD_MEMBER_WITHDRAW_TABS => social::INS_GUILD_MEMBER_WITHDRAW_TABS,
            Self::INS_GUILD_MEMBER_WITHDRAW_MONEY => social::INS_GUILD_MEMBER_WITHDRAW_MONEY,
            Self::DEL_GUILD_MEMBER_WITHDRAW => social::DEL_GUILD_MEMBER_WITHDRAW,
            Self::SEL_CHAR_DATA_FOR_GUILD => social::SEL_CHAR_DATA_FOR_GUILD,
            Self::DEL_GUILD_ACHIEVEMENT => social::DEL_GUILD_ACHIEVEMENT,
            Self::INS_GUILD_ACHIEVEMENT => social::INS_GUILD_ACHIEVEMENT,
            Self::DEL_GUILD_ACHIEVEMENT_CRITERIA => social::DEL_GUILD_ACHIEVEMENT_CRITERIA,
            Self::INS_GUILD_ACHIEVEMENT_CRITERIA => social::INS_GUILD_ACHIEVEMENT_CRITERIA,
            Self::DEL_ALL_GUILD_ACHIEVEMENTS => social::DEL_ALL_GUILD_ACHIEVEMENTS,
            Self::DEL_ALL_GUILD_ACHIEVEMENT_CRITERIA => social::DEL_ALL_GUILD_ACHIEVEMENT_CRITERIA,
            Self::SEL_GUILD_ACHIEVEMENT => social::SEL_GUILD_ACHIEVEMENT,
            Self::SEL_GUILD_ACHIEVEMENT_CRITERIA => social::SEL_GUILD_ACHIEVEMENT_CRITERIA,
            Self::INS_GUILD_NEWS => social::INS_GUILD_NEWS,
            Self::UPD_CHANNEL => social::UPD_CHANNEL,
            Self::UPD_CHANNEL_USAGE => social::UPD_CHANNEL_USAGE,
            Self::UPD_CHANNEL_OWNERSHIP => social::UPD_CHANNEL_OWNERSHIP,
            Self::DEL_CHANNEL => social::DEL_CHANNEL,
            Self::DEL_OLD_CHANNELS => social::DEL_OLD_CHANNELS,
            Self::UPD_EQUIP_SET => items::UPD_EQUIP_SET,
            Self::INS_EQUIP_SET => items::INS_EQUIP_SET,
            Self::DEL_EQUIP_SET => items::DEL_EQUIP_SET,
            Self::UPD_TRANSMOG_OUTFIT => items::UPD_TRANSMOG_OUTFIT,
            Self::INS_TRANSMOG_OUTFIT => items::INS_TRANSMOG_OUTFIT,
            Self::DEL_TRANSMOG_OUTFIT => items::DEL_TRANSMOG_OUTFIT,
            Self::INS_AURA => progression::INS_AURA,
            Self::INS_AURA_EFFECT => progression::INS_AURA_EFFECT,
            Self::SEL_ACCOUNT_DATA => character::SEL_ACCOUNT_DATA,
            Self::REP_ACCOUNT_DATA => character::REP_ACCOUNT_DATA,
            Self::DEL_ACCOUNT_DATA => character::DEL_ACCOUNT_DATA,
            Self::SEL_PLAYER_ACCOUNT_DATA => character::SEL_PLAYER_ACCOUNT_DATA,
            Self::REP_PLAYER_ACCOUNT_DATA => character::REP_PLAYER_ACCOUNT_DATA,
            Self::DEL_PLAYER_ACCOUNT_DATA => character::DEL_PLAYER_ACCOUNT_DATA,
            Self::SEL_TUTORIALS => character::SEL_TUTORIALS,
            Self::INS_TUTORIALS => character::INS_TUTORIALS,
            Self::UPD_TUTORIALS => character::UPD_TUTORIALS,
            Self::DEL_TUTORIALS => character::DEL_TUTORIALS,
            Self::SEL_PETITION => social::SEL_PETITION,
            Self::SEL_PETITION_SIGNATURE => social::SEL_PETITION_SIGNATURE,
            Self::DEL_ALL_PETITION_SIGNATURES => social::DEL_ALL_PETITION_SIGNATURES,
            Self::SEL_PETITION_BY_OWNER => social::SEL_PETITION_BY_OWNER,
            Self::SEL_PETITION_SIGNATURES => social::SEL_PETITION_SIGNATURES,
            Self::SEL_PETITION_SIG_BY_ACCOUNT => social::SEL_PETITION_SIG_BY_ACCOUNT,
            Self::SEL_PETITION_OWNER_BY_GUID => social::SEL_PETITION_OWNER_BY_GUID,
            Self::SEL_PETITION_SIG_BY_GUID => social::SEL_PETITION_SIG_BY_GUID,
            Self::SEL_CHARACTER_ARENAINFO => pets_pvp::SEL_CHARACTER_ARENAINFO,
            Self::INS_ARENA_TEAM => pets_pvp::INS_ARENA_TEAM,
            Self::INS_ARENA_TEAM_MEMBER => pets_pvp::INS_ARENA_TEAM_MEMBER,
            Self::DEL_ARENA_TEAM => pets_pvp::DEL_ARENA_TEAM,
            Self::DEL_ARENA_TEAM_MEMBERS => pets_pvp::DEL_ARENA_TEAM_MEMBERS,
            Self::UPD_ARENA_TEAM_CAPTAIN => pets_pvp::UPD_ARENA_TEAM_CAPTAIN,
            Self::DEL_ARENA_TEAM_MEMBER => pets_pvp::DEL_ARENA_TEAM_MEMBER,
            Self::UPD_ARENA_TEAM_STATS => pets_pvp::UPD_ARENA_TEAM_STATS,
            Self::UPD_ARENA_TEAM_MEMBER => pets_pvp::UPD_ARENA_TEAM_MEMBER,
            Self::DEL_CHARACTER_ARENA_STATS => pets_pvp::DEL_CHARACTER_ARENA_STATS,
            Self::REP_CHARACTER_ARENA_STATS => pets_pvp::REP_CHARACTER_ARENA_STATS,
            Self::UPD_ARENA_TEAM_NAME => pets_pvp::UPD_ARENA_TEAM_NAME,
            Self::INS_PLAYER_BGDATA => pets_pvp::INS_PLAYER_BGDATA,
            Self::DEL_PLAYER_BGDATA => pets_pvp::DEL_PLAYER_BGDATA,
            Self::INS_PLAYER_HOMEBIND => world::INS_PLAYER_HOMEBIND,
            Self::UPD_PLAYER_HOMEBIND => world::UPD_PLAYER_HOMEBIND,
            Self::DEL_PLAYER_HOMEBIND => world::DEL_PLAYER_HOMEBIND,
            Self::SEL_CORPSES => world::SEL_CORPSES,
            Self::INS_CORPSE => world::INS_CORPSE,
            Self::DEL_CORPSE => world::DEL_CORPSE,
            Self::DEL_CORPSES_FROM_MAP => world::DEL_CORPSES_FROM_MAP,
            Self::SEL_CORPSE_PHASES => world::SEL_CORPSE_PHASES,
            Self::INS_CORPSE_PHASES => world::INS_CORPSE_PHASES,
            Self::DEL_CORPSE_PHASES => world::DEL_CORPSE_PHASES,
            Self::SEL_CORPSE_CUSTOMIZATIONS => world::SEL_CORPSE_CUSTOMIZATIONS,
            Self::INS_CORPSE_CUSTOMIZATIONS => world::INS_CORPSE_CUSTOMIZATIONS,
            Self::DEL_CORPSE_CUSTOMIZATIONS => world::DEL_CORPSE_CUSTOMIZATIONS,
            Self::SEL_CORPSE_LOCATION => world::SEL_CORPSE_LOCATION,
            Self::SEL_CHAR_BAG_CONTENTS => items::SEL_CHAR_BAG_CONTENTS,
            Self::DEL_ITEM_REFUND_INSTANCE => items::DEL_ITEM_REFUND_INSTANCE,
            Self::DEL_ITEMCONTAINER_MONEY => items::DEL_ITEMCONTAINER_MONEY,
            Self::DEL_ITEMCONTAINER_ITEMS => items::DEL_ITEMCONTAINER_ITEMS,
            Self::DEL_ITEMCONTAINER_ITEM => items::DEL_ITEMCONTAINER_ITEM,
            Self::SEL_ITEMCONTAINER_MONEY => items::SEL_ITEMCONTAINER_MONEY,
            Self::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE => items::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE,
            Self::INS_ITEMCONTAINER_MONEY => items::INS_ITEMCONTAINER_MONEY,
            Self::SEL_ITEMCONTAINER_ITEMS => items::SEL_ITEMCONTAINER_ITEMS,
            Self::INS_ITEMCONTAINER_ITEMS => items::INS_ITEMCONTAINER_ITEMS,
            Self::INS_ITEM_REFUND_INSTANCE => items::INS_ITEM_REFUND_INSTANCE,
            Self::INS_CHARACTER_SPELL => progression::INS_CHARACTER_SPELL,
            Self::GENERATED_CPP { sql, .. } => sql,
            Self::SEL_CHAR_QUEST_STATUS => progression::SEL_CHAR_QUEST_STATUS,
            Self::SEL_CHARACTER_QUESTSTATUS => progression::SEL_CHARACTER_QUESTSTATUS,
            Self::SEL_CHAR_QUEST_STATUS_OBJECTIVES => progression::SEL_CHAR_QUEST_STATUS_OBJECTIVES,
            Self::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES => {
                progression::SEL_CHARACTER_QUESTSTATUS_OBJECTIVES
            }
            Self::SEL_CHAR_QUEST_STATUS_SEASONAL => progression::SEL_CHAR_QUEST_STATUS_SEASONAL,
            Self::INS_CHAR_QUEST_STATUS => progression::INS_CHAR_QUEST_STATUS,
            Self::DEL_CHAR_QUEST_STATUS => progression::DEL_CHAR_QUEST_STATUS,
            Self::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST => {
                progression::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST
            }
            Self::DEL_CHAR_QUESTSTATUS_OBJECTIVES_BY_QUEST => {
                progression::DEL_CHAR_QUESTSTATUS_OBJECTIVES_BY_QUEST
            }
            Self::REP_CHAR_QUEST_STATUS_OBJECTIVES => progression::REP_CHAR_QUEST_STATUS_OBJECTIVES,
            Self::REP_CHAR_QUESTSTATUS_OBJECTIVES => progression::REP_CHAR_QUESTSTATUS_OBJECTIVES,
        }
    }
}
