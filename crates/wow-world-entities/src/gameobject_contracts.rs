use std::time::Instant;

use wow_constants::Team;
use wow_core::{ObjectGuid, Position};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepresentedGameObjectUseEffect {
    UseRejectedNoDamageImmune {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    BattlegroundObjectUseRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        reason: RepresentedBattlegroundObjectUseRejection,
    },
    RemoveMountedAuras {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    RemoveStealthOrInvisibilityAuras {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    ClearPlayerTalkMenus {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    GossipHelloAi {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        handled: bool,
    },
    CooldownStarted {
        gameobject_guid: ObjectGuid,
        cooldown_secs: u32,
    },
    CooldownRejected {
        gameobject_guid: ObjectGuid,
    },
    TrapBombSpellCast {
        gameobject_guid: ObjectGuid,
        spell_id: u32,
    },
    TrapTargetActivated {
        gameobject_guid: ObjectGuid,
        target_guid: ObjectGuid,
    },
    TrapTargetSpellCast {
        gameobject_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_id: u32,
        original_caster_guid: ObjectGuid,
    },
    DoorOrButtonUsed {
        gameobject_guid: ObjectGuid,
        user_guid: ObjectGuid,
        restore_time_ms: u32,
        go_state: wow_entities::GoState,
    },
    DoorOrButtonRejectedNotReady {
        gameobject_guid: ObjectGuid,
    },
    #[allow(dead_code)]
    DoorOrButtonReset {
        gameobject_guid: ObjectGuid,
        go_state: wow_entities::GoState,
    },
    TriggerCinematic {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        cinematic_id: u32,
    },
    ShowPageText {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        page_id: u32,
    },
    SendGossip {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gossip_id: u32,
    },
    ReportUseAi {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        handled: bool,
    },
    TriggerGameEvent {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        event_id: u32,
    },
    TriggerLinkedTrap {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        trap_entry: u32,
    },
    KillCreditGo {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        entry: u32,
    },
    GooberQuestGateRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        quest_id: u32,
    },
    GooberSetGoStateForPlayer {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        go_state: wow_entities::GoState,
    },
    GooberDespawnForPlayer {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        despawn_secs: u32,
    },
    GameObjectPerPlayerStateExpired {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        despawned: bool,
        needs_state_update: bool,
    },
    GooberUsed {
        gameobject_guid: ObjectGuid,
        user_guid: ObjectGuid,
        custom_anim: u32,
        auto_close_ms: u32,
        go_state: Option<wow_entities::GoState>,
    },
    #[allow(dead_code)]
    GooberLinkedTrapDespawn {
        gameobject_guid: ObjectGuid,
        trap_entry: u32,
    },
    #[allow(dead_code)]
    GameObjectLinkedTrapDespawn {
        gameobject_guid: ObjectGuid,
        trap_entry: u32,
    },
    #[allow(dead_code)]
    GooberUniqueUserSpell {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spell_id: u32,
    },
    #[allow(dead_code)]
    GooberCleared {
        gameobject_guid: ObjectGuid,
        loot_state: wow_entities::LootState,
        go_state: Option<wow_entities::GoState>,
    },
    ChairUsed {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        slot: u32,
        teleport_position: Position,
        stand_state: u32,
    },
    ChairNoFreeSlot {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    BarberChairUsed {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        customization_scope: u32,
        teleport_position: Position,
        stand_state: u32,
        sit_anim_kit: u32,
    },
    UiLinkOpened {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ui_link_type: u32,
        interaction_type: i32,
    },
    ItemForgeUsed {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        condition_id: u32,
        forge_type: u32,
    },
    CapturePointAssaultRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        capture_time_ms: u32,
        world_state_id: u32,
        contested_event_horde: u32,
        contested_event_alliance: u32,
    },
    CapturePointAssaultAi {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        handled: bool,
    },
    CapturePointUpdated {
        gameobject_guid: ObjectGuid,
        state: RepresentedCapturePointStateLikeCpp,
        broadcast_text_id: u32,
        event_id: u32,
        world_state_id: u32,
        spell_visual_id: u32,
        custom_anim: u32,
        assault_timer_ms: u32,
    },
    BattlegroundFlagStandClicked {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        pickup_spell_id: u32,
        return_aura_id: u32,
        return_spell_id: u32,
    },
    BattlegroundFlagDropClicked {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        click_target: BattlegroundFlagDropClickTarget,
        event_id: u32,
        pickup_spell_id: u32,
        expire_duration_ms: u32,
    },
    GameObjectDeleted {
        gameobject_guid: ObjectGuid,
    },
    NewFlagPickupRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        pickup_spell_id: u32,
        expire_duration_ms: u32,
        respawn_time_ms: u32,
        flag_drop_entry: u32,
        exclusive_category: i32,
        world_state_id: u32,
        return_on_defender_interact: bool,
    },
    NewFlagDropInteracted {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spawn_vignette_id: u32,
    },
    NewFlagOwnerStateRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        state: RepresentedNewFlagStateRequest,
    },
    RitualWaitingForParticipants {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        unique_user_count: u32,
        casters_required: u32,
    },
    RitualCasterTargetSpellRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spell_id: u32,
        target_count: u32,
    },
    RitualCasterTargetSpellCast {
        gameobject_guid: ObjectGuid,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_id: u32,
        triggered: bool,
    },
    RitualCompleted {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        final_spell_id: u32,
        triggered: bool,
        persistent: bool,
        unique_user_count: u32,
    },
    MeetingStoneSummonRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        gameobject_entry: u32,
        spell_id: u32,
        area_id: u32,
        prevent_unfriendly_outside_instances: bool,
    },
    MeetingStoneTargetRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        target_guid: Option<ObjectGuid>,
    },
    MeetingStoneLevelRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        player_level: u8,
        target_level: u8,
        required_level: i32,
    },
    GameObjectPostUseSpellMissing {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        spell_id: u32,
        go_type: u32,
        spell_lookup_difficulty_id: u8,
    },
    OutdoorPvpCustomSpellRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        spell_id: u32,
        go_type: u32,
        spell_lookup_difficulty_id: u8,
        spell_info_missing: bool,
    },
    GameObjectPostUseSpellCast {
        gameobject_guid: ObjectGuid,
        target_guid: ObjectGuid,
        caster_guid: ObjectGuid,
        spell_id: u32,
        triggered: bool,
        caster: RepresentedGameObjectSpellCaster,
        spell_lookup_difficulty_id: u8,
    },
    FishingNodeOwnerRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    },
    FishingNodeActivated {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    FishingBobberReady {
        gameobject_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    },
    FishingSkillUpdated {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    FishingLootRoll {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        player_fishing_level: i32,
        area_fishing_level: i32,
        chance: i32,
        roll: i32,
    },
    FishingHoleDelegated {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        fishing_hole_guid: ObjectGuid,
    },
    FishingLootRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot_type: u8,
    },
    FishNotHooked {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    FishingHoleCatchCriteriaUpdated {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
    },
    FinishChanneledSpell {
        player_guid: ObjectGuid,
    },
    SpellcasterPartyOnlyRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    GameObjectUseCountIncremented {
        gameobject_guid: ObjectGuid,
        use_count: u32,
    },
    GameObjectChargesDepleted {
        gameobject_guid: ObjectGuid,
        max_charges: u32,
        loot_state: wow_entities::LootState,
    },
    GameObjectJustDeactivatedCleared {
        gameobject_guid: ObjectGuid,
        deleted: bool,
    },
    CastSpell {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spell_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlegroundFlagDropClickTarget {
    None,
    WarsongGulch,
    EyeOfTheStorm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedBattlegroundObjectUseRejection {
    UnfriendlyFaction,
    RecentlyDroppedFlag,
    DamageImmune,
    Dead,
    NotInBattleground,
    Vehicle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RepresentedNewFlagStateRequest {
    InBase,
    Taken,
    Dropped,
    Respawning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RepresentedCapturePointStateLikeCpp {
    Neutral,
    ContestedHorde,
    ContestedAlliance,
    HordeCaptured,
    AllianceCaptured,
}

impl RepresentedCapturePointStateLikeCpp {
    pub fn custom_anim_and_spell_visual_like_cpp(
        self,
        source: wow_entities::CapturePointUseSource,
    ) -> (u32, u32) {
        match self {
            Self::Neutral => (0, source.spell_visual_ids[0]),
            Self::ContestedHorde => (1, source.spell_visual_ids[1]),
            Self::ContestedAlliance => (2, source.spell_visual_ids[2]),
            Self::HordeCaptured => (3, source.spell_visual_ids[3]),
            Self::AllianceCaptured => (4, source.spell_visual_ids[4]),
        }
    }

    pub fn packet_state_like_cpp(self) -> u8 {
        match self {
            Self::Neutral => 1,
            Self::ContestedHorde => 2,
            Self::ContestedAlliance => 3,
            Self::HordeCaptured => 4,
            Self::AllianceCaptured => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedGameObjectSpellCaster {
    User,
    GameObject,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedGameObjectUseState {
    pub loot_state: Option<wow_entities::LootState>,
    pub loot_state_unit_guid: wow_core::ObjectGuid,
    pub owner_guid: Option<wow_core::ObjectGuid>,
    pub ritual_owner_guid: Option<wow_core::ObjectGuid>,
    pub owner_in_combat: Option<bool>,
    pub owner_current_channeled_spell_active: Option<bool>,
    pub go_state: Option<wow_entities::GoState>,
    pub prev_go_state: Option<wow_entities::GoState>,
    pub gameobject_flags: u32,
    pub gameobject_override_flags: Option<u32>,
    pub dynamic_flags: u32,
    pub despawn_delay_secs: Option<u32>,
    pub despawn_delay_until: Option<Instant>,
    pub respawn_delay_secs: Option<u32>,
    pub respawn_until: Option<Instant>,
    pub spawned_by_default: Option<bool>,
    pub per_player_despawn_secs: Option<u32>,
    pub per_player_despawn_until: Option<Instant>,
    pub per_player_state_player_guid: Option<wow_core::ObjectGuid>,
    pub per_player_go_state: Option<wow_entities::GoState>,
    pub per_player_go_state_until: Option<Instant>,
    pub personal_loot_uses: u32,
    pub use_count: u32,
    pub max_charges: Option<u32>,
    pub unique_users: Vec<wow_core::ObjectGuid>,
    pub chair_slots: Vec<Option<wow_core::ObjectGuid>>,
    pub chest_restock_time_secs: Option<u32>,
    pub chest_restock_until: Option<Instant>,
    pub chest_consumable: Option<bool>,
    pub chest_loot_source: Option<wow_entities::GameObjectLootSource>,
    pub chest_personal_loot_id: Option<u32>,
    pub gathering_node_loot_id: Option<u32>,
    pub map_id: Option<u16>,
    pub zone_id: Option<u32>,
    pub area_id: Option<u32>,
    pub position: Option<wow_core::Position>,
    pub go_anim_progress: u8,
    pub despawn_at_action: bool,
    pub display_id: Option<u32>,
    pub scale: f32,
    pub rotation: [f32; 4],
    pub go_type: Option<u8>,
    pub faction_template: Option<u32>,
    pub interact_radius_override: Option<u32>,
    /// Immutable template evidence for C++
    /// `GameObjectTemplate::IconName != "Point"`.
    pub icon_name_allows_interaction_like_cpp: Option<bool>,
    pub condition_id1: Option<u32>,
    pub lock_id: Option<u32>,
    pub fishing_hole_max_opens: Option<u32>,
    pub fishing_hole_radius: Option<f32>,
    pub fishing_area_level: Option<i32>,
    pub player_fishing_level: Option<i32>,
    pub fishing_roll: Option<i32>,
    pub nearby_fishing_hole_guid: Option<wow_core::ObjectGuid>,
    pub fishing_bobber_ready_at: Option<Instant>,
    pub linked_trap_entry: Option<u32>,
    pub linked_trap_guid: Option<wow_core::ObjectGuid>,
    pub trap_use_source: Option<wow_entities::TrapUseSource>,
    pub trap_target_guid: Option<wow_core::ObjectGuid>,
    pub goober_use_source: Option<wow_entities::GooberUseSource>,
    pub capture_point_state: Option<RepresentedCapturePointStateLikeCpp>,
    pub capture_point_source: Option<wow_entities::CapturePointUseSource>,
    pub capture_point_last_team_capture: Team,
    pub capture_point_assault_until: Option<Instant>,
    pub new_flag_state: Option<RepresentedNewFlagStateRequest>,
    pub new_flag_carrier_guid: Option<wow_core::ObjectGuid>,
    pub new_flag_taken_from_base_game_time_ms: Option<u32>,
    pub new_flag_respawn_until: Option<Instant>,
    pub new_flag_return_on_defender_interact: Option<bool>,
    pub new_flag_pickup_spell_id: Option<u32>,
    pub new_flag_entry: Option<u32>,
    pub report_use_ai_returns_true: bool,
    pub gossip_hello_ai_returns_true: bool,
    pub capture_point_assault_ai_returns_true: bool,
    pub cooldown_until: Option<Instant>,
}

impl Default for RepresentedGameObjectUseState {
    fn default() -> Self {
        Self {
            loot_state: None,
            loot_state_unit_guid: wow_core::ObjectGuid::EMPTY,
            owner_guid: None,
            ritual_owner_guid: None,
            owner_in_combat: None,
            owner_current_channeled_spell_active: None,
            go_state: None,
            prev_go_state: None,
            gameobject_flags: 0,
            gameobject_override_flags: None,
            dynamic_flags: 0,
            despawn_delay_secs: None,
            despawn_delay_until: None,
            respawn_delay_secs: None,
            respawn_until: None,
            spawned_by_default: None,
            per_player_despawn_secs: None,
            per_player_despawn_until: None,
            per_player_state_player_guid: None,
            per_player_go_state: None,
            per_player_go_state_until: None,
            personal_loot_uses: 0,
            use_count: 0,
            max_charges: None,
            unique_users: Vec::new(),
            chair_slots: Vec::new(),
            chest_restock_time_secs: None,
            chest_restock_until: None,
            chest_consumable: None,
            chest_loot_source: None,
            chest_personal_loot_id: None,
            gathering_node_loot_id: None,
            map_id: None,
            zone_id: None,
            area_id: None,
            position: None,
            go_anim_progress: 255,
            despawn_at_action: false,
            display_id: None,
            scale: 1.0,
            rotation: [0.0, 0.0, 0.0, 1.0],
            go_type: None,
            faction_template: None,
            interact_radius_override: None,
            icon_name_allows_interaction_like_cpp: None,
            condition_id1: None,
            lock_id: None,
            fishing_hole_max_opens: None,
            fishing_hole_radius: None,
            fishing_area_level: None,
            player_fishing_level: None,
            fishing_roll: None,
            nearby_fishing_hole_guid: None,
            fishing_bobber_ready_at: None,
            linked_trap_entry: None,
            linked_trap_guid: None,
            trap_use_source: None,
            trap_target_guid: None,
            goober_use_source: None,
            capture_point_state: None,
            capture_point_source: None,
            capture_point_last_team_capture: Team::Other,
            capture_point_assault_until: None,
            new_flag_state: None,
            new_flag_carrier_guid: None,
            new_flag_taken_from_base_game_time_ms: None,
            new_flag_respawn_until: None,
            new_flag_return_on_defender_interact: None,
            new_flag_pickup_spell_id: None,
            new_flag_entry: None,
            report_use_ai_returns_true: false,
            gossip_hello_ai_returns_true: false,
            capture_point_assault_ai_returns_true: false,
            cooldown_until: None,
        }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedGameObjectCriteriaEvent {
    UseGameobject {
        player_guid: ObjectGuid,
        gameobject_entry: u32,
    },
}
