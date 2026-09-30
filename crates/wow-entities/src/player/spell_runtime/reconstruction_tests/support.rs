//! One canonical Player, real metadata stores, and productive operation drivers.

use super::*;

mod catalogs;
mod owner;
mod operations;
mod unlearn;

pub(super) use catalogs::*;

pub(super) type RepresentedPlayerSkillLikeCpp = PlayerSkillRecord;
pub(super) type RepresentedPlayerSkillStateLikeCpp = PlayerSkillLoadState;

pub(super) struct TestPlayer {
    pub player: Player,
    pub chains: Option<Arc<wow_data::SpellChainStoreLikeCpp>>,
    pub learned: Option<Arc<wow_data::SpellLearnSpellStoreLikeCpp>>,
    pub learn_skills: Option<Arc<wow_data::SpellLearnSkillStoreLikeCpp>>,
    pub skills: Option<Arc<wow_data::SkillStore>>,
    pub lines: Option<Arc<wow_data::SkillLineStore>>,
    pub tiers: Option<Arc<wow_data::SkillTiersStoreLikeCpp>>,
    pub trace: Vec<LearnedSkillStep>,
}

pub(super) fn make_session() -> (TestPlayer, (), ()) {
    (TestPlayer {
        player: Player::new(None, false),
        chains: None,
        learned: None,
        learn_skills: None,
        skills: None,
        lines: None,
        tiers: None,
        trace: Vec::new(),
    }, (), ())
}

pub(super) fn node_fact(node: wow_data::SpellLearnSkillNodeLikeCpp) -> LearnedSkillNode {
    LearnedSkillNode {
        skill_id: node.skill, step: node.step, value: node.value, max_value: node.maxvalue,
    }
}
