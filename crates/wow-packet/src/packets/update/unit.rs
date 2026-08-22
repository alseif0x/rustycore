// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit and creature update blocks.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactPowerValuesUpdate {
    pub artifact_power_id: i16,
    pub purchased_rank: u8,
    pub current_rank_with_bonus: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnitChannelValuesUpdate {
    pub spell_id: i32,
    pub spell_visual_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitDataValuesDeltaUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub unit_data_mask: [u32; 8],
    pub state_world_effect_ids: Vec<u32>,
    pub passive_spells: Vec<PassiveSpellHistoryValuesUpdate>,
    pub passive_spells_update_mask: Option<Vec<u32>>,
    pub world_effects: Vec<i32>,
    pub world_effects_update_mask: Option<Vec<u32>>,
    pub channel_objects: Vec<ObjectGuid>,
    pub channel_objects_update_mask: Option<Vec<u32>>,
    pub health: i64,
    pub max_health: i64,
    pub display_id: i32,
    pub state_spell_visual_id: u32,
    pub state_anim_id: u32,
    pub state_anim_kit_id: u32,
    pub charm: ObjectGuid,
    pub summon: ObjectGuid,
    pub critter: ObjectGuid,
    pub charmed_by: ObjectGuid,
    pub summoned_by: ObjectGuid,
    pub created_by: ObjectGuid,
    pub demon_creator: ObjectGuid,
    pub look_at_controller_target: ObjectGuid,
    pub target: ObjectGuid,
    pub battle_pet_companion_guid: ObjectGuid,
    pub battle_pet_db_id: u64,
    pub channel_data: UnitChannelValuesUpdate,
    pub summoned_by_home_realm: u32,
    pub race: u8,
    pub class_id: u8,
    pub player_class_id: u8,
    pub sex: u8,
    pub display_power: u8,
    pub override_display_power_id: u32,
    pub level: i32,
    pub effective_level: i32,
    pub content_tuning_id: i32,
    pub scaling_level_min: i32,
    pub scaling_level_max: i32,
    pub scaling_level_delta: i32,
    pub scaling_faction_group: i32,
    pub scaling_health_item_level_curve_id: i32,
    pub scaling_damage_item_level_curve_id: i32,
    pub faction_template: i32,
    pub flags: u32,
    pub flags2: u32,
    pub flags3: u32,
    pub aura_state: u32,
    pub ranged_attack_round_base_time: u32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub display_scale: f32,
    pub native_display_id: i32,
    pub native_display_scale: f32,
    pub mount_display_id: i32,
    pub min_damage: f32,
    pub max_damage: f32,
    pub min_off_hand_damage: f32,
    pub max_off_hand_damage: f32,
    pub stand_state: u8,
    pub pet_talent_points: u8,
    pub vis_flags: u8,
    pub anim_tier: u8,
    pub pet_number: u32,
    pub pet_name_timestamp: u32,
    pub pet_experience: u32,
    pub pet_next_level_experience: u32,
    pub mod_casting_speed: f32,
    pub mod_spell_haste: f32,
    pub mod_haste: f32,
    pub mod_ranged_haste: f32,
    pub mod_haste_regen: f32,
    pub mod_time_rate: f32,
    pub created_by_spell: i32,
    pub emote_state: i32,
    pub training_points_used: i16,
    pub training_points_total: i16,
    pub base_mana: i32,
    pub base_health: i32,
    pub sheathe_state: u8,
    pub pvp_flags: u8,
    pub pet_flags: u8,
    pub shapeshift_form: u8,
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    pub attack_power_mod_neg: i32,
    pub attack_power_multiplier: f32,
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    pub ranged_attack_power_mod_neg: i32,
    pub ranged_attack_power_multiplier: f32,
    pub set_attack_speed_aura: i32,
    pub lifesteal: f32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub max_health_modifier: f32,
    pub hover_height: f32,
    pub min_item_level_cutoff: i32,
    pub min_item_level: i32,
    pub max_item_level: i32,
    pub wild_battle_pet_level: i32,
    pub battle_pet_companion_name_timestamp: u32,
    pub interact_spell_id: i32,
    pub scale_duration: i32,
    pub looks_like_mount_id: i32,
    pub looks_like_creature_id: i32,
    pub look_at_controller_id: i32,
    pub perks_vendor_item_id: i32,
    pub guild_guid: ObjectGuid,
    pub skinning_owner_guid: ObjectGuid,
    pub flight_capability_id: i32,
    pub glide_event_speed_divisor: f32,
    pub current_area_id: u32,
    pub combo_target: ObjectGuid,
    pub npc_flags: [u32; 2],
    pub power_regen_flat_modifier: [f32; 10],
    pub power_regen_interrupted_flat_modifier: [f32; 10],
    pub power: [i32; 10],
    pub max_power: [i32; 10],
    pub mod_power_regen: [f32; 10],
    pub virtual_items: [VisibleItemValuesUpdate; 3],
    pub attack_round_base_time: [u32; 2],
    pub stats: [i32; 5],
    pub stat_pos_buff: [i32; 5],
    pub stat_neg_buff: [i32; 5],
    pub resistances: [i32; 7],
    pub power_cost_modifier: [i32; 7],
    pub power_cost_multiplier: [f32; 7],
    pub resistance_buff_mods_positive: [i32; 7],
    pub resistance_buff_mods_negative: [i32; 7],
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StablePetInfoValuesUpdate {
    pub stable_pet_mask: u8,
    pub pet_slot: u32,
    pub pet_number: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub experience_level: u32,
    pub name: String,
    pub pet_flags: u8,
}

impl Default for UnitDataValuesDeltaUpdate {
    fn default() -> Self {
        Self {
            changed_object_type_mask: VALUES_TYPE_UNIT,
            object_data: None,
            unit_data_mask: [0; 8],
            state_world_effect_ids: Vec::new(),
            passive_spells: Vec::new(),
            passive_spells_update_mask: None,
            world_effects: Vec::new(),
            world_effects_update_mask: None,
            channel_objects: Vec::new(),
            channel_objects_update_mask: None,
            health: 0,
            max_health: 0,
            display_id: 0,
            state_spell_visual_id: 0,
            state_anim_id: 0,
            state_anim_kit_id: 0,
            charm: ObjectGuid::EMPTY,
            summon: ObjectGuid::EMPTY,
            critter: ObjectGuid::EMPTY,
            charmed_by: ObjectGuid::EMPTY,
            summoned_by: ObjectGuid::EMPTY,
            created_by: ObjectGuid::EMPTY,
            demon_creator: ObjectGuid::EMPTY,
            look_at_controller_target: ObjectGuid::EMPTY,
            target: ObjectGuid::EMPTY,
            battle_pet_companion_guid: ObjectGuid::EMPTY,
            battle_pet_db_id: 0,
            channel_data: UnitChannelValuesUpdate::default(),
            summoned_by_home_realm: 0,
            race: 0,
            class_id: 0,
            player_class_id: 0,
            sex: 0,
            display_power: 0,
            override_display_power_id: 0,
            level: 0,
            effective_level: 0,
            content_tuning_id: 0,
            scaling_level_min: 0,
            scaling_level_max: 0,
            scaling_level_delta: 0,
            scaling_faction_group: 0,
            scaling_health_item_level_curve_id: 0,
            scaling_damage_item_level_curve_id: 0,
            faction_template: 0,
            flags: 0,
            flags2: 0,
            flags3: 0,
            aura_state: 0,
            ranged_attack_round_base_time: 0,
            bounding_radius: 0.0,
            combat_reach: 0.0,
            display_scale: 0.0,
            native_display_id: 0,
            native_display_scale: 0.0,
            mount_display_id: 0,
            min_damage: 0.0,
            max_damage: 0.0,
            min_off_hand_damage: 0.0,
            max_off_hand_damage: 0.0,
            stand_state: 0,
            pet_talent_points: 0,
            vis_flags: 0,
            anim_tier: 0,
            pet_number: 0,
            pet_name_timestamp: 0,
            pet_experience: 0,
            pet_next_level_experience: 0,
            mod_casting_speed: 0.0,
            mod_spell_haste: 0.0,
            mod_haste: 0.0,
            mod_ranged_haste: 0.0,
            mod_haste_regen: 0.0,
            mod_time_rate: 0.0,
            created_by_spell: 0,
            emote_state: 0,
            training_points_used: 0,
            training_points_total: 0,
            base_mana: 0,
            base_health: 0,
            sheathe_state: 0,
            pvp_flags: 0,
            pet_flags: 0,
            shapeshift_form: 0,
            attack_power: 0,
            attack_power_mod_pos: 0,
            attack_power_mod_neg: 0,
            attack_power_multiplier: 0.0,
            ranged_attack_power: 0,
            ranged_attack_power_mod_pos: 0,
            ranged_attack_power_mod_neg: 0,
            ranged_attack_power_multiplier: 0.0,
            set_attack_speed_aura: 0,
            lifesteal: 0.0,
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,
            max_health_modifier: 0.0,
            hover_height: 0.0,
            min_item_level_cutoff: 0,
            min_item_level: 0,
            max_item_level: 0,
            wild_battle_pet_level: 0,
            battle_pet_companion_name_timestamp: 0,
            interact_spell_id: 0,
            scale_duration: 0,
            looks_like_mount_id: 0,
            looks_like_creature_id: 0,
            look_at_controller_id: 0,
            perks_vendor_item_id: 0,
            guild_guid: ObjectGuid::EMPTY,
            skinning_owner_guid: ObjectGuid::EMPTY,
            flight_capability_id: 0,
            glide_event_speed_divisor: 0.0,
            current_area_id: 0,
            combo_target: ObjectGuid::EMPTY,
            npc_flags: [0; 2],
            power_regen_flat_modifier: [0.0; 10],
            power_regen_interrupted_flat_modifier: [0.0; 10],
            power: [0; 10],
            max_power: [0; 10],
            mod_power_regen: [0.0; 10],
            virtual_items: [VisibleItemValuesUpdate::default(); 3],
            attack_round_base_time: [0; 2],
            stats: [0; 5],
            stat_pos_buff: [0; 5],
            stat_neg_buff: [0; 5],
            resistances: [0; 7],
            power_cost_modifier: [0; 7],
            power_cost_multiplier: [0.0; 7],
            resistance_buff_mods_positive: [0; 7],
            resistance_buff_mods_negative: [0; 7],
        }
    }
}

// ── ItemCreateData ──────────────────────────────────────────────────

/// C++ `Unit::Update` → `ModifyAuraState` health-derived `UNIT_FIELD_AURASTATE` bits
/// (Unit.cpp:469-476), applied to EVERY alive unit including the player. A full-HP unit
/// yields 0x00D00000. Mirrors `WorldCreature::health_aura_state_like_cpp` in wow-world
/// (both implement the same AURA_STATE 1-based-index bit math; kept in sync).
pub(super) fn health_aura_state_like_cpp(health: i64, max_health: i64, alive: bool) -> u32 {
    if !alive || max_health <= 0 {
        return 0;
    }
    let below = |p: i64| health.saturating_mul(100) < max_health.saturating_mul(p);
    let above = |p: i64| health.saturating_mul(100) > max_health.saturating_mul(p);
    let mut state = 0u32;
    let mut set = |idx: u32, on: bool| {
        if on {
            state |= 1 << (idx - 1);
        }
    };
    set(2, below(20)); // AURA_STATE_WOUNDED_20_PERCENT
    set(6, below(25)); // AURA_STATE_WOUNDED_25_PERCENT
    set(13, below(35)); // AURA_STATE_WOUNDED_35_PERCENT
    set(21, below(20) || above(80)); // AURA_STATE_WOUND_HEALTH_20_80
    set(23, above(75)); // AURA_STATE_HEALTHY_75_PERCENT
    set(24, below(35) || above(80)); // AURA_STATE_WOUND_HEALTH_35_80
    state
}

/// Get power type for a class (0=mana, 1=rage, 3=energy).
pub(super) fn power_type_for_class(class: u8) -> u8 {
    match class {
        1 => 1,  // Warrior → Rage
        4 => 3,  // Rogue → Energy
        11 => 0, // Druid → Mana
        6 => 6,  // DeathKnight → Runic Power (POWER_RUNIC_POWER, C++ SharedDefines.h:287)
        _ => 0,  // Default → Mana
    }
}

// ── CreatureCreateData ──────────────────────────────────────────────

/// Data needed to build a creature create packet for the client.
#[derive(Debug, Clone)]
pub struct CreatureCreateData {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub display_id: u32,
    pub native_display_id: u32,
    pub display_scale: f32,
    pub native_x_display_scale: f32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub health: i64,
    pub max_health: i64,
    pub level: u8,
    pub faction_template: i32,
    pub npc_flags: u64,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
    /// C++ `UNIT_FIELD_AURASTATE`. Derived from health in `Unit::Update` ->
    /// `ModifyAuraState` (Unit.cpp:469-476). A full-HP alive creature carries
    /// `0x00D00000` (WOUND_HEALTH_20_80 | HEALTHY_75_PERCENT | WOUND_HEALTH_35_80).
    /// The 3.4.3 client tests bit 0x100000 of this field on a per-frame unit tick;
    /// shipping 0 where the bit should be set crashes the client (ERROR #132).
    pub aura_state: u32,
    pub damage_school: u8,
    pub scale: f32,
    pub unit_class: u8,
    pub display_power: u8,
    pub power: [i32; 10],
    pub max_power: [i32; 10],
    pub base_mana: i32,
    pub virtual_items: [(i32, u16, u16); 3],
    pub base_attack_time: u32,
    pub ranged_attack_time: u32,
    pub movement_flags: u32,
    pub vehicle_id: u32,
    pub play_hover_anim: bool,
    pub hover_height: f32,
    pub mount_display_id: i32,
    pub stand_state: u8,
    pub vis_flags: u8,
    pub anim_tier: u8,
    pub emote_state: i32,
    pub sheathe_state: u8,
    pub pvp_flags: u8,
    pub current_area_id: u32,
    /// Speed rate from creature_template.speed_walk (1.0 = default).
    pub speed_walk_rate: f32,
    /// Speed rate from creature_template.speed_run (1.14286 = default).
    pub speed_run_rate: f32,
    pub ai_anim_kit_id: u16,
    pub movement_anim_kit_id: u16,
    pub melee_anim_kit_id: u16,
}

impl CreatureCreateData {
    /// Write the complete values block for CREATE (no change masks).
    ///
    /// For creatures: ObjectData + UnitData only (no PlayerData/ActivePlayerData).
    /// Flags = 0x00 (not owner), so many conditional blocks are skipped.
    pub fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: 0x00 for creatures viewed by a non-owner
        buf.write_uint8(0x00);

        self.write_object_data(&mut buf);
        self.write_unit_data(&mut buf);

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }

    fn write_object_data(&self, buf: &mut WorldPacket) {
        buf.write_int32(self.entry as i32); // EntryId (non-zero for creatures)
        buf.write_uint32(0); // DynamicFlags
        buf.write_float(self.scale); // Scale
    }

    fn write_unit_data(&self, buf: &mut WorldPacket) {
        // Health / MaxHealth
        buf.write_int64(self.health);
        buf.write_int64(self.max_health);

        // DisplayId
        buf.write_int32(self.display_id as i32);

        // NpcFlags[2] (split 64-bit into two u32s)
        buf.write_uint32(self.npc_flags as u32);
        buf.write_uint32((self.npc_flags >> 32) as u32);

        // StateSpellVisualID, StateAnimID, StateAnimKitID.
        // C++ Creature::UpdateEntry (Creature.cpp:613) seeds StateAnimID with
        // DB2Manager::GetEmptyAnimStateID() = 1772 (DB2Stores.cpp:1765): "the Classic
        // client expects the retail storage size so we have to hardcode the value".
        // Shipping 0 makes the 3.4.3 client index its AnimationData storage out of range
        // -> NULL deref in the render/anim worker (~4s in-world, ERROR #132). Players are
        // NOT seeded by C++ (only Creature::UpdateEntry), so PlayerCreateData stays 0.
        const EMPTY_ANIM_STATE_ID_LIKE_CPP: i32 = 1772;
        buf.write_int32(0);
        buf.write_int32(EMPTY_ANIM_STATE_ID_LIKE_CPP);
        buf.write_int32(0);

        // StateWorldEffectIDs.Count
        buf.write_int32(0);

        // 9 PackedGuids (no Critter — that's Owner-only)
        for _ in 0..9 {
            write_empty_guid(buf);
        }

        // BattlePetDBID
        buf.write_uint64(0);

        // ChannelData: SpellID + SpellXSpellVisualID
        buf.write_int32(0);
        buf.write_int32(0);

        // SummonedByHomeRealm
        buf.write_uint32(0);

        // Race, ClassId, PlayerClassId, Sex, DisplayPower
        buf.write_uint8(0); // Race (0 for creatures)
        buf.write_uint8(self.unit_class);
        buf.write_uint8(0); // PlayerClassId (0 for creatures)
        buf.write_uint8(0); // Sex
        buf.write_uint8(self.display_power);

        // OverrideDisplayPowerID
        buf.write_int32(0);

        // NO PowerRegen (Owner-only)

        // Power[10], MaxPower[10], ModPowerRegen[10]
        for index in 0..10 {
            buf.write_int32(self.power[index]);
            buf.write_int32(self.max_power[index]);
            buf.write_float(0.0); // ModPowerRegen
        }

        // Level, EffectiveLevel, ContentTuningID, Scaling fields (9x i32)
        buf.write_int32(self.level as i32);
        buf.write_int32(self.level as i32);
        buf.write_int32(0); // ContentTuningID
        buf.write_int32(0); // ScalingLevelMin
        buf.write_int32(0); // ScalingLevelMax
        buf.write_int32(0); // ScalingLevelDelta
        buf.write_int32(0); // ScalingFactionGroup
        buf.write_int32(0); // ScalingHealthItemLevelCurveID
        buf.write_int32(0); // ScalingDamageItemLevelCurveID

        // FactionTemplate
        buf.write_int32(self.faction_template);

        // VirtualItems[3]
        for (item_id, appearance_mod_id, item_visual) in self.virtual_items {
            buf.write_int32(item_id);
            buf.write_uint16(appearance_mod_id);
            buf.write_uint16(item_visual);
        }

        // Flags, Flags2, Flags3, AuraState
        buf.write_uint32(self.unit_flags);
        buf.write_uint32(self.unit_flags2);
        buf.write_uint32(self.unit_flags3);
        buf.write_uint32(self.aura_state); // AuraState (C++ UNIT_FIELD_AURASTATE)

        // AttackRoundBaseTime[2]
        buf.write_uint32(self.base_attack_time);
        buf.write_uint32(self.base_attack_time);

        // NO RangedAttackRoundBaseTime (Owner-only)

        // BoundingRadius, CombatReach, DisplayScale
        buf.write_float(self.bounding_radius);
        buf.write_float(self.combat_reach);
        buf.write_float(self.display_scale);

        // NativeDisplayID, NativeXDisplayScale, MountDisplayID
        buf.write_int32(self.native_display_id as i32);
        buf.write_float(self.native_x_display_scale);
        buf.write_int32(self.mount_display_id);

        // NO damage floats (Owner|Empath only)

        // StandState, PetTalentPoints, VisFlags, AnimTier
        buf.write_uint8(self.stand_state);
        buf.write_uint8(0);
        buf.write_uint8(self.vis_flags);
        buf.write_uint8(self.anim_tier);

        // PetNumber, PetNameTimestamp, PetExperience, PetNextLevelExperience
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // ModCastingSpeed, ModSpellHaste, ModHaste, ModRangedHaste,
        // ModHasteRegen, ModTimeRate.
        // C++ 3.4.3 `UnitData::WriteCreate` writes exactly these six floats
        // before CreatedBySpell (`UpdateFields.cpp:750-756`).
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);

        // CreatedBySpell, EmoteState
        buf.write_int32(0);
        buf.write_int32(self.emote_state);

        // TrainingPointsUsed, TrainingPointsTotal
        buf.write_int16(0);
        buf.write_int16(0);

        // NO Stats/StatBuff (Owner-only)
        // NO Resistances (Owner|Empath only)
        // NO PowerCostModifier/Multiplier (Owner-only)

        // ResistanceBuffModsPositive[7] + Negative[7]
        for _ in 0..7 {
            buf.write_int32(0);
            buf.write_int32(0);
        }

        // BaseMana
        buf.write_int32(self.base_mana);

        // NO BaseHealth (Owner-only)

        // SheatheState, PvpFlags, PetFlags, ShapeshiftForm
        buf.write_uint8(self.sheathe_state);
        buf.write_uint8(self.pvp_flags);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // NO AttackPower block (Owner-only)

        // HoverHeight + misc fields
        buf.write_float(self.hover_height);
        buf.write_int32(0); // MinItemLevelCutoff
        buf.write_int32(0); // MinItemLevel
        buf.write_int32(0); // MaxItemLevel
        buf.write_int32(0); // WildBattlePetLevel
        buf.write_int32(0); // BattlePetCompanionNameTimestamp
        buf.write_int32(0); // InteractSpellId
        buf.write_int32(0); // ScaleDuration
        buf.write_int32(0); // LooksLikeMountID
        buf.write_int32(0); // LooksLikeCreatureID
        buf.write_int32(0); // LookAtControllerID
        buf.write_int32(0); // PerksVendorItemID
        write_empty_guid(buf); // GuildGUID

        // Dynamic array sizes: PassiveSpells, WorldEffects, ChannelObjects
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        write_empty_guid(buf); // SkinningOwnerGUID

        // FlightCapabilityID, GlideEventSpeedDivisor, CurrentAreaID
        buf.write_int32(0);
        buf.write_float(0.0);
        buf.write_uint32(self.current_area_id);

        // NO ComboTarget (Owner-only)
    }
}

