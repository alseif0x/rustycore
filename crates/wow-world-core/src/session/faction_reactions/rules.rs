// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{
    FactionReactionCatalogs, FactionReactionError, FactionReactionOwner,
    RepresentedFactionReactionInputLikeCpp, RepresentedGetReactionInputLikeCpp,
};
use wow_data::progression_rewards::FactionEntry;
use wow_data::reputation::ReputationRankLikeCpp;
use wow_progression::ReputationMgrLikeCpp;

#[cfg(test)]
#[path = "../../../unit_tests/session/faction_reactions/rules.rs"]
mod tests;

pub fn resolve_faction_reaction(
    input: RepresentedGetReactionInputLikeCpp,
    source_owner: Option<FactionReactionOwner<'_>>,
    target_owner: Option<FactionReactionOwner<'_>>,
    catalogs: FactionReactionCatalogs<'_>,
) -> Result<ReputationRankLikeCpp, FactionReactionError> {
    resolve_faction_reaction_with_raid(input, source_owner, target_owner, catalogs, || {
        Ok(input.same_raid)
    })
}

pub fn resolve_faction_reaction_with_raid(
    input: RepresentedGetReactionInputLikeCpp,
    source_owner: Option<FactionReactionOwner<'_>>,
    target_owner: Option<FactionReactionOwner<'_>>,
    catalogs: FactionReactionCatalogs<'_>,
    raid_relation: impl FnOnce() -> Result<bool, FactionReactionError>,
) -> Result<ReputationRankLikeCpp, FactionReactionError> {
    use ReputationRankLikeCpp::{Friendly, Hostile, Neutral};

    if input.same_object {
        return Ok(Friendly);
    }
    if input.attackable_by_summoner {
        return Ok(Neutral);
    }
    if input.same_charmer_or_owner_or_self {
        return Ok(Friendly);
    }

    if input.self_has_player_owner {
        let templates = faction_templates(catalogs)?;
        if let Some(target_template) = templates.get(input.target_faction_template_id) {
            let source_owner =
                source_owner.ok_or(FactionReactionError::UnknownSourcePlayerOwner)?;
            if let Some(rank) = forced_rank(source_owner, u32::from(target_template.faction)) {
                return Ok(rank);
            }
        }
    } else if input.target_has_player_owner {
        let templates = faction_templates(catalogs)?;
        if let Some(source_template) = templates.get(input.self_faction_template_id) {
            let rank = if let Some(target_owner) = target_owner {
                forced_rank(target_owner, u32::from(source_template.faction))
            } else if let Some(rank) = input.target_owner_forced_rank_for_self {
                Some(rank)
            } else {
                return Err(FactionReactionError::UnknownTargetPlayerOwner);
            };
            if let Some(rank) = rank {
                return Ok(rank);
            }
        }
    }

    if input.self_unit_player_controlled && input.target_unit_player_controlled {
        if input.self_has_player_owner && input.target_has_player_owner {
            if input.same_player_owner {
                return Ok(Friendly);
            }
            if input.duel_in_progress {
                return Ok(Hostile);
            }
            if raid_relation()? {
                return Ok(Friendly);
            }
        }

        if input.self_ffa_pvp && input.target_ffa_pvp {
            return Ok(Hostile);
        }

        if input.self_has_player_owner {
            let templates = faction_templates(catalogs)?;
            if let Some(target_template) = templates.get(input.target_faction_template_id) {
                let source_owner =
                    source_owner.ok_or(FactionReactionError::UnknownSourcePlayerOwner)?;
                if let Some(rank) = forced_rank(source_owner, u32::from(target_template.faction)) {
                    return Ok(rank);
                }
                if !input.self_ignores_reputation {
                    let factions = catalogs
                        .faction_store
                        .ok_or(FactionReactionError::UnknownFactionStore)?;
                    if let Some(target_faction) = factions.get(u32::from(target_template.faction))
                        && target_faction.can_have_reputation_like_cpp()
                    {
                        if target_template.is_contested_guard_faction_like_cpp()
                            && input.self_player_contested_pvp
                        {
                            return Ok(Hostile);
                        }
                        let manager =
                            ReputationMgrLikeCpp::borrowing_like_cpp(source_owner.reputation);
                        if manager.is_at_war_with_faction_like_cpp(target_faction) {
                            return Ok(Hostile);
                        }
                        return Ok(Friendly);
                    }
                }
            }
        }
    }

    resolve_faction_template_reaction(
        RepresentedFactionReactionInputLikeCpp {
            source_faction_template_id: input.self_faction_template_id,
            target_faction_template_id: input.target_faction_template_id,
            target_has_player_owner: input.target_has_player_owner,
            target_player_owner_is_current_session: input.target_player_owner_is_current_session,
            target_player_contested_pvp: input.target_player_contested_pvp,
            target_is_unit: input.target_is_unit,
            target_ignores_reputation: input.target_ignores_reputation,
        },
        target_owner,
        catalogs,
    )
}

