use super::super::{
    FactionReactionCatalogs, FactionReactionError, FactionReactionOwner,
    RepresentedFactionReactionInputLikeCpp, RepresentedGetReactionInputLikeCpp,
};
use super::{
    resolve_faction_reaction, resolve_faction_reaction_with_raid, resolve_faction_template_reaction,
};
use wow_constants::reputation::ReputationFlagsLikeCpp;
use wow_data::progression_rewards::{
    FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP, FactionEntry, FactionStore,
    FactionTemplateEntry, FactionTemplateStore, FriendshipRepReactionStore,
};
use wow_data::reputation::ReputationRankLikeCpp;
use wow_entities::{PlayerFactionStateLikeCpp, PlayerReputationStateLikeCpp};

fn faction_template(id: u32, faction: u16, flags: u16) -> FactionTemplateEntry {
    FactionTemplateEntry {
        id,
        faction,
        flags,
        faction_group: 0,
        friend_group: 0,
        enemy_group: 0,
        enemies: [0; 8],
        friend: [0; 8],
    }
}

fn templates(contested_target: bool) -> FactionTemplateStore {
    FactionTemplateStore::from_entries([
        faction_template(1, 72, 0),
        faction_template(
            2,
            930,
            if contested_target {
                FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP
            } else {
                0
            },
        ),
    ])
}

fn catalogs<'a>(
    factions: Option<&'a FactionStore>,
    faction_templates: Option<&'a FactionTemplateStore>,
    friendship_reactions: Option<&'a FriendshipRepReactionStore>,
) -> FactionReactionCatalogs<'a> {
    FactionReactionCatalogs::new(factions, faction_templates, friendship_reactions)
}

fn upper_input() -> RepresentedGetReactionInputLikeCpp {
    RepresentedGetReactionInputLikeCpp {
        self_faction_template_id: 1,
        target_faction_template_id: 2,
        same_object: false,
        attackable_by_summoner: false,
        same_charmer_or_owner_or_self: false,
        self_has_player_owner: false,
        target_has_player_owner: false,
        target_player_owner_is_current_session: false,
        target_owner_forced_rank_for_self: None,
        same_player_owner: false,
        duel_in_progress: false,
        same_raid: false,
        self_unit_player_controlled: false,
        target_unit_player_controlled: false,
        self_ffa_pvp: false,
        target_ffa_pvp: false,
        self_ignores_reputation: false,
        target_ignores_reputation: false,
        target_is_unit: true,
        self_player_contested_pvp: false,
        target_player_contested_pvp: false,
    }
}

fn lower_input() -> RepresentedFactionReactionInputLikeCpp {
    RepresentedFactionReactionInputLikeCpp {
        source_faction_template_id: 1,
        target_faction_template_id: 2,
        target_has_player_owner: true,
        target_player_owner_is_current_session: false,
        target_player_contested_pvp: false,
        target_is_unit: true,
        target_ignores_reputation: false,
    }
}

fn owner(state: &PlayerReputationStateLikeCpp) -> FactionReactionOwner<'_> {
    FactionReactionOwner::new(state, 1, 1)
}

fn reputation_state(
    faction_id: u32,
    reputation_list_id: u32,
    standing: i32,
    at_war: bool,
) -> PlayerReputationStateLikeCpp {
    let mut state = PlayerReputationStateLikeCpp::default();
    state.insert_faction_like_cpp(PlayerFactionStateLikeCpp {
        faction_id,
        reputation_list_id,
        standing,
        flags: if at_war {
            ReputationFlagsLikeCpp::AT_WAR
        } else {
            ReputationFlagsLikeCpp::NONE
        },
        ..Default::default()
    });
    state
}