// ── UpdateBlock ─────────────────────────────────────────────────────

// ── GameObjectCreateData ──────────────────────────────────────────

pub(super) fn debug_creature_create_values_len_like_cpp(data: &CreatureCreateData) -> usize {
    let mut values = WorldPacket::new_empty();
    data.write_values_create(&mut values);
    values.into_data().len()
}

/// Write a single CreateObject block for a creature (TypeId::Unit).
pub(super) fn write_creature_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    movement: &MovementBlock,
    create_data: &CreatureCreateData,
) {
    // C++ `Object::BuildCreateUpdateBlockForPlayer` uses CreateObject2 only
    // while `Map::AddToMap` temporarily sets `m_isNewObject=true`
    // (`Map.cpp:573-575`, `Object.cpp:135`). Login visibility for creatures
    // already present in the map uses the normal CreateObject update type.
    buf.write_uint8(UpdateType::CreateObject as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = Unit (5)
    buf.write_uint8(TypeId::Unit as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    let has_anim_kit = create_data.ai_anim_kit_id != 0
        || create_data.movement_anim_kit_id != 0
        || create_data.melee_anim_kit_id != 0;
    let has_vehicle = create_data.vehicle_id != 0;
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(create_data.play_hover_anim); // 2: PlayHoverAnim
    buf.write_bit(true); // 3: MovementUpdate (always true for Unit)
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(false); // 5: Stationary
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(false); // 7: ServerTime
    buf.write_bit(has_vehicle); // 8: Vehicle
    buf.write_bit(has_anim_kit); // 9: AnimKit
    buf.write_bit(false); // 10: Rotation
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(false); // 12: GameObject
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(false); // 14: ThisIsYou (false for creatures)
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(false); // 16: ActivePlayer (false for creatures)
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // ── MovementUpdate block ───────────────────────────────
    write_movement_update(buf, guid, movement);

    // PauseTimes count
    buf.write_int32(0);

    if has_vehicle {
        buf.write_uint32(create_data.vehicle_id);
        buf.write_float(movement.position.orientation);
    }

    if has_anim_kit {
        buf.write_uint16(create_data.ai_anim_kit_id);
        buf.write_uint16(create_data.movement_anim_kit_id);
        buf.write_uint16(create_data.melee_anim_kit_id);
    }

    // No ActivePlayer block (bit 16 = false)

    // ── Values block ───────────────────────────────────────
    create_data.write_values_create(buf);
}

pub(super) fn write_stationary_world_object_create_prefix_like_cpp(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    type_id: TypeId,
    position: Position,
    scene_object: bool,
    conversation_texture_kit_id: Option<u32>,
) {
    buf.write_uint8(UpdateType::CreateObject as u8);
    buf.write_packed_guid(guid);
    buf.write_uint8(type_id as u8);

    buf.write_bit(false); // NoBirthAnim
    buf.write_bit(false); // EnablePortals
    buf.write_bit(false); // PlayHoverAnim
    buf.write_bit(false); // MovementUpdate
    buf.write_bit(false); // MovementTransport
    buf.write_bit(true); // Stationary
    buf.write_bit(false); // CombatVictim
    buf.write_bit(false); // ServerTime
    buf.write_bit(false); // Vehicle
    buf.write_bit(false); // AnimKit
    buf.write_bit(false); // Rotation
    buf.write_bit(false); // AreaTrigger
    buf.write_bit(false); // GameObject
    buf.write_bit(false); // SmoothPhasing
    buf.write_bit(false); // ThisIsYou
    buf.write_bit(scene_object); // SceneObject
    buf.write_bit(false); // ActivePlayer
    buf.write_bit(conversation_texture_kit_id.is_some()); // Conversation
    buf.flush_bits();

    buf.write_uint32(0); // PauseTimes count
    buf.write_float(position.x);
    buf.write_float(position.y);
    buf.write_float(position.z);
    buf.write_float(position.orientation);

    if scene_object {
        buf.write_bit(false); // HasLocalScriptData
        buf.write_bit(false); // HasPetBattleFullUpdate
        buf.flush_bits();
    }

    if let Some(texture_kit_id) = conversation_texture_kit_id {
        let has_texture_kit = texture_kit_id != 0;
        buf.write_bit(has_texture_kit);
        if has_texture_kit {
            buf.write_uint32(texture_kit_id);
        }
        buf.flush_bits();
    }
}

pub(super) const VALUES_TYPE_UNIT: u32 = 1 << 5;

pub(super) fn write_artifact_power_values_update(
    buf: &mut WorldPacket,
    data: &ArtifactPowerValuesUpdate,
) {
    buf.write_int16(data.artifact_power_id);
    buf.write_uint8(data.purchased_rank);
    buf.write_uint8(data.current_rank_with_bonus);
}

fn unit_mask_has(data: &UnitDataValuesDeltaUpdate, bit: usize) -> bool {
    let block = bit / 32;
    let offset = bit % 32;
    data.unit_data_mask.get(block).copied().unwrap_or(0) & (1 << offset) != 0
}

fn write_unit_channel_values_update(buf: &mut WorldPacket, data: &UnitChannelValuesUpdate) {
    buf.write_int32(data.spell_id);
    buf.write_int32(data.spell_visual_id);
}

pub(super) fn write_unit_data_values_update_section(
    buf: &mut WorldPacket,
    data: &UnitDataValuesDeltaUpdate,
) {
    write_update_field_blocks_mask_u32(buf, &data.unit_data_mask, 8);

    if unit_mask_has(data, 0) && unit_mask_has(data, 1) {
        buf.write_bits(data.state_world_effect_ids.len() as u32, 32);
        for effect_id in &data.state_world_effect_ids {
            buf.write_uint32(*effect_id);
        }
    }
    buf.flush_bits();

    if unit_mask_has(data, 0) {
        if unit_mask_has(data, 2) {
            write_dynamic_field_update_mask(
                buf,
                data.passive_spells.len(),
                data.passive_spells_update_mask.as_deref(),
            );
        }
        if unit_mask_has(data, 3) {
            write_dynamic_field_update_mask(
                buf,
                data.world_effects.len(),
                data.world_effects_update_mask.as_deref(),
            );
        }
        if unit_mask_has(data, 4) {
            write_dynamic_field_update_mask(
                buf,
                data.channel_objects.len(),
                data.channel_objects_update_mask.as_deref(),
            );
        }
    }
    buf.flush_bits();

    if unit_mask_has(data, 0) {
        if unit_mask_has(data, 2) {
            for (index, spell) in data.passive_spells.iter().enumerate() {
                if dynamic_mask_has_index(data.passive_spells_update_mask.as_deref(), index) {
                    write_passive_spell_history_values_update(buf, spell);
                }
            }
        }
        if unit_mask_has(data, 3) {
            write_changed_i32_dynamic_values(
                buf,
                &data.world_effects,
                data.world_effects_update_mask.as_deref(),
            );
        }
        if unit_mask_has(data, 4) {
            for (index, guid) in data.channel_objects.iter().enumerate() {
                if dynamic_mask_has_index(data.channel_objects_update_mask.as_deref(), index) {
                    buf.write_packed_guid(guid);
                }
            }
        }
        if unit_mask_has(data, 5) {
            buf.write_int64(data.health);
        }
        if unit_mask_has(data, 6) {
            buf.write_int64(data.max_health);
        }
        if unit_mask_has(data, 7) {
            buf.write_int32(data.display_id);
        }
        if unit_mask_has(data, 8) {
            buf.write_uint32(data.state_spell_visual_id);
        }
        if unit_mask_has(data, 9) {
            buf.write_uint32(data.state_anim_id);
        }
        if unit_mask_has(data, 10) {
            buf.write_uint32(data.state_anim_kit_id);
        }
        for (bit, guid) in [
            (11, &data.charm),
            (12, &data.summon),
            (13, &data.critter),
            (14, &data.charmed_by),
            (15, &data.summoned_by),
            (16, &data.created_by),
            (17, &data.demon_creator),
            (18, &data.look_at_controller_target),
            (19, &data.target),
            (20, &data.battle_pet_companion_guid),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_packed_guid(guid);
            }
        }
        if unit_mask_has(data, 21) {
            buf.write_uint64(data.battle_pet_db_id);
        }
        if unit_mask_has(data, 22) {
            write_unit_channel_values_update(buf, &data.channel_data);
        }
        if unit_mask_has(data, 23) {
            buf.write_uint32(data.summoned_by_home_realm);
        }
        if unit_mask_has(data, 24) {
            buf.write_uint8(data.race);
        }
        if unit_mask_has(data, 25) {
            buf.write_uint8(data.class_id);
        }
        if unit_mask_has(data, 26) {
            buf.write_uint8(data.player_class_id);
        }
        if unit_mask_has(data, 27) {
            buf.write_uint8(data.sex);
        }
        if unit_mask_has(data, 28) {
            buf.write_uint8(data.display_power);
        }
        if unit_mask_has(data, 29) {
            buf.write_uint32(data.override_display_power_id);
        }
        if unit_mask_has(data, 30) {
            buf.write_int32(data.level);
        }
        if unit_mask_has(data, 31) {
            buf.write_int32(data.effective_level);
        }
    }

    if unit_mask_has(data, 32) {
        for (bit, value) in [
            (33, data.content_tuning_id),
            (34, data.scaling_level_min),
            (35, data.scaling_level_max),
            (36, data.scaling_level_delta),
            (37, data.scaling_faction_group),
            (38, data.scaling_health_item_level_curve_id),
            (39, data.scaling_damage_item_level_curve_id),
            (40, data.faction_template),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 41) {
            buf.write_uint32(data.flags);
        }
        if unit_mask_has(data, 42) {
            buf.write_uint32(data.flags2);
        }
        if unit_mask_has(data, 43) {
            buf.write_uint32(data.flags3);
        }
        if unit_mask_has(data, 44) {
            buf.write_uint32(data.aura_state);
        }
        if unit_mask_has(data, 45) {
            buf.write_uint32(data.ranged_attack_round_base_time);
        }
        for (bit, value) in [
            (46, data.bounding_radius),
            (47, data.combat_reach),
            (48, data.display_scale),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_float(value);
            }
        }
        if unit_mask_has(data, 49) {
            buf.write_int32(data.native_display_id);
        }
        if unit_mask_has(data, 50) {
            buf.write_float(data.native_display_scale);
        }
        if unit_mask_has(data, 51) {
            buf.write_int32(data.mount_display_id);
        }
        for (bit, value) in [
            (52, data.min_damage),
            (53, data.max_damage),
            (54, data.min_off_hand_damage),
            (55, data.max_off_hand_damage),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_float(value);
            }
        }
        for (bit, value) in [
            (56, data.stand_state),
            (57, data.pet_talent_points),
            (58, data.vis_flags),
            (59, data.anim_tier),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_uint8(value);
            }
        }
        for (bit, value) in [
            (60, data.pet_number),
            (61, data.pet_name_timestamp),
            (62, data.pet_experience),
            (63, data.pet_next_level_experience),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_uint32(value);
            }
        }
    }

    if unit_mask_has(data, 64) {
        for (bit, value) in [
            (65, data.mod_casting_speed),
            (66, data.mod_spell_haste),
            (67, data.mod_haste),
            (68, data.mod_ranged_haste),
            (69, data.mod_haste_regen),
            (70, data.mod_time_rate),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_float(value);
            }
        }
        for (bit, value) in [(71, data.created_by_spell), (72, data.emote_state)] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 73) {
            buf.write_int16(data.training_points_used);
        }
        if unit_mask_has(data, 74) {
            buf.write_int16(data.training_points_total);
        }
        if unit_mask_has(data, 75) {
            buf.write_int32(data.base_mana);
        }
        if unit_mask_has(data, 76) {
            buf.write_int32(data.base_health);
        }
        for (bit, value) in [
            (77, data.sheathe_state),
            (78, data.pvp_flags),
            (79, data.pet_flags),
            (80, data.shapeshift_form),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_uint8(value);
            }
        }
        for (bit, value) in [
            (81, data.attack_power),
            (82, data.attack_power_mod_pos),
            (83, data.attack_power_mod_neg),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 84) {
            buf.write_float(data.attack_power_multiplier);
        }
        for (bit, value) in [
            (85, data.ranged_attack_power),
            (86, data.ranged_attack_power_mod_pos),
            (87, data.ranged_attack_power_mod_neg),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 88) {
            buf.write_float(data.ranged_attack_power_multiplier);
        }
        if unit_mask_has(data, 89) {
            buf.write_int32(data.set_attack_speed_aura);
        }
        for (bit, value) in [
            (90, data.lifesteal),
            (91, data.min_ranged_damage),
            (92, data.max_ranged_damage),
            (93, data.max_health_modifier),
            (94, data.hover_height),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_float(value);
            }
        }
        if unit_mask_has(data, 95) {
            buf.write_int32(data.min_item_level_cutoff);
        }
    }

    if unit_mask_has(data, 96) {
        for (bit, value) in [
            (97, data.min_item_level),
            (98, data.max_item_level),
            (99, data.wild_battle_pet_level),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 100) {
            buf.write_uint32(data.battle_pet_companion_name_timestamp);
        }
        for (bit, value) in [
            (101, data.interact_spell_id),
            (102, data.scale_duration),
            (103, data.looks_like_mount_id),
            (104, data.looks_like_creature_id),
            (105, data.look_at_controller_id),
            (106, data.perks_vendor_item_id),
        ] {
            if unit_mask_has(data, bit) {
                buf.write_int32(value);
            }
        }
        if unit_mask_has(data, 107) {
            buf.write_packed_guid(&data.guild_guid);
        }
        if unit_mask_has(data, 108) {
            buf.write_packed_guid(&data.skinning_owner_guid);
        }
        if unit_mask_has(data, 109) {
            buf.write_int32(data.flight_capability_id);
        }
        if unit_mask_has(data, 110) {
            buf.write_float(data.glide_event_speed_divisor);
        }
        if unit_mask_has(data, 111) {
            buf.write_uint32(data.current_area_id);
        }
        if unit_mask_has(data, 112) {
            buf.write_packed_guid(&data.combo_target);
        }
    }

    if unit_mask_has(data, 113) {
        for i in 0..2 {
            if unit_mask_has(data, 114 + i) {
                buf.write_uint32(data.npc_flags[i]);
            }
        }
    }

    if unit_mask_has(data, 116) {
        for i in 0..10 {
            if unit_mask_has(data, 117 + i) {
                buf.write_float(data.power_regen_flat_modifier[i]);
            }
            if unit_mask_has(data, 127 + i) {
                buf.write_float(data.power_regen_interrupted_flat_modifier[i]);
            }
            if unit_mask_has(data, 137 + i) {
                buf.write_int32(data.power[i]);
            }
            if unit_mask_has(data, 147 + i) {
                buf.write_int32(data.max_power[i]);
            }
            if unit_mask_has(data, 157 + i) {
                buf.write_float(data.mod_power_regen[i]);
            }
        }
    }

    if unit_mask_has(data, 167) {
        for i in 0..3 {
            if unit_mask_has(data, 168 + i) {
                write_visible_item_values_update(buf, &data.virtual_items[i]);
            }
        }
    }

    if unit_mask_has(data, 171) {
        for i in 0..2 {
            if unit_mask_has(data, 172 + i) {
                buf.write_uint32(data.attack_round_base_time[i]);
            }
        }
    }

    if unit_mask_has(data, 174) {
        for i in 0..5 {
            if unit_mask_has(data, 175 + i) {
                buf.write_int32(data.stats[i]);
            }
            if unit_mask_has(data, 180 + i) {
                buf.write_int32(data.stat_pos_buff[i]);
            }
            if unit_mask_has(data, 185 + i) {
                buf.write_int32(data.stat_neg_buff[i]);
            }
        }
    }

    if unit_mask_has(data, 190) {
        for i in 0..7 {
            if unit_mask_has(data, 191 + i) {
                buf.write_int32(data.resistances[i]);
            }
            if unit_mask_has(data, 198 + i) {
                buf.write_int32(data.power_cost_modifier[i]);
            }
            if unit_mask_has(data, 205 + i) {
                buf.write_float(data.power_cost_multiplier[i]);
            }
        }
    }

    if unit_mask_has(data, 212) {
        for i in 0..7 {
            if unit_mask_has(data, 213 + i) {
                buf.write_int32(data.resistance_buff_mods_positive[i]);
            }
            if unit_mask_has(data, 220 + i) {
                buf.write_int32(data.resistance_buff_mods_negative[i]);
            }
        }
    }
}

