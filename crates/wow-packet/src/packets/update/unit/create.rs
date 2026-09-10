//! Create packets.
//!
//! Separated from unit.rs under #689.

use super::*;

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

pub(in crate::packets::update) fn debug_creature_create_values_len_like_cpp(
    data: &CreatureCreateData,
) -> usize {
    let mut values = WorldPacket::new_empty();
    data.write_values_create(&mut values);
    values.into_data().len()
}

/// Write a single CreateObject block for a creature (TypeId::Unit).
pub(in crate::packets::update) fn write_creature_create_block(
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

pub(in crate::packets::update) fn write_stationary_world_object_create_prefix_like_cpp(
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
