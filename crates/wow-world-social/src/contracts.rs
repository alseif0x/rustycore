use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAwayModeLikeCpp {
    Afk,
    Dnd,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ChatFloodThrottleDataLikeCpp {
    pub(crate) time: i64,
    pub(crate) count: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum ChatFloodThrottleIndexLikeCpp {
    Regular = 0,
    Addon = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedSignPetitionLikeCpp {
    pub petition_guid: ObjectGuid,
    pub choice: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedDeclinePetitionLikeCpp {
    pub petition_guid: ObjectGuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQueryPetitionLikeCpp {
    pub petition_id: u32,
    pub item_guid: ObjectGuid,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedSilencePartyTalkerLikeCpp {
    pub target: ObjectGuid,
    pub silent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedCalendarCommunityInviteLikeCpp {
    pub guild_id: u64,
    pub min_level: u8,
    pub max_level: u8,
    pub max_rank_order: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedCalendarAddEventLikeCpp {
    pub guild_id: Option<u64>,
    pub club_id: u64,
    pub event_type: u8,
    pub texture_id: i32,
    pub time_packed: u32,
    pub flags: u32,
    pub invite_count: usize,
    pub title: String,
    pub description: String,
    pub max_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedCalendarRemoveEventLikeCpp {
    pub event_id: u64,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedCanDuelSpellCastLikeCpp {
    pub target_guid: ObjectGuid,
    pub spell_id: u32,
    pub to_the_death: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedDuelRequestedLikeCpp {
    pub target_guid: ObjectGuid,
    pub arbiter_guid: ObjectGuid,
    pub gameobject_entry: u32,
    pub to_the_death: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedDuelAcceptedLikeCpp {
    pub opponent_guid: ObjectGuid,
    pub arbiter_guid: ObjectGuid,
    pub countdown_ms: u32,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedDuelCancelOutcomeLikeCpp {
    Interrupted,
    Surrendered,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedDuelCancelledLikeCpp {
    pub opponent_guid: ObjectGuid,
    pub outcome: RepresentedDuelCancelOutcomeLikeCpp,
    pub beg_spell_id: Option<u32>,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedForceDeselectLikeCpp {
    pub caster_guid: ObjectGuid,
    pub visibility_range_yards: u32,
    pub break_target_packet_bytes: Vec<u8>,
    pub clear_target_packet_bytes: Vec<u8>,
    pub hostile_visible_fanout_unrepresented: bool,
    pub attacker_pet_attack_stop_unrepresented: bool,
}