pub(super) fn write_full_unit_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &UnitDataValuesDeltaUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_UNIT != 0 {
        write_unit_data_values_update_section(&mut val_buf, data);
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

/// UnitData VALUES update: VirtualItems[3] and/or stat fields.
///
/// C++ `UF::UnitData::WriteUpdate` format
/// (`Entities/Object/Updates/UpdateFields.cpp:852-900`):
///   WriteBits(blocksMask, 8) — which of 8 blocks have changes
///   for each active block: WriteBits(block, 32)
///   [dynamic arrays if block 0 active]
///   FlushBits()
///   [field values in generated C++ update-field definition order]
///
/// Field write order (C++ `UnitData::WriteUpdate`):
///   Block 0: Health(5), MaxHealth(6)
///   Block 1: MinDamage(52→20), MaxDamage(53→21)
///   Block 2: BaseMana(75→11), BaseHealth(76→12), AttackPower(81-84→17-20),
///            RangedAttackPower(85-88→21-24), MinRangedDamage(91→27), MaxRangedDamage(92→28)
///   Block 3: Power parent(116→20)
///   Block 4: Power[0](137→9), MaxPower[0](147→19)
///   Block 5: VirtualItems(167-170→7-10), Stats(174-179→14-19),
///            StatPosBuff(180-184→20-24), StatNegBuff(185-189→25-29),
///            Resistances(190-191→30-31)
pub(super) fn write_unit_data_values_update(
    buf: &mut WorldPacket,
    virtual_item_changes: &[(u8, i32, u16, u16)],
    stat_changes: Option<&PlayerStatChanges>,
) {
    let mut blocks = [0u32; 8];

    // VirtualItems in block 5
    if !virtual_item_changes.is_empty() {
        blocks[5] |= 1 << 7; // parent bit 167
        for &(idx, _, _, _) in virtual_item_changes {
            if idx < 3 {
                blocks[5] |= 1 << (8 + idx);
            }
        }
    }

    // Stat change bits
    if stat_changes.is_some() {
        blocks[0] |= (1 << 0) | (1 << 5) | (1 << 6);
        blocks[1] |= (1 << 0) | (1 << 20) | (1 << 21);
        blocks[2] |= (1 << 0)
            | (1 << 11)
            | (1 << 12)
            | (1 << 17)
            | (1 << 18)
            | (1 << 19)
            | (1 << 20)
            | (1 << 21)
            | (1 << 22)
            | (1 << 23)
            | (1 << 24)
            | (1 << 27)
            | (1 << 28);
        blocks[3] |= (1 << 20) | (1 << 21) | (1 << 31);
        blocks[4] |= (1 << 9) | (1 << 19) | (1 << 29);
        blocks[5] |= (1 << 14)
            | (1 << 15)
            | (1 << 16)
            | (1 << 17)
            | (1 << 18)
            | (1 << 19)
            | (1 << 20)
            | (1 << 21)
            | (1 << 22)
            | (1 << 23)
            | (1 << 24)
            | (1 << 25)
            | (1 << 26)
            | (1 << 27)
            | (1 << 28)
            | (1 << 29)
            | (1 << 30)
            | (1 << 31);
    }

    let mut blocks_mask: u32 = 0;
    for i in 0..8 {
        if blocks[i] != 0 {
            blocks_mask |= 1 << i;
        }
    }

    buf.write_bits(blocks_mask, 8);
    for i in 0..8 {
        if blocks[i] != 0 {
            buf.write_bits(blocks[i], 32);
        }
    }

    // Dynamic arrays: block 0 bit 0 set enters the generated C++ dynamic-array
    // check, but bits 1-4 are NOT set, so nothing is written.
    buf.flush_bits();

    // ── Field values in generated C++ definition order ──
    // Blocks 0-4: only stat fields
    if let Some(sc) = stat_changes {
        // Block 0: Health, MaxHealth
        buf.write_int64(sc.health);
        buf.write_int64(sc.max_health);

        // Block 1: MinDamage, MaxDamage
        buf.write_float(sc.min_damage);
        buf.write_float(sc.max_damage);

        // Block 2: BaseMana, BaseHealth, AP base/modifiers, ranged AP
        //          base/modifiers, MinRangedDamage, MaxRangedDamage
        buf.write_int32(sc.base_mana);
        buf.write_int32(sc.base_health);
        buf.write_int32(sc.attack_power);
        buf.write_int32(sc.attack_power_mod_pos);
        buf.write_int32(sc.attack_power_mod_neg);
        buf.write_float(sc.attack_power_multiplier);
        buf.write_int32(sc.ranged_attack_power);
        buf.write_int32(sc.ranged_attack_power_mod_pos);
        buf.write_int32(sc.ranged_attack_power_mod_neg);
        buf.write_float(sc.ranged_attack_power_multiplier);
        buf.write_float(sc.min_ranged_damage);
        buf.write_float(sc.max_ranged_damage);

        // Blocks 3-4: Power interleaved loop (index 0)
        // C++ writes PowerRegenFlat, PowerRegenInterrupted, Power, MaxPower,
        // ModPowerRegen in generated update-field order.
        buf.write_float(sc.mana_regen); // PowerRegenFlatModifier[0]
        buf.write_float(sc.mana_regen_combat); // PowerRegenInterruptedFlatModifier[0]
        buf.write_int32(sc.power0); // Power[0]
        buf.write_int32(sc.max_power0); // MaxPower[0]
        buf.write_float(sc.mana_regen_mp5); // ModPowerRegen[0]
    }

    // Block 5: VirtualItems FIRST (bits 7-10), then Stats (14-24), then Resistances (30-31)
    for idx in 0..3u8 {
        if let Some(&(_, item_id, app_mod, item_visual)) =
            virtual_item_changes.iter().find(|&&(i, _, _, _)| i == idx)
        {
            buf.write_bits(0x0Fu32, 4);
            buf.flush_bits();
            buf.write_int32(item_id);
            buf.write_uint16(app_mod);
            buf.write_uint16(item_visual);
        }
    }

    // Stats/StatPosBuff/StatNegBuff are interleaved per index in generated C++
    // update-field order, then Resistances after VirtualItems in block 5.
    if let Some(sc) = stat_changes {
        for i in 0..5 {
            buf.write_int32(sc.stats[i]); // Stats[i]
            buf.write_int32(sc.stat_pos_buff[i]); // StatPosBuff[i]
            buf.write_int32(sc.stat_neg_buff[i]); // StatNegBuff[i]
        }
        buf.write_int32(sc.armor); // Resistances[0]
    }
}

pub fn write_stable_pet_info_values_update(
    buf: &mut WorldPacket,
    data: &StablePetInfoValuesUpdate,
) {
    let mask = data.stable_pet_mask;
    buf.write_bits(mask as u32, 8);

    buf.flush_bits();
    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_uint32(data.pet_slot);
        }
        if mask & 0x04 != 0 {
            buf.write_uint32(data.pet_number);
        }
        if mask & 0x08 != 0 {
            buf.write_uint32(data.creature_id);
        }
        if mask & 0x10 != 0 {
            buf.write_uint32(data.display_id);
        }
        if mask & 0x20 != 0 {
            buf.write_uint32(data.experience_level);
        }
        if mask & 0x80 != 0 {
            buf.write_uint8(data.pet_flags);
        }
        if mask & 0x40 != 0 {
            buf.write_bits(data.name.len() as u32, 8);
            buf.write_string(&data.name);
        }
    }
    buf.flush_bits();
}

