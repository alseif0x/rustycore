// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player construction and initial state loading.

use super::{
    ACTIVE_PLAYER_DATA_BITS, ActivePlayerDataValues, Gender, HashSet, ObjectGuid, PLAYER_DATA_BITS,
    PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP, Player, PlayerCreateLifecycleRecord, PlayerDataValues,
    PlayerDbLoadLifecycleRecord, PlayerEffectiveCombatStatsLikeCpp, PlayerGameplayState,
    PlayerLifecycleMetadata, PlayerLifecyclePower, PlayerPowerIndexResolver, Position, PowerType,
    TEAM_OTHER, TypeId, TypeMask, Unit, UpdateMask, death_operation, deferred_save, quest_log,
};

#[derive(Debug, Clone, PartialEq)]
struct PlayerLifecycleBase {
    guid: ObjectGuid,
    name: String,
    race: u8,
    class_id: u8,
    gender: Gender,
    level: u8,
    xp: i32,
    money: u64,
    inventory_slot_count: u8,
    bank_bag_slot_count: u8,
    map_id: u32,
    position: Position,
    max_health: u64,
    health: u64,
    powers: Vec<PlayerLifecyclePower>,
    display_power: PowerType,
    faction_template: Option<u32>,
    display_id: Option<u32>,
    player_flags: u32,
    player_flags_ex: u32,
    extra_flags: u32,
    metadata: PlayerLifecycleMetadata,
}

impl Player {
    pub fn new(session_id: Option<u64>, can_filter_whispers: bool) -> Self {
        let mut unit = Unit::new(true);
        unit.set_type(
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );
        let mut gameplay_state = PlayerGameplayState::default();
        gameplay_state.void_storage_items = vec![None; PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP];

        Self {
            unit,
            session_id,
            data: PlayerDataValues::default(),
            active_data: ActivePlayerDataValues::default(),
            inventory: Box::default(),
            inventory_runtime: Box::default(),
            gameplay_state,
            effective_combat_stats: PlayerEffectiveCombatStatsLikeCpp::default(),
            swing_error_msg_like_cpp: None,
            drunk_timer: 0,
            deferred_save: deferred_save::DeferredPlayerSave::default(),
            death_operation: death_operation::PlayerDeathOperationState::default(),
            player_xp_table_like_cpp: None,
            player_data_changes: UpdateMask::new(PLAYER_DATA_BITS),
            quest_log_change_masks: [0; quest_log::MAX_QUEST_LOG_SIZE_LIKE_CPP],
            active_player_data_changes: UpdateMask::new(ACTIVE_PLAYER_DATA_BITS),
            rest_info_change_masks: [0; 2],
            mod_melee_hit_chance: 7.5,
            mod_ranged_hit_chance: 7.5,
            mod_spell_hit_chance: 15.0,
            ingame_time: 0,
            shared_quest_id: 0,
            extra_flags: 0,
            team: TEAM_OTHER,
            is_active: true,
            controlled_by_player: true,
            accept_whispers: !can_filter_whispers,
            can_titan_grip: false,
            titan_grip_penalty_spell_id: 0,
            soulbound_tradeable_items: HashSet::new(),
            item_durations: Vec::new(),
            enchant_durations: Vec::new(),
            lifecycle_metadata: PlayerLifecycleMetadata::default(),
            duel: None,
            duel_arbiter: None,
        }
    }

    pub fn create_from_lifecycle(
        session_id: Option<u64>,
        can_filter_whispers: bool,
        record: PlayerCreateLifecycleRecord,
        resolver: &impl PlayerPowerIndexResolver,
    ) -> Self {
        let mut player = Self::new(session_id, can_filter_whispers);
        player.apply_create_lifecycle(record, resolver);
        player
    }

    pub fn load_from_db_lifecycle(
        session_id: Option<u64>,
        can_filter_whispers: bool,
        record: PlayerDbLoadLifecycleRecord,
        resolver: &impl PlayerPowerIndexResolver,
    ) -> Self {
        let mut player = Self::new(session_id, can_filter_whispers);
        player.apply_db_load_lifecycle(record, resolver);
        player
    }

