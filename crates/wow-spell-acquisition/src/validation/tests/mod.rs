//! Acquisition validation fixtures construct the real owner's immutable plan.

use super::*;
use crate::test_fixtures::SpellAcquisitionPlanFixtureLikeCpp;

mod original;
mod replay;
mod actions;

fn snapshot(
    spells: Vec<PlayerSpellAcquisitionRowLikeCpp>,
) -> PlayerSpellAcquisitionSnapshotLikeCpp {
    PlayerSpellAcquisitionSnapshotLikeCpp {
        character_guid: Some(wow_core::ObjectGuid::create_player(1, 42)),
        spells,
        skills: Vec::new(),
        occupied_skill_slots: 0,
        overrides: Vec::new(),
        primary_profession_skill_ids: Vec::new(),
        non_durable_skill_tombstone_ids: Vec::new(),
        race: 1,
        class: 1,
        level: 80,
        lifecycle: PlayerAcquisitionLifecycleLikeCpp::InWorld,
        future_player_condition_resolutions: Vec::new(),
        cast_resolutions: BTreeMap::new(),
    }
}

fn spell(
    spell_id: u32,
    state: PlayerSpellPersistenceStateLikeCpp,
) -> PlayerSpellAcquisitionRowLikeCpp {
    PlayerSpellAcquisitionRowLikeCpp {
        spell_id,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
        trait_definition_id: None,
        state,
    }
}

fn direct_learn_actions(
    spell_id: u32,
    favorite: bool,
    suppress_messaging: bool,
) -> Vec<SpellAcquisitionPostCommitActionLikeCpp> {
    vec![
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id },
        SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id,
            favorite,
            suppress_messaging,
        },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { spell_id },
    ]
}

fn direct_learn_requirements(
    spell_id: u32,
    favorite: bool,
    suppress_messaging: bool,
) -> Vec<SpellAcquisitionPublicationRequirementLikeCpp> {
    vec![
        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id },
        SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
            spell_id,
            favorite,
            suppress_messaging,
        },
        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective { spell_id },
    ]
}

fn direct_learn_plan() -> (
    PlayerSpellAcquisitionSnapshotLikeCpp,
    SpellAcquisitionPlanLikeCpp,
) {
    let source = snapshot(Vec::new());
    let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
    let transition = PlannedSpellTransitionLikeCpp {
        spell_id: 100,
        before: None,
        after: Some(learned),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let plan = SpellAcquisitionPlanFixtureLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
        spell_transitions: vec![transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: direct_learn_requirements(100, false, false),
        profession_association_inputs: Vec::new(),
        post_commit_actions: direct_learn_actions(100, false, false),
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![learned]),
    }
    .build();
    (source, plan)
}

fn unchanged_plan(source: PlayerSpellAcquisitionSnapshotLikeCpp) -> SpellAcquisitionPlanLikeCpp {
    SpellAcquisitionPlanFixtureLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: Vec::new(),
        spell_transitions: Vec::new(),
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: Vec::new(),
        profession_association_inputs: source.skills.clone(),
        post_commit_actions: Vec::new(),
        diagnostics: Vec::new(),
        resulting_snapshot: source,
    }
    .build()
}

fn skill(skill_id: u32) -> PlayerSkillAcquisitionRowLikeCpp {
    PlayerSkillAcquisitionRowLikeCpp {
        skill_id,
        step: 1,
        value: 1,
        maximum: 75,
        profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
        state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
    }
}