#[test]
fn faction_reaction_uses_source_forced_rank_before_target_owner_forced_rank() {
    let templates = templates(false);
    let mut source_state = PlayerReputationStateLikeCpp::default();
    source_state.set_forced_reaction_like_cpp(930, Some(ReputationRankLikeCpp::Hostile));
    let mut target_state = PlayerReputationStateLikeCpp::default();
    target_state.set_forced_reaction_like_cpp(72, Some(ReputationRankLikeCpp::Friendly));

    let mut input = upper_input();
    input.self_has_player_owner = true;
    input.target_has_player_owner = true;
    input.target_owner_forced_rank_for_self = Some(ReputationRankLikeCpp::Revered);
    assert_eq!(
        resolve_faction_reaction(
            input,
            Some(owner(&source_state)),
            Some(owner(&target_state)),
            catalogs(None, Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Hostile)
    );

    assert_eq!(
        resolve_faction_reaction(
            input,
            None,
            Some(owner(&target_state)),
            catalogs(None, Some(&templates), None),
        ),
        Err(FactionReactionError::UnknownSourcePlayerOwner)
    );

    input.self_has_player_owner = false;
    input.target_owner_forced_rank_for_self = None;
    assert_eq!(
        resolve_faction_reaction(
            input,
            None,
            Some(owner(&target_state)),
            catalogs(None, Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Friendly)
    );

    assert_eq!(
        resolve_faction_reaction(input, None, None, catalogs(None, Some(&templates), None)),
        Err(FactionReactionError::UnknownTargetPlayerOwner)
    );

    input.target_owner_forced_rank_for_self = Some(ReputationRankLikeCpp::Revered);
    assert_eq!(
        resolve_faction_reaction(input, None, None, catalogs(None, Some(&templates), None),),
        Ok(ReputationRankLikeCpp::Revered)
    );
}

#[test]
fn faction_reaction_lower_rule_uses_target_reputation_owner() {
    let templates = templates(false);
    let factions = FactionStore::from_entries([FactionEntry::for_test_like_cpp(72, 1)]);
    let source_state = reputation_state(72, 1, -42_000, true);
    let target_state = reputation_state(72, 1, 42_000, false);
    let mut input = upper_input();
    input.target_has_player_owner = true;

    assert_eq!(
        resolve_faction_reaction(
            input,
            Some(owner(&source_state)),
            Some(owner(&target_state)),
            catalogs(Some(&factions), Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Exalted)
    );

    let no_target_reputation = reputation_state(72, 1, -42_000, false);
    assert_eq!(
        resolve_faction_reaction(
            input,
            Some(owner(&source_state)),
            Some(owner(&no_target_reputation)),
            catalogs(Some(&factions), Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Hated)
    );
}

#[test]
fn faction_reaction_player_controlled_branch_uses_source_contested_flag() {
    let templates = templates(true);
    let factions = FactionStore::from_entries([FactionEntry::for_test_like_cpp(930, 2)]);
    let source_state = PlayerReputationStateLikeCpp::default();
    let mut input = upper_input();
    input.self_has_player_owner = true;
    input.self_unit_player_controlled = true;
    input.target_unit_player_controlled = true;
    input.target_player_contested_pvp = true;

    assert_eq!(
        resolve_faction_reaction(
            input,
            Some(owner(&source_state)),
            None,
            catalogs(Some(&factions), Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Friendly)
    );

    input.self_player_contested_pvp = true;
    input.target_player_contested_pvp = false;
    assert_eq!(
        resolve_faction_reaction(
            input,
            Some(owner(&source_state)),
            None,
            catalogs(Some(&factions), Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Hostile)
    );
}

#[test]
fn faction_reaction_lower_rule_keeps_target_contested_flag() {
    let templates = FactionTemplateStore::from_entries([
        faction_template(1, 72, FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP),
        faction_template(2, 930, 0),
    ]);
    let input = lower_input();

    let mut target_contested = input;
    target_contested.target_player_contested_pvp = true;
    assert_eq!(
        resolve_faction_template_reaction(
            target_contested,
            None,
            catalogs(None, Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Hostile)
    );

    let target_state = PlayerReputationStateLikeCpp::default();
    let mut source_contested = input;
    source_contested.target_player_contested_pvp = false;
    source_contested.target_is_unit = false;
    assert_eq!(
        resolve_faction_template_reaction(
            source_contested,
            Some(owner(&target_state)),
            catalogs(None, Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Neutral)
    );
}

#[test]
fn faction_reaction_distinguishes_missing_rows_from_unknown_catalogs() {
    let input = upper_input();
    let empty_templates = FactionTemplateStore::from_entries(Vec::<FactionTemplateEntry>::new());
    assert_eq!(
        resolve_faction_reaction(
            input,
            None,
            None,
            catalogs(None, Some(&empty_templates), None)
        ),
        Ok(ReputationRankLikeCpp::Neutral)
    );
    let partial_templates = FactionTemplateStore::from_entries([faction_template(2, 930, 0)]);
    assert_eq!(
        resolve_faction_reaction(
            input,
            None,
            None,
            catalogs(None, Some(&partial_templates), None)
        ),
        Ok(ReputationRankLikeCpp::Neutral)
    );
    assert_eq!(
        resolve_faction_reaction(input, None, None, catalogs(None, None, None)),
        Err(FactionReactionError::UnknownFactionTemplateStore)
    );

    let source_state = PlayerReputationStateLikeCpp::default();
    let mut player_controlled = input;
    player_controlled.self_has_player_owner = true;
    player_controlled.target_has_player_owner = true;
    player_controlled.self_unit_player_controlled = true;
    player_controlled.target_unit_player_controlled = true;
    player_controlled.same_player_owner = true;
    assert_eq!(
        resolve_faction_reaction(
            player_controlled,
            Some(owner(&source_state)),
            None,
            catalogs(None, Some(&empty_templates), None),
        ),
        Ok(ReputationRankLikeCpp::Friendly)
    );

    let templates = templates(false);
    let target_state = PlayerReputationStateLikeCpp::default();
    assert_eq!(
        resolve_faction_template_reaction(
            lower_input(),
            Some(owner(&target_state)),
            catalogs(None, Some(&templates), None),
        ),
        Err(FactionReactionError::UnknownFactionStore)
    );
    let empty_factions = FactionStore::from_entries(Vec::<FactionEntry>::new());
    assert_eq!(
        resolve_faction_template_reaction(
            lower_input(),
            Some(owner(&target_state)),
            catalogs(Some(&empty_factions), Some(&templates), None),
        ),
        Ok(ReputationRankLikeCpp::Neutral)
    );

    let mut decisive = input;
    decisive.same_object = true;
    assert_eq!(
        resolve_faction_reaction(decisive, None, None, catalogs(None, None, None)),
        Ok(ReputationRankLikeCpp::Friendly)
    );
}

#[test]
fn faction_reaction_preserves_force_duel_raid_and_ffa_priority() {
    let templates = templates(false);
    let mut forced_state = PlayerReputationStateLikeCpp::default();
    forced_state.set_forced_reaction_like_cpp(930, Some(ReputationRankLikeCpp::Friendly));
    let mut input = upper_input();
    input.self_has_player_owner = true;
    input.target_has_player_owner = true;
    input.self_unit_player_controlled = true;
    input.target_unit_player_controlled = true;
    input.duel_in_progress = true;
    input.same_raid = true;
    input.self_ffa_pvp = true;
    input.target_ffa_pvp = true;

    let call = |input, source_state: &PlayerReputationStateLikeCpp| {
        resolve_faction_reaction(
            input,
            Some(owner(source_state)),
            None,
            catalogs(None, Some(&templates), None),
        )
    };
    assert_eq!(
        call(input, &forced_state),
        Ok(ReputationRankLikeCpp::Friendly)
    );

    let source_state = PlayerReputationStateLikeCpp::default();
    input.same_player_owner = true;
    assert_eq!(
        call(input, &source_state),
        Ok(ReputationRankLikeCpp::Friendly)
    );

    input.same_player_owner = false;
    assert_eq!(
        call(input, &source_state),
        Ok(ReputationRankLikeCpp::Hostile)
    );

    input.duel_in_progress = false;
    assert_eq!(
        call(input, &source_state),
        Ok(ReputationRankLikeCpp::Friendly)
    );

    input.same_raid = false;
    assert_eq!(
        call(input, &source_state),
        Ok(ReputationRankLikeCpp::Hostile)
    );
}

#[test]
fn faction_reaction_uses_stock_rank_for_known_empty_friendship_reactions() {
    let templates = templates(false);
    let mut faction = FactionEntry::for_test_like_cpp(72, 1);
    faction.friendship_rep_id = 7;
    let factions = FactionStore::from_entries([faction]);
    let empty_friendships = FriendshipRepReactionStore::from_entries(Vec::<
        wow_data::progression_rewards::FriendshipRepReactionEntry,
    >::new());
    let target_state = reputation_state(72, 1, 3_500, false);

    assert_eq!(
        resolve_faction_template_reaction(
            lower_input(),
            Some(owner(&target_state)),
            catalogs(Some(&factions), Some(&templates), Some(&empty_friendships)),
        ),
        Ok(ReputationRankLikeCpp::Friendly)
    );
    assert_eq!(
        resolve_faction_template_reaction(
            lower_input(),
            Some(owner(&target_state)),
            catalogs(Some(&factions), Some(&templates), None),
        ),
        Err(FactionReactionError::UnknownFriendshipReactionStore)
    );
}

#[test]
fn faction_reaction_resolves_raid_only_at_its_source_order_position() {
    use std::cell::Cell;

    let templates = templates(false);
    let empty_templates = FactionTemplateStore::from_entries(Vec::<FactionTemplateEntry>::new());
    let mut source_state = PlayerReputationStateLikeCpp::default();
    source_state.set_forced_reaction_like_cpp(930, Some(ReputationRankLikeCpp::Friendly));
    let target_state = PlayerReputationStateLikeCpp::default();
    let mut forced = upper_input();
    forced.self_has_player_owner = true;
    forced.target_has_player_owner = true;
    forced.self_unit_player_controlled = true;
    forced.target_unit_player_controlled = true;
    let forced_raid_calls = Cell::new(0);
    assert_eq!(
        resolve_faction_reaction_with_raid(
            forced,
            Some(owner(&source_state)),
            Some(owner(&target_state)),
            catalogs(None, Some(&templates), None),
            || {
                forced_raid_calls.set(forced_raid_calls.get() + 1);
                Err(FactionReactionError::UnknownRaidRelation)
            },
        ),
        Ok(ReputationRankLikeCpp::Friendly)
    );
    assert_eq!(forced_raid_calls.get(), 0);

    let mut duel = upper_input();
    duel.self_has_player_owner = true;
    duel.target_has_player_owner = true;
    duel.self_unit_player_controlled = true;
    duel.target_unit_player_controlled = true;
    duel.duel_in_progress = true;
    let duel_raid_calls = Cell::new(0);
    assert_eq!(
        resolve_faction_reaction_with_raid(
            duel,
            Some(owner(&target_state)),
            Some(owner(&target_state)),
            catalogs(None, Some(&templates), None),
            || {
                duel_raid_calls.set(duel_raid_calls.get() + 1);
                Err(FactionReactionError::UnknownRaidRelation)
            },
        ),
        Ok(ReputationRankLikeCpp::Hostile)
    );
    assert_eq!(duel_raid_calls.get(), 0);

    let mut controlled_ffa = upper_input();
    controlled_ffa.self_has_player_owner = true;
    controlled_ffa.target_has_player_owner = true;
    controlled_ffa.self_unit_player_controlled = true;
    controlled_ffa.target_unit_player_controlled = true;
    controlled_ffa.self_ffa_pvp = true;
    controlled_ffa.target_ffa_pvp = true;
    assert_eq!(
        resolve_faction_reaction_with_raid(
            controlled_ffa,
            Some(owner(&target_state)),
            Some(owner(&target_state)),
            catalogs(None, Some(&empty_templates), None),
            || Err(FactionReactionError::UnknownRaidRelation),
        ),
        Err(FactionReactionError::UnknownRaidRelation)
    );
    assert_eq!(
        resolve_faction_reaction_with_raid(
            controlled_ffa,
            Some(owner(&target_state)),
            Some(owner(&target_state)),
            catalogs(None, Some(&empty_templates), None),
            || Ok(true),
        ),
        Ok(ReputationRankLikeCpp::Friendly)
    );
}
