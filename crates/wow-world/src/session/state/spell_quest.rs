//! Existing Session substate definitions; authority and field contracts are unchanged.

use super::*;

/// The session's spell-side represented state: the cached spell-script id sets
/// the startup audit installs, the spell-acquisition authorities, the execute-log
/// effects and the offhand re-check switch, until the owning Player runtime and the
/// spell-acquisition module take them over.
pub(crate) struct SessionSpellState {
    pub(in crate::session) legacy_spell_script_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub(in crate::session) spell_linked_rejected_trigger_spell_ids_like_cpp:
        Option<Arc<BTreeSet<u32>>>,
    pub(in crate::session) spell_script_all_rank_root_spell_ids_like_cpp:
        Option<Arc<BTreeSet<u32>>>,
    /// Effective C++ spell-script hooks. These remain optional so a session
    /// constructed without the startup audit fails closed.
    pub(in crate::session) spell_script_exact_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    /// C++ `CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN` represented switch.
    pub(in crate::session) represented_offhand_check_at_spell_unlearn_like_cpp: bool,
    pub(in crate::session) represented_spell_execute_log_effects_like_cpp:
        Vec<wow_packet::packets::combat::SpellLogEffect>,
    pub(crate) spell_acquisition_cast_authority_like_cpp:
        Option<Arc<crate::spell_acquisition::SpellAcquisitionCastAuthorityLikeCpp>>,
    pub(crate) spell_acquisition_craft_authority_like_cpp:
        Option<Arc<crate::spell_acquisition::SpellAcquisitionCraftValidityAuthorityLikeCpp>>,
}

/// The session's quest-side represented state: the level-gap thresholds that
/// decide quest visibility, the completed-quest status updates and objective
/// progress the player owner drains, and the visibility refreshes those
/// transitions request.
pub(crate) struct SessionQuestState {
    pub(crate) min_quest_scaled_xp_ratio_like_cpp: u32,
    pub(crate) quest_high_level_hide_diff_like_cpp: u32,
    pub(crate) quest_low_level_hide_diff_like_cpp: u32,
    /// Evidence for represented `Player::CompleteQuest` status-update side effects.
    pub(crate) represented_quest_complete_status_updates_like_cpp:
        Vec<RepresentedQuestCompleteStatusUpdateLikeCpp>,
    pub(in crate::session) represented_quest_objective_progress_draining_like_cpp: bool,
    pub(in crate::session) represented_quest_objective_progress_events_like_cpp:
        VecDeque<RepresentedQuestObjectiveProgressEventLikeCpp>,
    /// Count of visibility refreshes requested by movement initialization.
    pub(in crate::session) movement_visibility_refresh_requests_like_cpp: u32,
}
