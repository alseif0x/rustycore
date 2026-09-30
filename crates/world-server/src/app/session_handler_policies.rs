//! Immutable process-owned handler policy composition.

use std::sync::Arc;

use wow_config::WorldConfigSet;
use wow_data::CfgCategoriesStore;
use wow_world::session::{
    ChatPolicyCatalogsLikeCpp, GroupInvitePolicyLikeCpp, SupportFeaturePolicyLikeCpp,
};
use wow_world::{ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp};

use crate::bootstrap::{
    declined_names_used_like_cpp, world_config_bool, world_config_f32, world_config_u8,
    world_config_u32,
};

pub(super) fn build_chat_policy(world_configs: &WorldConfigSet) -> Arc<ChatPolicyCatalogsLikeCpp> {
    Arc::new(wow_world::session::ChatPolicyCatalogsLikeCpp {
        addon_channel: world_config_bool(world_configs, "CONFIG_ADDON_CHANNEL", true),
        fake_message_preventing: world_config_bool(
            world_configs,
            "CONFIG_CHAT_FAKE_MESSAGE_PREVENTING",
            false,
        ),
        strict_link_checking_kick: world_config_u8(
            world_configs,
            "CONFIG_CHAT_STRICT_LINK_CHECKING_KICK",
            0,
        ) != 0,
        level_requirements: ChatLevelRequirementsLikeCpp {
            channel: world_config_u8(world_configs, "CONFIG_CHAT_CHANNEL_LEVEL_REQ", 1),
            whisper: world_config_u8(world_configs, "CONFIG_CHAT_WHISPER_LEVEL_REQ", 1),
            emote: world_config_u8(world_configs, "CONFIG_CHAT_EMOTE_LEVEL_REQ", 1),
            say: world_config_u8(world_configs, "CONFIG_CHAT_SAY_LEVEL_REQ", 1),
            yell: world_config_u8(world_configs, "CONFIG_CHAT_YELL_LEVEL_REQ", 1),
        },
        listen_ranges: ChatListenRangesLikeCpp {
            say: world_config_f32(world_configs, "CONFIG_LISTEN_RANGE_SAY", 25.0),
            text_emote: world_config_f32(world_configs, "CONFIG_LISTEN_RANGE_TEXTEMOTE", 25.0),
            yell: world_config_f32(world_configs, "CONFIG_LISTEN_RANGE_YELL", 300.0),
        },
        flood: ChatFloodConfigLikeCpp {
            message_count: world_config_u32(world_configs, "CONFIG_CHATFLOOD_MESSAGE_COUNT", 10),
            message_delay_secs: world_config_u32(
                world_configs,
                "CONFIG_CHATFLOOD_MESSAGE_DELAY",
                1,
            ),
            addon_message_count: world_config_u32(
                world_configs,
                "CONFIG_CHATFLOOD_ADDON_MESSAGE_COUNT",
                100,
            ),
            addon_message_delay_secs: world_config_u32(
                world_configs,
                "CONFIG_CHATFLOOD_ADDON_MESSAGE_DELAY",
                1,
            ),
            mute_time_secs: world_config_u32(world_configs, "CONFIG_CHATFLOOD_MUTE_TIME", 10),
        },
        party_raid_warnings: world_config_bool(
            world_configs,
            "CONFIG_CHAT_PARTY_RAID_WARNINGS",
            false,
        ),
    })
}

pub(super) fn build_group_invite_policy(
    world_configs: &WorldConfigSet,
) -> Arc<GroupInvitePolicyLikeCpp> {
    Arc::new(wow_world::session::GroupInvitePolicyLikeCpp {
        allow_gm_group: world_config_bool(world_configs, "CONFIG_ALLOW_GM_GROUP", false),
        allow_two_side_interaction: world_config_bool(
            world_configs,
            "CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP",
            false,
        ),
        minimum_level: world_config_u32(world_configs, "CONFIG_PARTY_LEVEL_REQ", 1),
    })
}

pub(super) fn build_support_feature_policy(
    world_configs: &WorldConfigSet,
    cfg_categories_store: &CfgCategoriesStore,
) -> Arc<SupportFeaturePolicyLikeCpp> {
    Arc::new(wow_world::session::SupportFeaturePolicyLikeCpp {
        support_enabled: world_config_bool(world_configs, "CONFIG_SUPPORT_ENABLED", true),
        tickets_enabled: world_config_bool(world_configs, "CONFIG_SUPPORT_TICKETS_ENABLED", false),
        bugs_enabled: world_config_bool(world_configs, "CONFIG_SUPPORT_BUGS_ENABLED", false),
        complaints_enabled: world_config_bool(
            world_configs,
            "CONFIG_SUPPORT_COMPLAINTS_ENABLED",
            false,
        ),
        suggestions_enabled: world_config_bool(
            world_configs,
            "CONFIG_SUPPORT_SUGGESTIONS_ENABLED",
            false,
        ),
        character_undelete_enabled: world_config_bool(
            world_configs,
            "CONFIG_FEATURE_SYSTEM_CHARACTER_UNDELETE_ENABLED",
            false,
        ),
        bpay_store_enabled: world_config_bool(
            world_configs,
            "CONFIG_FEATURE_SYSTEM_BPAY_STORE_ENABLED",
            false,
        ),
        max_characters_per_realm: world_config_u32(
            world_configs,
            "CONFIG_CHARACTERS_PER_REALM",
            60,
        ),
        declined_names_used: declined_names_used_like_cpp(world_configs, cfg_categories_store),
    })
}
