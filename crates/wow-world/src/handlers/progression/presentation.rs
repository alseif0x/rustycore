use wow_packet::packets::reputation::{
    FactionStandingData, ForcedReaction, InitializeFactions, SetFactionStanding,
    SetForcedReactions,
};
use wow_progression::mgr::{
    FactionStandingUpdateLikeCpp, ForcedReactionsStateLikeCpp, InitializeFactionStateLikeCpp,
};

pub(crate) fn initialize_factions_packet_like_cpp(
    state: InitializeFactionStateLikeCpp,
) -> InitializeFactions {
    InitializeFactions {
        faction_standings: state.faction_standings,
        faction_has_bonus: state.faction_has_bonus,
        faction_flags: state.faction_flags,
    }
}

pub(crate) fn set_faction_standing_packet_like_cpp(
    update: FactionStandingUpdateLikeCpp,
) -> SetFactionStanding {
    SetFactionStanding {
        bonus_from_achievement_system: update.bonus_from_achievement_system,
        faction: update
            .faction
            .into_iter()
            .map(|standing| FactionStandingData {
                index: standing.index,
                standing: standing.standing,
            })
            .collect(),
        show_visual: update.show_visual,
    }
}

pub(crate) fn set_forced_reactions_packet_like_cpp(
    state: ForcedReactionsStateLikeCpp,
) -> SetForcedReactions {
    SetForcedReactions {
        reactions: state
            .reactions
            .into_iter()
            .map(|reaction| ForcedReaction {
                faction: reaction.faction,
                reaction: reaction.reaction,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_constants::reputation::FACTION_COUNT_LIKE_CPP;
    use wow_packet::ServerPacket;
    use wow_progression::mgr::{FactionStandingStateLikeCpp, ForcedReactionStateLikeCpp};

    #[test]
    fn initialize_factions_presentation_matches_packet_writer_and_full_size() {
        let mut state = InitializeFactionStateLikeCpp::default();
        state.faction_flags[2] = 0x1234;
        state.faction_standings[2] = -123;
        state.faction_has_bonus[2] = true;

        let mut expected = InitializeFactions::default();
        expected.faction_flags[2] = 0x1234;
        expected.faction_standings[2] = -123;
        expected.faction_has_bonus[2] = true;

        let bytes = initialize_factions_packet_like_cpp(state).to_bytes();
        assert_eq!(bytes, expected.to_bytes());
        assert_eq!(
            bytes.len(),
            2 + FACTION_COUNT_LIKE_CPP * 6 + FACTION_COUNT_LIKE_CPP / 8
        );
    }

    #[test]
    fn faction_standing_presentation_preserves_primary_secondary_wire_order() {
        let update = FactionStandingUpdateLikeCpp {
            bonus_from_achievement_system: 1.5,
            faction: vec![
                FactionStandingStateLikeCpp {
                    index: 7,
                    standing: 3000,
                },
                FactionStandingStateLikeCpp {
                    index: 8,
                    standing: -42000,
                },
            ],
            show_visual: true,
        };
        let expected = SetFactionStanding {
            bonus_from_achievement_system: 1.5,
            faction: vec![
                FactionStandingData {
                    index: 7,
                    standing: 3000,
                },
                FactionStandingData {
                    index: 8,
                    standing: -42000,
                },
            ],
            show_visual: true,
        };

        assert_eq!(
            set_faction_standing_packet_like_cpp(update).to_bytes(),
            expected.to_bytes()
        );
    }

    #[test]
    fn forced_reactions_presentation_preserves_btree_projection_order() {
        let state = ForcedReactionsStateLikeCpp {
            reactions: vec![
                ForcedReactionStateLikeCpp {
                    faction: 72,
                    reaction: 5,
                },
                ForcedReactionStateLikeCpp {
                    faction: 930,
                    reaction: -1,
                },
            ],
        };
        let expected = SetForcedReactions {
            reactions: vec![
                ForcedReaction {
                    faction: 72,
                    reaction: 5,
                },
                ForcedReaction {
                    faction: 930,
                    reaction: -1,
                },
            ],
        };

        assert_eq!(
            set_forced_reactions_packet_like_cpp(state).to_bytes(),
            expected.to_bytes()
        );
    }
}