/// Write a creature VALUES update block containing only health + max_health.
///
/// C++ `UF::UnitData::WriteUpdate` field positions:
///   `Health    = new(0, 5)` → block 0, bit 5
///   `MaxHealth = new(0, 6)` → block 0, bit 6
///   Bit 0 is the parent/dynamic-array indicator bit.
///
/// Wire format:
/// ```text
/// [u8]  UpdateType = 0 (Values)
/// [PackedGuid] creature GUID
/// [u32] data_size
///   [u32] ChangedObjectTypeMask = 1<<5 (TypeId::Unit)
///   UnitData block masks (8 words): only block 0 is non-zero = 0x61 (bits 0|5|6)
///   block 0 values: Health (i64), MaxHealth (i64)
/// ```
pub(super) fn write_creature_health_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    health: i64,
    max_health: i64,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();

    // ChangedObjectTypeMask: TypeId::Unit = 5 → bit 5 = 32
    val_buf.write_uint32(1 << 5);

    // UnitData section
    // 8 block words, only block 0 is set (bits 0, 5, 6).
    let block0: u32 = (1 << 0) | (1 << 5) | (1 << 6);
    // Emit: non-zero block mask (which blocks to include), then block 0 only.
    // The encoding is: 8-bit mask of which of the 8 words are present,
    // then the non-zero words in order.
    val_buf.write_bits(0x01u32, 8); // only block 0
    val_buf.write_bits(block0, 32);
    val_buf.flush_bits();

    // block 0 fields: Health (i64) then MaxHealth (i64).
    val_buf.write_int64(health);
    val_buf.write_int64(max_health);

    let data = val_buf.into_data();
    buf.write_uint32(data.len() as u32);
    buf.write_bytes(&data);
}
