// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ReputationMgr` gain arithmetic of the World session (#1363 slice 7b,
//! moved from the World `session/progression/reputation.rs`): the kill and
//! spell reputation gain, the aura and low-level rates, the reward rate and
//! the Recruit-a-Friend bonus.

use wow_entities::RepresentedAuraEffectLikeCpp;
use wow_world_core::session::ReputationGainSourceLikeCpp;

use crate::SocialSessionHostLikeCpp;

pub fn calculate_kill_reputation_gain_like_cpp<H: SocialSessionHostLikeCpp>(
    host: &H,
    creature_level: u8,
    rep: i32,
    faction_id: u32,
) -> i32 {
    calculate_reputation_gain_like_cpp(
        host,
        ReputationGainSourceLikeCpp::Kill,
        u32::from(creature_level),
        rep,
        faction_id,
        false,
    )
}

pub fn reputation_gain_percent_before_reward_rate_like_cpp<H: SocialSessionHostLikeCpp>(
    host: &H,
    source: ReputationGainSourceLikeCpp,
    creature_or_quest_level: u32,
    rep: i32,
    faction_id: u32,
    no_quest_bonus: bool,
) -> Option<f32> {
    let mut percent = 100.0f32;
    let mut rep_mod = if no_quest_bonus {
        0.0
    } else {
        host.hub_ref()
            .resolved_total_represented_aura_modifier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModReputationGain,
            )? as f32
    };

    if source == ReputationGainSourceLikeCpp::Kill {
        rep_mod += host.resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
            RepresentedAuraEffectLikeCpp::ModFactionReputationGain,
            faction_id as i32,
        )? as f32;
    }

    percent += if rep > 0 { rep_mod } else { -rep_mod };

    let reputation_rates = host.hub_ref().config.reputation_rates_like_cpp();
    let low_level_rate = match source {
        ReputationGainSourceLikeCpp::Kill => reputation_rates.low_level_kill,
        ReputationGainSourceLikeCpp::Quest
        | ReputationGainSourceLikeCpp::DailyQuest
        | ReputationGainSourceLikeCpp::WeeklyQuest
        | ReputationGainSourceLikeCpp::MonthlyQuest
        | ReputationGainSourceLikeCpp::RepeatableQuest => reputation_rates.low_level_quest,
        ReputationGainSourceLikeCpp::Spell => 1.0,
    };
    if low_level_rate != 1.0
        && creature_or_quest_level
            < u32::from(
                host.hub_ref()
                    .gray_level(host.hub_ref().player_level_like_cpp()),
            )
    {
        percent *= low_level_rate;
    }

    (percent > 0.0).then_some(percent)
}

pub fn calculate_reputation_gain_like_cpp<H: SocialSessionHostLikeCpp>(
    host: &H,
    source: ReputationGainSourceLikeCpp,
    creature_or_quest_level: u32,
    rep: i32,
    faction_id: u32,
    no_quest_bonus: bool,
) -> i32 {
    let Some(mut percent) = reputation_gain_percent_before_reward_rate_like_cpp(
        host,
        source,
        creature_or_quest_level,
        rep,
        faction_id,
        no_quest_bonus,
    ) else {
        return 0;
    };

    if let Some(rep_rate) = host
        .hub_ref()
        .reputation_reward_rate_for_source_like_cpp(source, faction_id)
    {
        if rep_rate <= 0.0 {
            return 0;
        }
        percent *= rep_rate;
    }

    percent = apply_recruit_a_friend_reputation_bonus_like_cpp(host, source, percent);

    (rep as f32 * percent / 100.0) as i32
}

pub fn apply_recruit_a_friend_reputation_bonus_like_cpp<H: SocialSessionHostLikeCpp>(
    host: &H,
    source: ReputationGainSourceLikeCpp,
    mut percent: f32,
) -> f32 {
    if source != ReputationGainSourceLikeCpp::Spell
        && gets_recruit_a_friend_reputation_bonus_like_cpp(host)
    {
        percent *= 1.0
            + host
                .hub_ref()
                .config
                .reputation_rates_like_cpp()
                .recruit_a_friend_bonus;
    }
    percent
}

fn gets_recruit_a_friend_reputation_bonus_like_cpp<H: SocialSessionHostLikeCpp>(host: &H) -> bool {
    host.gets_recruit_a_friend_bonus_like_cpp(false)
}