    pub fn apply_create_lifecycle(
        &mut self,
        record: PlayerCreateLifecycleRecord,
        resolver: &impl PlayerPowerIndexResolver,
    ) {
        let metadata = PlayerLifecycleMetadata {
            account_id: None,
            create_time: record.create_time,
            create_mode: record.create_mode,
            played_time_total: record.played_time_total,
            played_time_level: record.played_time_level,
            active_talent_group: record.active_talent_group,
            zone_id: None,
        };

        self.apply_lifecycle_base(
            PlayerLifecycleBase {
                guid: record.guid,
                name: record.name,
                race: record.race,
                class_id: record.class_id,
                gender: record.gender,
                level: record.level,
                xp: record.xp,
                money: record.money,
                inventory_slot_count: record.inventory_slot_count,
                bank_bag_slot_count: record.bank_bag_slot_count,
                map_id: record.map_id,
                position: record.position,
                max_health: record.max_health,
                health: record.health,
                powers: record.powers,
                display_power: record.display_power,
                faction_template: record.faction_template,
                display_id: record.display_id,
                player_flags: record.player_flags,
                player_flags_ex: record.player_flags_ex,
                extra_flags: record.extra_flags,
                metadata,
            },
            resolver,
        );
    }

    pub fn apply_db_load_lifecycle(
        &mut self,
        record: PlayerDbLoadLifecycleRecord,
        resolver: &impl PlayerPowerIndexResolver,
    ) {
        let metadata = PlayerLifecycleMetadata {
            account_id: Some(record.account_id),
            create_time: record.create_time,
            create_mode: record.create_mode,
            played_time_total: record.played_time_total,
            played_time_level: record.played_time_level,
            active_talent_group: record.active_talent_group,
            zone_id: record.zone_id,
        };

        self.apply_lifecycle_base(
            PlayerLifecycleBase {
                guid: record.guid,
                name: record.name,
                race: record.race,
                class_id: record.class_id,
                gender: record.gender,
                level: record.level,
                xp: record.xp,
                money: record.money,
                inventory_slot_count: record.inventory_slot_count,
                bank_bag_slot_count: record.bank_bag_slot_count,
                map_id: record.map_id,
                position: record.position,
                max_health: record.max_health,
                health: record.health,
                powers: record.powers,
                display_power: record.display_power,
                faction_template: record.faction_template,
                display_id: record.display_id,
                player_flags: record.player_flags,
                player_flags_ex: record.player_flags_ex,
                extra_flags: record.extra_flags,
                metadata,
            },
            resolver,
        );
    }

    fn apply_lifecycle_base(
        &mut self,
        record: PlayerLifecycleBase,
        resolver: &impl PlayerPowerIndexResolver,
    ) {
        self.unit.world_mut().object_mut().create(record.guid);
        self.unit.world_mut().object_mut().set_scale(1.0);
        self.unit.world_mut().set_name(record.name);
        self.unit
            .world_mut()
            .world_relocate(record.map_id, record.position);

        self.set_race_class_gender(record.race, record.class_id, record.gender);
        self.unit.set_level(record.level);
        self.set_inventory_slot_count(record.inventory_slot_count);
        self.set_bank_bag_slot_count(record.bank_bag_slot_count);
        self.set_xp(record.xp);
        self.set_money(record.money);
        self.replace_all_player_flags(record.player_flags);
        self.replace_all_player_flags_ex(record.player_flags_ex);
        self.extra_flags = record.extra_flags;
        self.lifecycle_metadata = record.metadata;
        self.clear_effective_combat_stats_like_cpp();

        self.unit.set_display_power(record.display_power);
        if let Some(faction_template) = record.faction_template {
            self.unit.set_faction(faction_template);
        }
        if let Some(display_id) = record.display_id {
            self.unit.set_display_id(display_id, true);
        }

        self.configure_power_indices_for_class(resolver);
        self.unit.set_max_health(record.max_health);
        self.unit.set_health(record.health);
        for power in record.powers {
            self.unit.set_max_power(power.power, power.max);
            self.unit.set_power(power.power, power.current);
        }

        self.clear_data_changes();
    }
}