pub(super) fn resolve_faction_template_reaction(
    input: RepresentedFactionReactionInputLikeCpp,
    target_owner: Option<FactionReactionOwner<'_>>,
    catalogs: FactionReactionCatalogs<'_>,
) -> Result<ReputationRankLikeCpp, FactionReactionError> {
    use ReputationRankLikeCpp::{Friendly, Hostile, Neutral};

    let templates = faction_templates(catalogs)?;
    let Some(source_template) = templates.get(input.source_faction_template_id) else {
        return Ok(Neutral);
    };
    let Some(target_template) = templates.get(input.target_faction_template_id) else {
        return Ok(Neutral);
    };

    if input.target_has_player_owner {
        if source_template.is_contested_guard_faction_like_cpp()
            && input.target_player_contested_pvp
        {
            return Ok(Hostile);
        }

        let target_owner = target_owner.ok_or(FactionReactionError::UnknownTargetPlayerOwner)?;
        let manager = ReputationMgrLikeCpp::borrowing_like_cpp(target_owner.reputation);
        if let Some(rank) =
            manager.forced_rank_by_faction_id_like_cpp(u32::from(source_template.faction))
        {
            return Ok(rank);
        }

        if input.target_is_unit && !input.target_ignores_reputation {
            let factions = catalogs
                .faction_store
                .ok_or(FactionReactionError::UnknownFactionStore)?;
            if let Some(source_faction) = factions.get(u32::from(source_template.faction))
                && source_faction.can_have_reputation_like_cpp()
            {
                let friendship_reactions = friendship_reactions_for(
                    source_faction,
                    catalogs.friendship_rep_reaction_store,
                )?;
                let rank = manager.rank_for_faction_entry_like_cpp(
                    source_faction,
                    friendship_reactions,
                    target_owner.race,
                    target_owner.class,
                );
                if manager.is_at_war_with_faction_like_cpp(source_faction) {
                    return Ok(rank.min(Neutral));
                }
                return Ok(rank);
            }
        }
    }

    if source_template.is_hostile_to_like_cpp(target_template) {
        return Ok(Hostile);
    }
    if source_template.is_friendly_to_like_cpp(target_template) {
        return Ok(Friendly);
    }
    if target_template.is_friendly_to_like_cpp(source_template) {
        return Ok(Friendly);
    }
    if source_template.is_hostile_by_default_like_cpp() {
        return Ok(Hostile);
    }
    Ok(Neutral)
}

fn faction_templates<'a>(
    catalogs: FactionReactionCatalogs<'a>,
) -> Result<&'a wow_data::progression_rewards::FactionTemplateStore, FactionReactionError> {
    catalogs
        .faction_template_store
        .ok_or(FactionReactionError::UnknownFactionTemplateStore)
}

fn forced_rank(owner: FactionReactionOwner<'_>, faction_id: u32) -> Option<ReputationRankLikeCpp> {
    ReputationMgrLikeCpp::borrowing_like_cpp(owner.reputation)
        .forced_rank_by_faction_id_like_cpp(faction_id)
}

fn friendship_reactions_for<'a>(
    faction: &FactionEntry,
    store: Option<&'a wow_data::progression_rewards::FriendshipRepReactionStore>,
) -> Result<
    Option<&'a wow_data::progression_rewards::FriendshipRepReactionStore>,
    FactionReactionError,
> {
    if faction.friendship_rep_id == 0 {
        return Ok(None);
    }
    let store = store.ok_or(FactionReactionError::UnknownFriendshipReactionStore)?;
    if store
        .reactions_for_friendship_rep_like_cpp(faction.friendship_rep_id)
        .is_empty()
    {
        return Ok(None);
    }
    Ok(Some(store))
}
