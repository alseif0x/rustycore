// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! UpdateObject packet — used to create, update, and destroy game objects
//! in the client's view.
//!
//! Wire format matches RustyCore C# for WoW 3.4.3.54261.

use wow_constants::ServerOpcodes;
use wow_core::guid::TypeId;
use wow_core::{ObjectGuid, Position};

use crate::{ServerPacket, WorldPacket};

// ── UpdateType ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UpdateType {
    Values = 0,
    CreateObject = 1,
    CreateObject2 = 2,
}

// ── MovementBlock ───────────────────────────────────────────────────

/// Movement data included in a CreateObject block.
#[derive(Debug, Clone)]
pub struct MovementBlock {
    pub position: Position,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub run_back_speed: f32,
    pub swim_speed: f32,
    pub swim_back_speed: f32,
    pub fly_speed: f32,
    pub fly_back_speed: f32,
    pub turn_rate: f32,
    pub pitch_rate: f32,
}

impl Default for MovementBlock {
    fn default() -> Self {
        Self {
            position: Position::ZERO,
            walk_speed: 2.5,
            run_speed: 7.0,
            run_back_speed: 4.5,
            swim_speed: 4.72222,
            swim_back_speed: 2.5,
            fly_speed: 7.0,
            fly_back_speed: 4.5,
            turn_rate: std::f32::consts::PI,
            pitch_rate: std::f32::consts::PI,
        }
    }
}

// ── ItemCreateData ──────────────────────────────────────────────────

/// Data needed to build an Item CREATE_OBJECT block for the client.
///
/// Each equipped item must exist as a separate game object so the client
/// can display it in the character panel / inventory UI.
pub struct ItemCreateData {
    pub item_guid: ObjectGuid,
    pub entry_id: i32,
    pub owner_guid: ObjectGuid,
    pub contained_in: ObjectGuid,
}

// ── PlayerStatChanges ──────────────────────────────────────────────

/// Stat values for a VALUES update after equip/desequip.
///
/// Contains all UnitData fields that change when gear changes,
/// used by `UpdateObject::player_stat_update` to send a partial
/// VALUES update without recreating the whole player object.
#[derive(Debug, Clone, Copy)]
pub struct PlayerStatChanges {
    pub health: i64,
    pub max_health: i64,
    pub min_damage: f32,
    pub max_damage: f32,
    pub base_mana: i32,
    pub base_health: i32,
    pub attack_power: i32,
    pub ranged_attack_power: i32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub power0: i32,       // Mana/Rage/Energy current
    pub max_power0: i32,    // Mana/Rage/Energy max
    pub stats: [i32; 5],    // STR, AGI, STA, INT, SPI
    pub stat_pos_buff: [i32; 5], // gear bonuses shown as positive buffs
    pub armor: i32,         // Resistances[0] = Physical
    // ActivePlayerData secondary stats
    pub combat_ratings: [i32; 32], // CombatRatings[32] (indices per CombatRating enum, 0-24 used)
    pub spell_power: i32,          // ModDamageDonePos for magic schools 1-6
    // Percentage fields (server-computed, displayed by client)
    pub block_pct: f32,            // BlockPercentage (bit 41)
    pub dodge_pct: f32,            // DodgePercentage (bit 42)
    pub parry_pct: f32,            // ParryPercentage (bit 44)
    pub crit_pct: f32,             // CritPercentage (bit 46) — melee
    pub ranged_crit_pct: f32,      // RangedCritPercentage (bit 47)
    pub spell_crit_pct: [f32; 7],  // SpellCritPercentage[7] (bits 270-276)
    // UnitData: mana regen (parent 116 interleaved loop)
    pub mana_regen: f32,              // PowerRegenFlatModifier[0] (bit 117)
    pub mana_regen_combat: f32,       // PowerRegenInterruptedFlatModifier[0] (bit 127)
    pub mana_regen_mp5: f32,          // ModPowerRegen[0] (bit 157)
    // ActivePlayerData parent 0: expertise (bits 36-37)
    pub mainhand_expertise: f32,      // MainhandExpertise (bit 36)
    pub offhand_expertise: f32,       // OffhandExpertise (bit 37)
    // ActivePlayerData parent 38: extended fields (bits 39-69)
    pub ranged_expertise: f32,        // bit 39
    pub combat_rating_expertise: f32, // bit 40
    pub dodge_from_attr: f32,         // bit 43
    pub parry_from_attr: f32,         // bit 45
    pub offhand_crit_pct: f32,        // bit 48
    pub shield_block: i32,            // bit 49
    pub shield_block_crit_pct: f32,   // bit 50
    pub mod_healing_pct: f32,         // bit 60 (1.0)
    pub mod_healing_done_pct: f32,    // bit 61 (1.0)
    pub mod_periodic_healing_pct: f32,// bit 62 (1.0)
    pub mod_spell_power_pct: f32,     // bit 63 (1.0)
}

// ── PlayerCombatStats ──────────────────────────────────────────────

/// All combat-related stats computed from base stats + gear.
///
/// Passed as a single struct to `create_player` to avoid 20+ parameters.
#[derive(Debug, Clone, Copy)]
pub struct PlayerCombatStats {
    pub health: i64,
    pub max_health: i64,
    pub stats: [i32; 5],
    pub base_armor: i32,
    pub max_mana: i64,
    pub attack_power: i32,
    pub ranged_attack_power: i32,
    pub min_damage: f32,
    pub max_damage: f32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub dodge_pct: f32,
    pub parry_pct: f32,
    pub crit_pct: f32,
    pub ranged_crit_pct: f32,
    pub spell_crit_pct: f32,
}

impl Default for PlayerCombatStats {
    fn default() -> Self {
        Self {
            health: 100, max_health: 100,
            stats: [0; 5], base_armor: 0, max_mana: 60,
            attack_power: 0, ranged_attack_power: 0,
            min_damage: 1.0, max_damage: 2.0,
            min_ranged_damage: 0.0, max_ranged_damage: 0.0,
            dodge_pct: 0.0, parry_pct: 0.0,
            crit_pct: 5.0, ranged_crit_pct: 5.0, spell_crit_pct: 0.0,
        }
    }
}

// ── PlayerCreateData ────────────────────────────────────────────────

/// Data needed to build a full player create packet for the client.
pub struct PlayerCreateData {
    pub guid: ObjectGuid,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
    pub display_id: u32,
    pub native_display_id: u32,
    pub health: i64,
    pub max_health: i64,
    pub faction_template: i32,
    pub zone_id: u32,
    /// Primary stats: [STR, AGI, STA, INT, SPI].
    pub stats: [i32; 5],
    /// Base armor (AGI * 2).
    pub base_armor: i32,
    /// Max mana from level stats (for caster classes).
    pub max_mana: i64,
    /// Melee attack power.
    pub attack_power: i32,
    /// Ranged attack power.
    pub ranged_attack_power: i32,
    /// Melee min/max damage (unarmed base).
    pub min_damage: f32,
    pub max_damage: f32,
    /// Ranged min/max damage.
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    /// Dodge percentage.
    pub dodge_pct: f32,
    /// Parry percentage.
    pub parry_pct: f32,
    /// Melee crit percentage.
    pub crit_pct: f32,
    /// Ranged crit percentage.
    pub ranged_crit_pct: f32,
    /// Spell crit percentage (applied to all 7 schools).
    pub spell_crit_pct: f32,
    /// Visible equipment items (19 slots).
    /// Each entry: (ItemID, AppearanceModID, ItemVisual).
    /// Slots: Head(0), Neck(1), Shoulders(2), Shirt(3), Chest(4), Waist(5),
    /// Legs(6), Feet(7), Wrist(8), Hands(9), Finger1(10), Finger2(11),
    /// Trinket1(12), Trinket2(13), Cloak(14), MainHand(15), OffHand(16),
    /// Ranged(17), Tabard(18).
    pub visible_items: [(i32, u16, u16); 19],
    /// Inventory slots (141 entries) for ActivePlayerData.
    /// Slots 0-18 = equipped, 19-22 = bag containers, rest = backpack/bank.
    /// Each entry is an Item ObjectGuid (or EMPTY).
    pub inv_slots: [ObjectGuid; 141],
    /// Character's learned skills for the SkillInfo array (up to 256).
    /// Each entry: (skill_id, step, rank, starting_rank, max_rank, temp_bonus, perm_bonus).
    pub skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
    /// Current money in copper (Coinage field in ActivePlayerData).
    pub coinage: u64,
}

impl PlayerCreateData {
    /// Get the faction template for a race.
    pub fn faction_for_race(race: u8) -> i32 {
        match race {
            1 => 1,      // Human
            2 => 2,      // Orc
            3 => 3,      // Dwarf
            4 => 4,      // NightElf
            5 => 5,      // Undead
            6 => 6,      // Tauren
            7 => 115,    // Gnome
            8 => 116,    // Troll
            10 => 1610,  // BloodElf
            11 => 1629,  // Draenei
            22 => 1,     // Worgen → Human faction
            _ => 1,
        }
    }

    /// Get the power value for slot 0, using real mana for caster classes.
    ///
    /// - Warrior (1): rage = 1000 (stored as 10×)
    /// - Rogue (4): energy = 100
    /// - DK (6): runic power = 1000 (stored as 10×)
    /// - All others: mana from `max_mana` field (loaded from player_levelstats)
    fn power_for_slot0(&self) -> i32 {
        match self.class {
            1 => 1000,                   // Warrior: rage
            4 => 100,                    // Rogue: energy
            6 => 1000,                   // DK: runic power
            _ => self.max_mana as i32,   // Casters: real mana from DB
        }
    }

    /// Write the complete values block for CREATE (no change masks).
    ///
    /// Format: `[u32 size][u8 flags][ObjectData][UnitData][PlayerData][ActivePlayerData?]`
    pub fn write_values_create(&self, pkt: &mut WorldPacket, is_self: bool) {
        // Build into a temp buffer so we can prefix with size
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: Owner=0x01 for self, 0x00 for others
        let flags: u8 = if is_self { 0x01 } else { 0x00 };
        buf.write_uint8(flags);

        self.write_object_data(&mut buf);
        self.write_unit_data(&mut buf, flags);
        self.write_player_data(&mut buf, flags);
        if is_self {
            self.write_active_player_data(&mut buf);
        }

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32); // Size prefix
        pkt.write_bytes(&data);
    }

    // ── ObjectFieldData.WriteCreate ─────────────────────────────

    fn write_object_data(&self, buf: &mut WorldPacket) {
        buf.write_int32(0);    // EntryId (0 for players)
        buf.write_uint32(0);   // DynamicFlags
        buf.write_float(1.0);  // Scale
    }

    // ── UnitData.WriteCreate ────────────────────────────────────

    fn write_unit_data(&self, buf: &mut WorldPacket, flags: u8) {
        let is_owner = flags & 0x01 != 0;

        // Health / MaxHealth
        buf.write_int64(self.health);
        buf.write_int64(self.max_health);

        // DisplayId
        buf.write_int32(self.display_id as i32);

        // NpcFlags[2]
        buf.write_uint32(0);
        buf.write_uint32(0);

        // StateSpellVisualID, StateAnimID, StateAnimKitID
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // StateWorldEffectIDs.Count (dynamic array size = 0)
        buf.write_int32(0);

        // 10 PackedGuids: Charm, Summon, [Critter if Owner], CharmedBy,
        // SummonedBy, CreatedBy, DemonCreator, LookAtControllerTarget,
        // Target, BattlePetCompanionGUID
        write_empty_guid(buf); // Charm
        write_empty_guid(buf); // Summon
        if is_owner {
            write_empty_guid(buf); // Critter (only if Owner)
        }
        write_empty_guid(buf); // CharmedBy
        write_empty_guid(buf); // SummonedBy
        write_empty_guid(buf); // CreatedBy
        write_empty_guid(buf); // DemonCreator
        write_empty_guid(buf); // LookAtControllerTarget
        write_empty_guid(buf); // Target
        write_empty_guid(buf); // BattlePetCompanionGUID

        // BattlePetDBID
        buf.write_uint64(0);

        // ChannelData (UnitChannel.WriteCreate): SpellID + SpellXSpellVisualID
        buf.write_int32(0);
        buf.write_int32(0);

        // SummonedByHomeRealm
        buf.write_uint32(0);

        // Race, ClassId, PlayerClassId, Sex, DisplayPower
        buf.write_uint8(self.race);
        buf.write_uint8(self.class);
        buf.write_uint8(self.class); // PlayerClassId = same as ClassId for players
        buf.write_uint8(self.sex);
        buf.write_uint8(power_type_for_class(self.class)); // DisplayPower

        // OverrideDisplayPowerID
        buf.write_int32(0);

        // PowerRegen + PowerRegenInterrupted (Owner|UnitAll only)
        if is_owner {
            for _ in 0..10 {
                buf.write_float(0.0); // PowerRegenFlatModifier
                buf.write_float(0.0); // PowerRegenInterruptedFlatModifier
            }
        }

        // Power[10], MaxPower[10], ModPowerRegen[10]
        let power0 = self.power_for_slot0();
        for i in 0..10 {
            if i == 0 {
                buf.write_int32(power0);
                buf.write_int32(power0);
            } else {
                buf.write_int32(0);
                buf.write_int32(0);
            }
            buf.write_float(0.0); // ModPowerRegen
        }

        // Level, EffectiveLevel, ContentTuningID, Scaling fields (9x i32)
        buf.write_int32(self.level as i32);
        buf.write_int32(self.level as i32); // EffectiveLevel
        buf.write_int32(0); // ContentTuningID
        buf.write_int32(0); // ScalingLevelMin
        buf.write_int32(0); // ScalingLevelMax
        buf.write_int32(0); // ScalingLevelDelta
        buf.write_int32(0); // ScalingFactionGroup
        buf.write_int32(0); // ScalingHealthItemLevelCurveID
        buf.write_int32(0); // ScalingDamageItemLevelCurveID

        // FactionTemplate
        buf.write_int32(self.faction_template);

        // VirtualItems[3] — weapons visible on character model
        // [0]=MainHand(slot 15), [1]=OffHand(slot 16), [2]=Ranged(slot 17)
        for &slot in &[15usize, 16, 17] {
            let (item_id, appearance_mod, item_visual) = self.visible_items[slot];
            buf.write_int32(item_id);
            buf.write_uint16(appearance_mod);
            buf.write_uint16(item_visual);
        }

        // Flags, Flags2, Flags3, AuraState
        buf.write_uint32(0x0000_0008); // UnitFlags: UNIT_FLAG_PLAYER_CONTROLLED
        buf.write_uint32(0);            // Flags2
        buf.write_uint32(0);            // Flags3
        buf.write_uint32(0);            // AuraState

        // AttackRoundBaseTime[2]
        buf.write_uint32(2000); // MainHand
        buf.write_uint32(2000); // OffHand

        // RangedAttackRoundBaseTime (Owner only)
        if is_owner {
            buf.write_uint32(0);
        }

        // BoundingRadius, CombatReach, DisplayScale
        buf.write_float(0.306); // BoundingRadius (human default)
        buf.write_float(1.5);   // CombatReach
        buf.write_float(1.0);   // DisplayScale

        // NativeDisplayID, NativeXDisplayScale, MountDisplayID
        buf.write_int32(self.native_display_id as i32);
        buf.write_float(1.0); // NativeXDisplayScale
        buf.write_int32(0);   // MountDisplayID

        // MinDamage, MaxDamage, MinOffHandDamage, MaxOffHandDamage (Owner|Empath)
        if is_owner {
            buf.write_float(self.min_damage);
            buf.write_float(self.max_damage);
            buf.write_float(0.0); // MinOffHandDamage
            buf.write_float(0.0); // MaxOffHandDamage
        }

        // StandState, PetTalentPoints, VisFlags, AnimTier
        buf.write_uint8(0); // StandState (UNIT_STAND_STATE_STAND)
        buf.write_uint8(0); // PetTalentPoints
        buf.write_uint8(0); // VisFlags
        buf.write_uint8(0); // AnimTier

        // PetNumber, PetNameTimestamp, PetExperience, PetNextLevelExperience
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // ModCastingSpeed, ModSpellHaste, ModHaste, ModRangedHaste, ModHasteRegen, ModTimeRate
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);

        // CreatedBySpell, EmoteState
        buf.write_int32(0);
        buf.write_int32(0);

        // TrainingPointsUsed, TrainingPointsTotal (2x i16)
        buf.write_int16(0);
        buf.write_int16(0);

        // Stats[5], StatPosBuff[5], StatNegBuff[5] (Owner only)
        if is_owner {
            for i in 0..5 {
                buf.write_int32(self.stats[i]); // Stat
                buf.write_int32(0);              // StatPosBuff
                buf.write_int32(0);              // StatNegBuff
            }
        }

        // Resistances[7] (Owner|Empath): Physical, Holy, Fire, Nature, Frost, Shadow, Arcane
        if is_owner {
            buf.write_int32(self.base_armor); // [0] Physical = base armor
            for _ in 1..7 {
                buf.write_int32(0); // [1-6] spell resistances
            }
        }

        // PowerCostModifier[7], PowerCostMultiplier[7] (Owner only)
        if is_owner {
            for _ in 0..7 {
                buf.write_int32(0);   // PowerCostModifier
                buf.write_float(1.0); // PowerCostMultiplier
            }
        }

        // ResistanceBuffModsPositive[7], ResistanceBuffModsNegative[7]
        for _ in 0..7 {
            buf.write_int32(0); // Positive
            buf.write_int32(0); // Negative
        }

        // BaseMana — use real mana from stats store for caster classes
        buf.write_int32(self.power_for_slot0());

        // BaseHealth (Owner only)
        if is_owner {
            buf.write_int32(self.max_health as i32);
        }

        // SheatheState, PvpFlags, PetFlags, ShapeshiftForm
        buf.write_uint8(0); // SheatheState
        buf.write_uint8(0); // PvpFlags
        buf.write_uint8(0); // PetFlags
        buf.write_uint8(0); // ShapeshiftForm

        // AttackPower block (Owner only — 13 fields)
        if is_owner {
            buf.write_int32(self.attack_power);         // AttackPower
            buf.write_int32(0);                          // AttackPowerModPos
            buf.write_int32(0);                          // AttackPowerModNeg
            buf.write_float(1.0);                        // AttackPowerMultiplier
            buf.write_int32(self.ranged_attack_power);   // RangedAttackPower
            buf.write_int32(0);                          // RangedAttackPowerModPos
            buf.write_int32(0);                          // RangedAttackPowerModNeg
            buf.write_float(1.0);                        // RangedAttackPowerMultiplier
            buf.write_int32(0);                          // SetAttackSpeedAura
            buf.write_float(0.0);                        // Lifesteal
            buf.write_float(self.min_ranged_damage);     // MinRangedDamage
            buf.write_float(self.max_ranged_damage);     // MaxRangedDamage
            buf.write_float(1.0);                        // MaxHealthModifier
        }

        // HoverHeight + misc fields
        buf.write_float(1.0);  // HoverHeight
        buf.write_int32(0);    // MinItemLevelCutoff
        buf.write_int32(0);    // MinItemLevel
        buf.write_int32(0);    // MaxItemLevel
        buf.write_int32(0);    // WildBattlePetLevel
        buf.write_int32(0);    // BattlePetCompanionNameTimestamp
        buf.write_int32(0);    // InteractSpellId
        buf.write_int32(0);    // ScaleDuration
        buf.write_int32(0);    // LooksLikeMountID
        buf.write_int32(0);    // LooksLikeCreatureID
        buf.write_int32(0);    // LookAtControllerID
        buf.write_int32(0);    // PerksVendorItemID
        write_empty_guid(buf); // GuildGUID

        // Dynamic array sizes: PassiveSpells, WorldEffects, ChannelObjects
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        write_empty_guid(buf); // SkinningOwnerGUID

        // FlightCapabilityID, GlideEventSpeedDivisor, CurrentAreaID
        buf.write_int32(0);
        buf.write_float(0.0);
        buf.write_uint32(self.zone_id);

        // ComboTarget (Owner only)
        if is_owner {
            write_empty_guid(buf);
        }

        // Dynamic arrays (all empty — sizes were 0 above)
    }

    // ── PlayerData.WriteCreate ──────────────────────────────────

    fn write_player_data(&self, buf: &mut WorldPacket, flags: u8) {
        let _is_party = flags & 0x04 != 0; // PartyMember flag

        // 3 PackedGuids
        write_empty_guid(buf); // DuelArbiter
        write_empty_guid(buf); // WowAccount
        write_empty_guid(buf); // LootTargetGUID

        // PlayerFlags, PlayerFlagsEx
        buf.write_uint32(0);
        buf.write_uint32(0);

        // GuildRankID, GuildDeleteDate, GuildLevel
        buf.write_int32(0);
        buf.write_uint32(0);
        buf.write_int32(0);

        // Customizations.Size
        buf.write_int32(0);

        // PartyType[2]
        buf.write_uint8(0);
        buf.write_uint8(0);

        // NumBankSlots, NativeSex, Inebriation, PvpTitle, ArenaFaction, PvpRank
        buf.write_uint8(0);
        buf.write_uint8(self.sex);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // Field_88, DuelTeam, GuildTimeStamp
        buf.write_int32(0);
        buf.write_uint32(0);
        buf.write_int32(0);

        // QuestLog[25] — only if PartyMember flag set (skip for self on login)
        // We never set PartyMember flag for self-view, so skip.

        // VisibleItems[19] (each: i32 ItemID + u16 AppearanceModID + u16 ItemVisual)
        for &(item_id, appearance_mod, item_visual) in &self.visible_items {
            buf.write_int32(item_id);
            buf.write_uint16(appearance_mod);
            buf.write_uint16(item_visual);
        }

        // PlayerTitle, FakeInebriation, VirtualPlayerRealm, CurrentSpecID, TaxiMountAnimKitID
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_uint32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // AvgItemLevel[6]
        for _ in 0..6 {
            buf.write_float(0.0);
        }

        // CurrentBattlePetBreedQuality
        buf.write_uint8(0);

        // HonorLevel
        buf.write_int32(0);

        // LogoutTime
        buf.write_int64(0);

        // ArenaCooldowns.Size, CurrentBattlePetSpeciesID
        buf.write_int32(0);
        buf.write_int32(0);

        // BnetAccount
        write_empty_guid(buf);

        // VisualItemReplacements.Size
        buf.write_int32(0);

        // Field_3120[19]
        for _ in 0..19 {
            buf.write_uint32(0);
        }

        // Dynamic arrays (all empty — Customizations, ArenaCooldowns, VisualItemReplacements)

        // DungeonScoreSummary.Write:
        //   OverallScoreCurrentSeason(f32), LadderScoreCurrentSeason(f32), Runs.Count(i32)
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_int32(0);
    }

    // ── ActivePlayerData.WriteCreate ────────────────────────────

    fn write_active_player_data(&self, buf: &mut WorldPacket) {
        // InvSlots[141]
        for i in 0..141 {
            buf.write_packed_guid(&self.inv_slots[i]);
        }

        // FarsightObject, SummonedBattlePetGUID
        write_empty_guid(buf);
        write_empty_guid(buf);

        // KnownTitles.Size
        buf.write_uint32(0);

        // Coinage, XP, NextLevelXP, TrialXP
        buf.write_int64(self.coinage as i64);
        buf.write_int32(0);
        buf.write_int32(400); // NextLevelXP for level 1
        buf.write_int32(0);

        // SkillInfo.WriteCreate: 256 entries × 7 u16s each
        for i in 0..256 {
            if i < self.skill_info.len() {
                let (id, step, rank, start, max, temp, perm) = self.skill_info[i];
                buf.write_uint16(id);    // SkillLineID
                buf.write_uint16(step);  // SkillStep
                buf.write_uint16(rank);  // SkillRank
                buf.write_uint16(start); // SkillStartingRank
                buf.write_uint16(max);   // SkillMaxRank
                buf.write_int16(temp);   // SkillTempBonus
                buf.write_uint16(perm);  // SkillPermBonus
            } else {
                buf.write_uint16(0);
                buf.write_uint16(0);
                buf.write_uint16(0);
                buf.write_uint16(0);
                buf.write_uint16(0);
                buf.write_int16(0);
                buf.write_uint16(0);
            }
        }

        // CharacterPoints, MaxTalentTiers
        buf.write_int32(0);
        buf.write_int32(0);

        // TrackCreatureMask
        buf.write_uint32(0);

        // TrackResourceMask[2]
        buf.write_uint32(0);
        buf.write_uint32(0);

        // Expertise floats: Mainhand, Offhand, Ranged, CombatRating
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_float(0.0);

        // Block, Dodge, DodgeFromAttr, Parry, ParryFromAttr, Crit, RangedCrit, OffhandCrit
        buf.write_float(0.0);                // Block (need shield)
        buf.write_float(self.dodge_pct);     // Dodge
        buf.write_float(self.dodge_pct);     // DodgeFromAttr (same as dodge for display)
        buf.write_float(self.parry_pct);     // Parry
        buf.write_float(self.parry_pct);     // ParryFromAttr
        buf.write_float(self.crit_pct);      // CritPercentage
        buf.write_float(self.ranged_crit_pct); // RangedCritPercentage
        buf.write_float(self.crit_pct);      // OffhandCritPercentage

        // SpellCritPercentage[7], ModDamageDonePos[7], ModDamageDoneNeg[7], ModDamageDonePercent[7]
        for _ in 0..7 {
            buf.write_float(self.spell_crit_pct); // SpellCritPercentage per school
            buf.write_int32(0);                    // ModDamageDonePos (spell power from gear)
            buf.write_int32(0);                    // ModDamageDoneNeg
            buf.write_float(1.0);                  // ModDamageDonePercent
        }

        // ShieldBlock, ShieldBlockCritPercentage
        buf.write_int32(0);
        buf.write_float(0.0);

        // Mastery, Speed, Avoidance, Sturdiness
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_float(0.0);

        // Versatility, VersatilityBonus
        buf.write_int32(0);
        buf.write_float(0.0);

        // PvpPowerDamage, PvpPowerHealing
        buf.write_float(0.0);
        buf.write_float(0.0);

        // ExploredZones[240] (all zero u64s)
        for _ in 0..240 {
            buf.write_uint64(0);
        }

        // RestInfo[2] (each: i32 Threshold + u8 StateID)
        // StateID: 1=Rested, 2=Normal, 6=RAFLinked — must NOT be 0 (invalid)
        for _ in 0..2 {
            buf.write_int32(0);        // Threshold (no rest bonus)
            buf.write_uint8(2);        // StateID = Normal
        }

        // ModHealingDonePos, ModHealingPercent, ModHealingDonePercent, ModPeriodicHealingDonePercent
        buf.write_int32(0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);

        // WeaponDmgMultipliers[3], WeaponAtkSpeedMultipliers[3]
        for _ in 0..3 {
            buf.write_float(1.0); // WeaponDmgMultipliers
            buf.write_float(1.0); // WeaponAtkSpeedMultipliers
        }

        // ModSpellPowerPercent, ModResiliencePercent
        buf.write_float(1.0);
        buf.write_float(0.0);

        // OverrideSpellPowerByAPPercent, OverrideAPBySpellPowerPercent
        buf.write_float(-1.0);
        buf.write_float(-1.0);

        // ModTargetResistance, ModTargetPhysicalResistance
        buf.write_int32(0);
        buf.write_int32(0);

        // LocalFlags
        buf.write_uint32(0);

        // GrantableLevels, MultiActionBars, LifetimeMaxRank, NumRespecs
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // AmmoID, PvpMedals
        buf.write_int32(0);
        buf.write_uint32(0);

        // BuybackPrice[12] + BuybackTimestamp[12]
        for _ in 0..12 {
            buf.write_uint32(0); // BuybackPrice
            buf.write_int64(0);  // BuybackTimestamp
        }

        // HonorableKills/DishonorableKills (8x u16)
        buf.write_uint16(0); // TodayHonorableKills
        buf.write_uint16(0); // TodayDishonorableKills
        buf.write_uint16(0); // YesterdayHonorableKills
        buf.write_uint16(0); // YesterdayDishonorableKills
        buf.write_uint16(0); // LastWeekHonorableKills
        buf.write_uint16(0); // LastWeekDishonorableKills
        buf.write_uint16(0); // ThisWeekHonorableKills
        buf.write_uint16(0); // ThisWeekDishonorableKills

        // ThisWeekContribution, LifetimeHonorableKills, LifetimeDishonorableKills
        buf.write_uint32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // Field_F24, YesterdayContribution, LastWeekContribution, LastWeekRank
        buf.write_uint32(0);
        buf.write_uint32(0);
        buf.write_uint32(0);
        buf.write_uint32(0);

        // WatchedFactionIndex
        buf.write_int32(-1);

        // CombatRatings[32]
        for _ in 0..32 {
            buf.write_int32(0);
        }

        // MaxLevel, ScalingPlayerLevelDelta, MaxCreatureScalingLevel
        buf.write_int32(80);
        buf.write_int32(0);
        buf.write_int32(0);

        // NoReagentCostMask[4]
        for _ in 0..4 {
            buf.write_uint32(0);
        }

        // PetSpellPower
        buf.write_int32(0);

        // ProfessionSkillLine[2]
        buf.write_int32(0);
        buf.write_int32(0);

        // UiHitModifier, UiSpellHitModifier
        buf.write_float(0.0);
        buf.write_float(0.0);

        // HomeRealmTimeOffset
        buf.write_int32(0);

        // ModPetHaste
        buf.write_float(1.0);

        // LocalRegenFlags, AuraVision, NumBackpackSlots
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(16); // 16 default backpack slots

        // OverrideSpellsID, LfgBonusFactionID
        buf.write_int32(0);
        buf.write_int32(0);

        // LootSpecID
        buf.write_uint16(0);

        // OverrideZonePVPType
        buf.write_uint32(0);

        // BagSlotFlags[4]
        for _ in 0..4 {
            buf.write_uint32(0);
        }

        // BankBagSlotFlags[7]
        for _ in 0..7 {
            buf.write_uint32(0);
        }

        // QuestCompleted[875] (all zero u64s)
        for _ in 0..875 {
            buf.write_uint64(0);
        }

        // Honor, HonorNextLevel, Field_F74, PvpTierMaxFromWins, PvpLastWeeksTierMaxFromWins
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // PvpRankProgress
        buf.write_uint8(0);

        // PerksProgramCurrency
        buf.write_int32(0);

        // ResearchSites loop (1 iteration): 3 sizes (all 0) + no dynamic data
        buf.write_int32(0); // ResearchSites[0].Size()
        buf.write_int32(0); // ResearchSiteProgress[0].Size()
        buf.write_int32(0); // Research[0].Size()

        // DailyQuestsCompleted.Size, AvailableQuestLineXQuestIDs.Size, Field_1000.Size
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // Heirlooms.Size, HeirloomFlags.Size, Toys.Size, Transmog.Size
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // ConditionalTransmog.Size, SelfResSpells.Size, CharacterRestrictions.Size
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // SpellPctModByLabel.Size, SpellFlatModByLabel.Size, TaskQuests.Size
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // TransportServerTime
        buf.write_uint32(0);

        // TraitConfigs.Size
        buf.write_int32(0);

        // ActiveCombatTraitConfigID
        buf.write_int32(0);

        // GlyphSlots[6] + Glyphs[6]
        for _ in 0..6 {
            buf.write_int32(0); // GlyphSlots
            buf.write_int32(0); // Glyphs
        }

        // GlyphsEnabled, LfgRoles
        buf.write_uint8(0);
        buf.write_uint8(0);

        // CategoryCooldownMods.Size, WeeklySpellUses.Size
        buf.write_int32(0);
        buf.write_int32(0);

        // NumStableSlots
        buf.write_uint8(0);

        // Dynamic arrays: all empty (KnownTitles, DailyQuests, etc.) — sizes were 0

        // PvpInfo[7].WriteCreate (each: i8 Bracket + 16 i32/u32 fields + bit Disqualified)
        for _ in 0..7 {
            buf.write_int8(0);    // Bracket
            buf.write_int32(0);   // PvpRatingID
            buf.write_int32(0);   // WeeklyPlayed
            buf.write_int32(0);   // WeeklyWon
            buf.write_int32(0);   // SeasonPlayed
            buf.write_int32(0);   // SeasonWon
            buf.write_int32(0);   // Rating
            buf.write_int32(0);   // WeeklyBestRating
            buf.write_int32(0);   // SeasonBestRating
            buf.write_int32(0);   // PvpTierID
            buf.write_int32(0);   // WeeklyBestWinPvpTierID
            buf.write_uint32(0);  // Field_28
            buf.write_uint32(0);  // Field_2C
            buf.write_int32(0);   // WeeklyRoundsPlayed
            buf.write_int32(0);   // WeeklyRoundsWon
            buf.write_int32(0);   // SeasonRoundsPlayed
            buf.write_int32(0);   // SeasonRoundsWon
            buf.write_bit(false); // Disqualified
            buf.flush_bits();
        }

        // Trailing bits + FlushBits
        buf.flush_bits();

        // SortBagsRightToLeft, InsertItemsLeftToRight, PetStable has value
        buf.write_bit(false);
        buf.write_bit(false);
        buf.write_bits(0, 1); // PetStable.HasValue = false
        buf.flush_bits();

        // ResearchHistory.WriteCreate: CompletedProjects.Size (i32)
        buf.write_int32(0);

        // FrozenPerksVendorItem.Write: 7 i32 + 1 i64 + 1 bit
        buf.write_int32(0); // VendorItemID
        buf.write_int32(0); // MountID
        buf.write_int32(0); // BattlePetSpeciesID
        buf.write_int32(0); // TransmogSetID
        buf.write_int32(0); // ItemModifiedAppearanceID
        buf.write_int32(0); // Field_14
        buf.write_int32(0); // Field_18
        buf.write_int32(0); // Price
        buf.write_int64(0); // AvailableUntil
        buf.write_bit(false); // Disabled
        buf.flush_bits();

        // CharacterRestrictions (size 0, no data)
        // TraitConfigs (size 0, no data)
        // PetStable (not present)

        buf.flush_bits();
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Write an empty packed GUID (2 zero mask bytes).
fn write_empty_guid(buf: &mut WorldPacket) {
    buf.write_packed_guid(&ObjectGuid::EMPTY);
}

/// Get power type for a class (0=mana, 1=rage, 3=energy).
fn power_type_for_class(class: u8) -> u8 {
    match class {
        1 => 1,  // Warrior → Rage
        4 => 3,  // Rogue → Energy
        11 => 0, // Druid → Mana
        6 => 5,  // DeathKnight → Runic Power
        _ => 0,  // Default → Mana
    }
}

/// Get starting max power for a class at a given level.
fn max_power_for_class(class: u8, _level: u8) -> i32 {
    match class {
        1 => 1000, // Warrior: 1000 rage (stored as 10x)
        4 => 100,  // Rogue: 100 energy
        6 => 1000, // DK: 1000 runic power (stored as 10x)
        _ => 60,   // Casters: base mana
    }
}

// ── CreatureCreateData ──────────────────────────────────────────────

/// Data needed to build a creature create packet for the client.
pub struct CreatureCreateData {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub display_id: u32,
    pub native_display_id: u32,
    pub health: i64,
    pub max_health: i64,
    pub level: u8,
    pub faction_template: i32,
    pub npc_flags: u64,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_flags3: u32,
    pub scale: f32,
    pub unit_class: u8,
    pub base_attack_time: u32,
    pub ranged_attack_time: u32,
    pub zone_id: u32,
    /// Speed rate from creature_template.speed_walk (1.0 = default).
    pub speed_walk_rate: f32,
    /// Speed rate from creature_template.speed_run (1.14286 = default).
    pub speed_run_rate: f32,
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
        buf.write_uint32(0);                // DynamicFlags
        buf.write_float(self.scale);        // Scale
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

        // StateSpellVisualID, StateAnimID, StateAnimKitID
        buf.write_int32(0);
        buf.write_int32(0);
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
        buf.write_uint8(0); // DisplayPower (mana)

        // OverrideDisplayPowerID
        buf.write_int32(0);

        // NO PowerRegen (Owner-only)

        // Power[10], MaxPower[10], ModPowerRegen[10]
        for _ in 0..10 {
            buf.write_int32(0); // Power
            buf.write_int32(0); // MaxPower
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
        for _ in 0..3 {
            buf.write_int32(0);
            buf.write_uint16(0);
            buf.write_uint16(0);
        }

        // Flags, Flags2, Flags3, AuraState
        buf.write_uint32(self.unit_flags);
        buf.write_uint32(self.unit_flags2);
        buf.write_uint32(self.unit_flags3);
        buf.write_uint32(0); // AuraState

        // AttackRoundBaseTime[2]
        buf.write_uint32(self.base_attack_time);
        buf.write_uint32(self.base_attack_time);

        // NO RangedAttackRoundBaseTime (Owner-only)

        // BoundingRadius, CombatReach, DisplayScale
        buf.write_float(0.389); // BoundingRadius (common default)
        buf.write_float(1.5);   // CombatReach
        buf.write_float(1.0);   // DisplayScale

        // NativeDisplayID, NativeXDisplayScale, MountDisplayID
        buf.write_int32(self.native_display_id as i32);
        buf.write_float(1.0);
        buf.write_int32(0);

        // NO damage floats (Owner|Empath only)

        // StandState, PetTalentPoints, VisFlags, AnimTier
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // PetNumber, PetNameTimestamp, PetExperience, PetNextLevelExperience
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);
        buf.write_int32(0);

        // ModCastingSpeed, ModSpellHaste, ModHaste, ModRangedHaste, ModHasteRegen, ModTimeRate
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);
        buf.write_float(1.0);

        // CreatedBySpell, EmoteState
        buf.write_int32(0);
        buf.write_int32(0);

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
        buf.write_int32(0);

        // NO BaseHealth (Owner-only)

        // SheatheState, PvpFlags, PetFlags, ShapeshiftForm
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);
        buf.write_uint8(0);

        // NO AttackPower block (Owner-only)

        // HoverHeight + misc fields
        buf.write_float(1.0);
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
        buf.write_uint32(self.zone_id);

        // NO ComboTarget (Owner-only)
    }
}

// ── UpdateBlock ─────────────────────────────────────────────────────

// ── GameObjectCreateData ──────────────────────────────────────────

/// Data needed to build a gameobject create packet for the client.
pub struct GameObjectCreateData {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub display_id: u32,
    pub go_type: u8,
    pub position: Position,
    pub rotation: [f32; 4], // rotation0..3 (quaternion)
    pub anim_progress: u8,
    pub state: i8,
    pub faction_template: i32,
    pub scale: f32,
}

impl GameObjectCreateData {
    /// Write the values block for CREATE.
    ///
    /// For GameObjects: ObjectData + GameObjectFieldData (no UnitData/PlayerData).
    pub fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: 0x00 for non-owner
        buf.write_uint8(0x00);

        // ObjectFieldData.WriteCreate
        buf.write_int32(self.entry as i32); // EntryId
        buf.write_uint32(0);                 // DynamicFlags
        buf.write_float(self.scale);         // Scale

        // GameObjectFieldData.WriteCreate (matches C# GameObjectFieldData.WriteCreate)
        buf.write_int32(self.display_id as i32); // DisplayID
        buf.write_int32(0);                       // SpellVisualID
        buf.write_int32(0);                       // StateSpellVisualID
        buf.write_int32(0);                       // SpawnTrackingStateAnimID
        buf.write_int32(0);                       // SpawnTrackingStateAnimKitID
        buf.write_int32(0);                       // StateWorldEffectIDs.Count
        // No StateWorldEffectIDs entries (count=0)
        write_empty_guid(&mut buf);               // CreatedBy
        write_empty_guid(&mut buf);               // GuildGUID
        buf.write_uint32(0);                       // Flags
        // ParentRotation (Quaternion: x, y, z, w)
        // In C# this comes from gameobject_addon.parent_rotation, NOT from gameobject.rotation0-3.
        // For most GameObjects it's the identity quaternion (0, 0, 0, 1).
        // Only some transports have non-standard parent rotation.
        buf.write_float(0.0); // ParentRotation.X
        buf.write_float(0.0); // ParentRotation.Y
        buf.write_float(0.0); // ParentRotation.Z
        buf.write_float(1.0); // ParentRotation.W
        buf.write_int32(self.faction_template);    // FactionTemplate
        buf.write_uint32(0);                       // Level
        buf.write_int8(self.state);                // State
        buf.write_int8(self.go_type as i8);        // TypeID (gameobject type)
        buf.write_uint8(self.anim_progress);       // PercentHealth (anim progress)
        buf.write_int32(0);                        // ArtKit
        buf.write_int32(0);                        // EnableDoodadSets.Size
        buf.write_int32(0);                        // CustomParam
        buf.write_int32(0);                        // WorldEffects.Size
        // No EnableDoodadSets/WorldEffects entries

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }

    /// Pack the local rotation as a 64-bit integer for the Rotation flag.
    ///
    /// Matches C# `GameObject.UpdatePackedRotation()` exactly:
    /// ```csharp
    /// const int PACK_YZ = 1 << 20;          // 1,048,576
    /// const int PACK_X  = PACK_YZ << 1;     // 2,097,152
    /// const int PACK_YZ_MASK = (PACK_YZ << 1) - 1;  // 0x1FFFFF (21 bits)
    /// const int PACK_X_MASK  = (PACK_X << 1) - 1;   // 0x3FFFFF (22 bits)
    /// sbyte w_sign = (sbyte)(W >= 0 ? 1 : -1);
    /// long x = (int)(X * PACK_X)  * w_sign & PACK_X_MASK;
    /// long y = (int)(Y * PACK_YZ) * w_sign & PACK_YZ_MASK;
    /// long z = (int)(Z * PACK_YZ) * w_sign & PACK_YZ_MASK;
    /// result = z | (y << 21) | (x << 42);
    /// ```
    /// Layout: bits[0:20]=Z(21), bits[21:41]=Y(21), bits[42:63]=X(22).
    pub fn packed_rotation(&self) -> i64 {
        const PACK_YZ: i64 = 1 << 20;           // 1,048,576
        const PACK_X: i64 = PACK_YZ << 1;       // 2,097,152
        const PACK_YZ_MASK: i64 = (PACK_YZ << 1) - 1; // 0x1FFFFF
        const PACK_X_MASK: i64 = (PACK_X << 1) - 1;   // 0x3FFFFF

        // Normalize quaternion (C# SetLocalRotation does this before packing)
        let (rx, ry, rz, rw) = {
            let dot = self.rotation[0] * self.rotation[0]
                + self.rotation[1] * self.rotation[1]
                + self.rotation[2] * self.rotation[2]
                + self.rotation[3] * self.rotation[3];
            let inv_len = 1.0 / dot.sqrt();
            (
                self.rotation[0] * inv_len,
                self.rotation[1] * inv_len,
                self.rotation[2] * inv_len,
                self.rotation[3] * inv_len,
            )
        };

        let w_sign: i32 = if rw >= 0.0 { 1 } else { -1 };

        let x = ((rx * PACK_X as f32) as i32 as i64) * w_sign as i64 & PACK_X_MASK;
        let y = ((ry * PACK_YZ as f32) as i32 as i64) * w_sign as i64 & PACK_YZ_MASK;
        let z = ((rz * PACK_YZ as f32) as i32 as i64) * w_sign as i64 & PACK_YZ_MASK;

        z | (y << 21) | (x << 42)
    }
}

/// A single update block within an UpdateObject packet.
pub enum UpdateBlock {
    CreateObject {
        update_type: UpdateType,
        guid: ObjectGuid,
        type_id: TypeId,
        movement: Option<MovementBlock>,
        create_data: PlayerCreateData,
        is_self: bool,
    },
    CreateCreature {
        guid: ObjectGuid,
        movement: MovementBlock,
        create_data: CreatureCreateData,
    },
    CreateGameObject {
        guid: ObjectGuid,
        create_data: GameObjectCreateData,
    },
    CreateItem {
        guid: ObjectGuid,
        create_data: ItemCreateData,
    },
    /// VALUES update for a player: only changed InvSlots, VisibleItems, VirtualItems.
    PlayerValuesUpdate {
        guid: ObjectGuid,
        /// Changed InvSlots: (slot_index 0-140, new ObjectGuid or EMPTY).
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        /// Changed VisibleItems in PlayerData: (slot 0-18, item_id, appearance_mod, visual).
        visible_item_changes: Vec<(u8, i32, u16, u16)>,
        /// Changed VirtualItems in UnitData: (index 0-2 for MH/OH/Ranged, item_id, app, visual).
        virtual_item_changes: Vec<(u8, i32, u16, u16)>,
        /// Optional stat changes to include in UnitData section.
        stat_changes: Option<PlayerStatChanges>,
        /// Optional coinage update (ActivePlayerData.Coinage field, block 0 bit 28).
        coinage_change: Option<u64>,
    },
    /// VALUES update for a creature: only health and max health.
    CreatureHealthUpdate {
        guid: ObjectGuid,
        health: i64,
        max_health: i64,
    },
    /// Out-of-range destroy (removes object from client view without full destroy).
    DestroyOutOfRange {
        guid: ObjectGuid,
    },
}

// ── UpdateObject (SMSG_UPDATE_OBJECT) ───────────────────────────────

/// The main update packet used to create, update, or destroy objects.
///
/// Wire format (matches C# UpdateData.BuildPacket + UpdateObject.Write):
/// ```text
/// [u32] NumObjUpdates
/// [u16] MapID
/// [byte[]] Data — built from:
///   [bit] HasDestroyOrOutOfRange
///     if true: [u16 destroyCount][i32 totalCount][PackedGuid... destroy][PackedGuid... oor]
///   [i32] dataBlockSize
///   [bytes] concatenated update blocks
/// ```
pub struct UpdateObject {
    pub map_id: u16,
    pub num_updates: u32,
    pub destroy_guids: Vec<ObjectGuid>,
    pub out_of_range_guids: Vec<ObjectGuid>,
    pub blocks: Vec<UpdateBlock>,
}

impl UpdateObject {
    /// Create a creature spawn block.
    ///
    /// Speed rates from `creature_template` are multiplied by base speeds:
    /// walk = rate × 2.5, run = rate × 7.0.
    pub fn create_creature_block(
        create_data: CreatureCreateData,
        position: &Position,
    ) -> UpdateBlock {
        let walk_speed = create_data.speed_walk_rate * 2.5;
        let run_speed = create_data.speed_run_rate * 7.0;
        let movement = MovementBlock {
            position: *position,
            walk_speed,
            run_speed,
            ..Default::default()
        };
        UpdateBlock::CreateCreature {
            guid: create_data.guid,
            movement,
            create_data,
        }
    }

    /// Create a gameobject spawn block.
    pub fn create_gameobject_block(
        create_data: GameObjectCreateData,
    ) -> UpdateBlock {
        UpdateBlock::CreateGameObject {
            guid: create_data.guid,
            create_data,
        }
    }

    /// Create a batched UpdateObject with mixed blocks (creatures + gameobjects).
    pub fn create_world_objects(blocks: Vec<UpdateBlock>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: blocks.len() as u32,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }

    /// Create a batched UpdateObject with multiple creature blocks.
    pub fn create_creatures(blocks: Vec<UpdateBlock>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: blocks.len() as u32,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }

    /// Create a player create packet for login.
    pub fn create_player(
        guid: ObjectGuid,
        race: u8,
        class: u8,
        sex: u8,
        level: u8,
        display_id: u32,
        position: &Position,
        map_id: u16,
        zone_id: u32,
        is_self: bool,
        visible_items: [(i32, u16, u16); 19],
        inv_slots: [ObjectGuid; 141],
        combat: PlayerCombatStats,
        skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
        coinage: u64,
    ) -> Self {
        let faction = PlayerCreateData::faction_for_race(race);

        let create_data = PlayerCreateData {
            guid,
            race,
            class,
            sex,
            level,
            display_id,
            native_display_id: display_id,
            health: combat.health,
            max_health: combat.max_health,
            faction_template: faction,
            zone_id,
            stats: combat.stats,
            base_armor: combat.base_armor,
            max_mana: combat.max_mana,
            attack_power: combat.attack_power,
            ranged_attack_power: combat.ranged_attack_power,
            min_damage: combat.min_damage,
            max_damage: combat.max_damage,
            min_ranged_damage: combat.min_ranged_damage,
            max_ranged_damage: combat.max_ranged_damage,
            dodge_pct: combat.dodge_pct,
            parry_pct: combat.parry_pct,
            crit_pct: combat.crit_pct,
            ranged_crit_pct: combat.ranged_crit_pct,
            spell_crit_pct: combat.spell_crit_pct,
            visible_items,
            inv_slots,
            skill_info,
            coinage,
        };

        let movement = MovementBlock {
            position: *position,
            ..Default::default()
        };

        let type_id = if is_self {
            TypeId::ActivePlayer
        } else {
            TypeId::Player
        };

        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CreateObject {
                update_type: UpdateType::CreateObject2,
                guid,
                type_id,
                movement: Some(movement),
                create_data,
                is_self,
            }],
        }
    }

    /// Create a player VALUES update for changed inventory fields.
    ///
    /// Used when items are swapped/equipped/unequipped to update the client's
    /// InvSlots (ActivePlayerData) and VisibleItems (PlayerData) without
    /// recreating the entire player object.
    pub fn player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        visible_item_changes: Vec<(u8, i32, u16, u16)>,
        virtual_item_changes: Vec<(u8, i32, u16, u16)>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes,
                visible_item_changes,
                virtual_item_changes,
                stat_changes: None,
                coinage_change: None,
            }],
        }
    }

    /// Create a VALUES update for player coinage + optional inv slot change.
    ///
    /// Used after buy/sell to update the client's displayed gold and inventory.
    pub fn player_money_update(
        guid: ObjectGuid,
        map_id: u16,
        coinage: u64,
        inv_slot_change: Option<(u8, ObjectGuid)>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes: inv_slot_change.map(|c| vec![c]).unwrap_or_default(),
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: None,
                coinage_change: Some(coinage),
            }],
        }
    }

    /// Create a VALUES update for player stats only (after equip/desequip).
    pub fn player_stat_update(
        guid: ObjectGuid,
        map_id: u16,
        changes: PlayerStatChanges,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes: Vec::new(),
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: Some(changes),
                coinage_change: None,
            }],
        }
    }

    /// Create an UpdateObject with item CREATE blocks.
    ///
    /// Each item gets its own block. Sent BEFORE the player CREATE packet
    /// so the client has item objects when it processes InvSlots.
    pub fn create_items(items: Vec<ItemCreateData>, map_id: u16) -> Self {
        let num = items.len() as u32;
        let blocks = items
            .into_iter()
            .map(|data| {
                let guid = data.item_guid;
                UpdateBlock::CreateItem {
                    guid,
                    create_data: data,
                }
            })
            .collect();

        Self {
            map_id,
            num_updates: num,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }
}

impl ServerPacket for UpdateObject {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateObject;

    fn write(&self, pkt: &mut WorldPacket) {
        // Top level: NumObjUpdates + MapID
        pkt.write_uint32(self.num_updates);
        pkt.write_uint16(self.map_id);

        // Build the Data buffer (matches C# UpdateData.BuildPacket)
        let mut data_buf = WorldPacket::new_empty();

        // HasDestroyOrOutOfRange bit
        let has_destroy_or_oor =
            !self.destroy_guids.is_empty() || !self.out_of_range_guids.is_empty();
        data_buf.write_bit(has_destroy_or_oor);

        if has_destroy_or_oor {
            data_buf.write_uint16(self.destroy_guids.len() as u16);
            data_buf.write_int32(
                (self.destroy_guids.len() + self.out_of_range_guids.len()) as i32,
            );
            for g in &self.destroy_guids {
                data_buf.write_packed_guid(g);
            }
            for g in &self.out_of_range_guids {
                data_buf.write_packed_guid(g);
            }
        }

        // Build all update blocks into a separate buffer
        let mut blocks_buf = WorldPacket::new_empty();
        for block in &self.blocks {
            match block {
                UpdateBlock::CreateObject {
                    update_type,
                    guid,
                    type_id,
                    movement,
                    create_data,
                    is_self,
                } => {
                    write_create_block(
                        &mut blocks_buf,
                        *update_type,
                        guid,
                        *type_id,
                        movement.as_ref(),
                        create_data,
                        *is_self,
                    );
                }
                UpdateBlock::CreateCreature {
                    guid,
                    movement,
                    create_data,
                } => {
                    write_creature_create_block(
                        &mut blocks_buf,
                        guid,
                        movement,
                        create_data,
                    );
                }
                UpdateBlock::CreateGameObject {
                    guid,
                    create_data,
                } => {
                    write_gameobject_create_block(
                        &mut blocks_buf,
                        guid,
                        create_data,
                    );
                }
                UpdateBlock::CreateItem {
                    guid,
                    create_data,
                } => {
                    write_item_create_block(
                        &mut blocks_buf,
                        guid,
                        create_data,
                    );
                }
                UpdateBlock::PlayerValuesUpdate {
                    guid,
                    inv_slot_changes,
                    visible_item_changes,
                    virtual_item_changes,
                    stat_changes,
                    coinage_change,
                } => {
                    write_player_values_update_block(
                        &mut blocks_buf,
                        guid,
                        inv_slot_changes,
                        visible_item_changes,
                        virtual_item_changes,
                        stat_changes.as_ref(),
                        *coinage_change,
                    );
                }
                UpdateBlock::CreatureHealthUpdate { guid, health, max_health } => {
                    write_creature_health_update_block(&mut blocks_buf, guid, *health, *max_health);
                }
                UpdateBlock::DestroyOutOfRange { .. } => {
                    // Handled via destroy_guids / out_of_range_guids, not as a block.
                }
            }
        }

        let blocks_data = blocks_buf.into_data();
        data_buf.write_int32(blocks_data.len() as i32); // Data block size
        data_buf.write_bytes(&blocks_data);

        // Write the assembled Data buffer into the packet
        let assembled = data_buf.into_data();
        pkt.write_bytes(&assembled);
    }
}

/// Write a single CreateObject block.
fn write_create_block(
    buf: &mut WorldPacket,
    update_type: UpdateType,
    guid: &ObjectGuid,
    type_id: TypeId,
    movement: Option<&MovementBlock>,
    create_data: &PlayerCreateData,
    is_self: bool,
) {
    // UpdateType byte
    buf.write_uint8(update_type as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId byte
    buf.write_uint8(type_id as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────────
    let has_movement = movement.is_some();
    buf.write_bit(false);         // 0: NoBirthAnim
    buf.write_bit(false);         // 1: EnablePortals
    buf.write_bit(false);         // 2: PlayHoverAnim
    buf.write_bit(has_movement);  // 3: MovementUpdate
    buf.write_bit(false);         // 4: MovementTransport
    buf.write_bit(false);         // 5: Stationary
    buf.write_bit(false);         // 6: CombatVictim
    buf.write_bit(false);         // 7: ServerTime
    buf.write_bit(false);         // 8: Vehicle
    buf.write_bit(false);         // 9: AnimKit
    buf.write_bit(false);         // 10: Rotation
    buf.write_bit(false);         // 11: AreaTrigger
    buf.write_bit(false);         // 12: GameObject
    buf.write_bit(false);         // 13: SmoothPhasing
    buf.write_bit(is_self);       // 14: ThisIsYou
    buf.write_bit(false);         // 15: SceneObject
    buf.write_bit(is_self);       // 16: ActivePlayer
    buf.write_bit(false);         // 17: Conversation
    buf.flush_bits();

    // ── MovementUpdate block ───────────────────────────────────
    if let Some(mv) = movement {
        write_movement_update(buf, guid, mv);
    }

    // PauseTimes count (i32) — always 0, written after movement regardless of flags
    buf.write_int32(0);

    // No Stationary, CombatVictim, ServerTime, Vehicle, AnimKit, Rotation,
    // AreaTrigger, GameObject, SmoothPhasing, SceneObject blocks
    // (all flags are false)

    // MovementTransport block — not present (bit 4 = false)

    // ── ActivePlayer block (bit 16) ─────────────────────────────
    // C# BuildMovementUpdate writes this when flags.ActivePlayer is true.
    // Contains: 3 bits (HasSceneInstanceIDs, HasRuneState, HasActionButtons)
    //           + optional scene IDs, rune data, and 180 action buttons.
    if is_self {
        write_active_player_movement_block(buf);
    }

    // No Conversation block (bit 17 = false)

    // ── Values block ───────────────────────────────────────────
    create_data.write_values_create(buf, is_self);
}

/// Write the movement update block (when bit 3 = true).
fn write_movement_update(buf: &mut WorldPacket, guid: &ObjectGuid, mv: &MovementBlock) {
    // MoverGUID
    buf.write_packed_guid(guid);

    // MovementFlags, MovementFlags2, ExtraMovementFlags2
    buf.write_uint32(0);
    buf.write_uint32(0);
    buf.write_uint32(0);

    // MoveTime
    buf.write_uint32(0);

    // Position
    buf.write_float(mv.position.x);
    buf.write_float(mv.position.y);
    buf.write_float(mv.position.z);
    buf.write_float(mv.position.orientation);

    // Pitch
    buf.write_float(0.0);

    // StepUpStartElevation (f32, NOT u32!)
    buf.write_float(0.0);

    // RemoveForcesIDs.Count
    buf.write_uint32(0);

    // MoveIndex
    buf.write_uint32(0);

    // 7 conditional bits
    buf.write_bit(false); // HasStandingOnGameObjectGUID
    buf.write_bit(false); // HasTransport
    buf.write_bit(false); // HasFall
    buf.write_bit(false); // HasSpline
    buf.write_bit(false); // HeightChangeFailed
    buf.write_bit(false); // RemoteTimeValid
    buf.write_bit(false); // HasInertia
    // Note: no FlushBits here — we continue writing after conditional blocks

    // No transport, standing, inertia, advFlying, fall blocks (all bits false)

    // 9 movement speeds
    buf.write_float(mv.walk_speed);
    buf.write_float(mv.run_speed);
    buf.write_float(mv.run_back_speed);
    buf.write_float(mv.swim_speed);
    buf.write_float(mv.swim_back_speed);
    buf.write_float(mv.fly_speed);
    buf.write_float(mv.fly_back_speed);
    buf.write_float(mv.turn_rate);
    buf.write_float(mv.pitch_rate);

    // MovementForces count + modMagnitude
    buf.write_int32(0);
    buf.write_float(1.0);

    // 17 AdvancedFlying parameters (hardcoded defaults from C#)
    buf.write_float(2.0);   // airFriction
    buf.write_float(65.0);  // maxVel
    buf.write_float(1.0);   // liftCoefficient
    buf.write_float(3.0);   // doubleJumpVelMod
    buf.write_float(10.0);  // glideStartMinHeight
    buf.write_float(100.0); // addImpulseMaxSpeed
    buf.write_float(90.0);  // minBankingRate
    buf.write_float(140.0); // maxBankingRate
    buf.write_float(180.0); // minPitchingRateDown
    buf.write_float(360.0); // maxPitchingRateDown
    buf.write_float(90.0);  // minPitchingRateUp
    buf.write_float(270.0); // maxPitchingRateUp
    buf.write_float(30.0);  // minTurnVelThreshold
    buf.write_float(80.0);  // maxTurnVelThreshold
    buf.write_float(2.75);  // surfaceFriction
    buf.write_float(7.0);   // overMaxDeceleration
    buf.write_float(0.4);   // launchSpeedCoefficient

    // HasSplineData bit
    buf.write_bit(false);
    buf.flush_bits();

    // No movement forces, no spline data
}

/// The ActivePlayer block in BuildMovementUpdate (C# lines 733-768).
///
/// Written when the `ActivePlayer` bit (bit 16) is set in CreateObjectBits.
/// Contains 3 conditional bits, then optionally: scene instance IDs, rune state,
/// and 180 action buttons (4 bytes each = 720 bytes).
///
/// For a fresh player: HasSceneInstanceIDs=false, HasRuneState=false,
/// HasActionButtons=true, all 180 buttons = 0.
const MAX_ACTION_BUTTONS: usize = 180;

fn write_active_player_movement_block(buf: &mut WorldPacket) {
    // 3 bits: HasSceneInstanceIDs, HasRuneState, HasActionButtons
    buf.write_bit(false); // HasSceneInstanceIDs
    buf.write_bit(false); // HasRuneState
    buf.write_bit(true);  // HasActionButtons
    buf.flush_bits();

    // HasSceneInstanceIDs: if true, would write i32 count + i32[] IDs (skipped)
    // HasRuneState: if true, would write rune data (skipped)

    // HasActionButtons: 180 action buttons, each i32 (4 bytes)
    for _ in 0..MAX_ACTION_BUTTONS {
        buf.write_uint32(0); // No action buttons configured
    }
}

/// Write a single CreateObject block for a creature (TypeId::Unit).
fn write_creature_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    movement: &MovementBlock,
    create_data: &CreatureCreateData,
) {
    // UpdateType: CreateObject2 — always used when object appears for the first time
    // to a player (matches C# Map.AddToMap → SetIsNewObject(true) → CreateObject2).
    buf.write_uint8(UpdateType::CreateObject2 as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = Unit (5)
    buf.write_uint8(TypeId::Unit as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    buf.write_bit(false);        // 0: NoBirthAnim
    buf.write_bit(false);        // 1: EnablePortals
    buf.write_bit(false);        // 2: PlayHoverAnim
    buf.write_bit(true);         // 3: MovementUpdate (always true for Unit)
    buf.write_bit(false);        // 4: MovementTransport
    buf.write_bit(false);        // 5: Stationary
    buf.write_bit(false);        // 6: CombatVictim
    buf.write_bit(false);        // 7: ServerTime
    buf.write_bit(false);        // 8: Vehicle
    buf.write_bit(false);        // 9: AnimKit
    buf.write_bit(false);        // 10: Rotation
    buf.write_bit(false);        // 11: AreaTrigger
    buf.write_bit(false);        // 12: GameObject
    buf.write_bit(false);        // 13: SmoothPhasing
    buf.write_bit(false);        // 14: ThisIsYou (false for creatures)
    buf.write_bit(false);        // 15: SceneObject
    buf.write_bit(false);        // 16: ActivePlayer (false for creatures)
    buf.write_bit(false);        // 17: Conversation
    buf.flush_bits();

    // ── MovementUpdate block ───────────────────────────────
    write_movement_update(buf, guid, movement);

    // PauseTimes count
    buf.write_int32(0);

    // No ActivePlayer block (bit 16 = false)

    // ── Values block ───────────────────────────────────────
    create_data.write_values_create(buf);
}

/// Write a single CreateObject block for a gameobject (TypeId::GameObject).
///
/// GameObjects use: Stationary (bit 5) + Rotation (bit 10) + GameObject (bit 12) flags.
/// No MovementUpdate block.
fn write_gameobject_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &GameObjectCreateData,
) {
    // UpdateType: CreateObject2 — first appearance of this object to the client
    buf.write_uint8(UpdateType::CreateObject2 as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = GameObject (8)
    buf.write_uint8(TypeId::GameObject as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    buf.write_bit(false);        // 0: NoBirthAnim
    buf.write_bit(false);        // 1: EnablePortals
    buf.write_bit(false);        // 2: PlayHoverAnim
    buf.write_bit(false);        // 3: MovementUpdate (false for GOs)
    buf.write_bit(false);        // 4: MovementTransport
    buf.write_bit(true);         // 5: Stationary (true for GOs)
    buf.write_bit(false);        // 6: CombatVictim
    buf.write_bit(false);        // 7: ServerTime
    buf.write_bit(false);        // 8: Vehicle
    buf.write_bit(false);        // 9: AnimKit
    buf.write_bit(true);         // 10: Rotation (true for GOs)
    buf.write_bit(false);        // 11: AreaTrigger
    buf.write_bit(true);         // 12: GameObject (true for GOs)
    buf.write_bit(false);        // 13: SmoothPhasing
    buf.write_bit(false);        // 14: ThisIsYou
    buf.write_bit(false);        // 15: SceneObject
    buf.write_bit(false);        // 16: ActivePlayer
    buf.write_bit(false);        // 17: Conversation
    buf.flush_bits();

    // No MovementUpdate (bit 3 = false)

    // PauseTimes count (i32) — always 0
    buf.write_int32(0);

    // ── Stationary block (bit 5 = true) ─────────────────────
    buf.write_float(create_data.position.x);
    buf.write_float(create_data.position.y);
    buf.write_float(create_data.position.z);
    buf.write_float(create_data.position.orientation);

    // ── Rotation block (bit 10 = true) ──────────────────────
    buf.write_int64(create_data.packed_rotation());

    // ── GameObject block (bit 12 = true) ─────────────────────
    buf.write_int32(0);        // WorldEffectID
    buf.write_bit(false);      // has extra u32
    buf.flush_bits();

    // ── Values block ─────────────────────────────────────────
    create_data.write_values_create(buf);
}

/// Write a single CreateObject block for an Item (TypeId::Item).
///
/// Items have NO movement block, NO stationary, and all 18 bits are false.
/// Values = ObjectData + ItemData (with Owner conditional fields).
fn write_item_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &ItemCreateData,
) {
    // UpdateType: CreateObject2 — first appearance of item to the client
    buf.write_uint8(UpdateType::CreateObject2 as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = Item (1)
    buf.write_uint8(TypeId::Item as u8);

    // ── 18-bit CreateObjectBits (all false for items) ────
    for _ in 0..18 {
        buf.write_bit(false);
    }
    buf.flush_bits();

    // PauseTimes count (i32) — always 0
    buf.write_int32(0);

    // ── Values block ─────────────────────────────────────
    let mut val_buf = WorldPacket::new_empty();
    let flags: u8 = 0x01; // Owner
    val_buf.write_uint8(flags);

    // -- ObjectData (3 fields) --
    val_buf.write_int32(data.entry_id);   // EntryId
    val_buf.write_uint32(0);              // DynamicFlags
    val_buf.write_float(1.0);             // Scale

    // -- ItemData --
    // Owner, ContainedIn, Creator, GiftCreator
    val_buf.write_packed_guid(&data.owner_guid);
    val_buf.write_packed_guid(&data.contained_in);
    write_empty_guid(&mut val_buf);       // Creator
    write_empty_guid(&mut val_buf);       // GiftCreator

    // Owner conditional block 1
    val_buf.write_int32(1);               // StackCount
    val_buf.write_int32(0);               // Expiration
    for _ in 0..5 {
        val_buf.write_int32(0);           // SpellCharges[5]
    }

    // DynamicFlags
    val_buf.write_uint32(0);

    // 13 x ItemEnchantment (all zeros)
    for _ in 0..13 {
        val_buf.write_int32(0);           // ID
        val_buf.write_int32(0);           // Duration
        val_buf.write_int16(0);           // Charges
        val_buf.write_uint8(0);           // Field_A
        val_buf.write_uint8(0);           // Field_B
    }

    // PropertySeed, RandomPropertiesID
    val_buf.write_int32(0);
    val_buf.write_int32(0);

    // Owner conditional block 2
    val_buf.write_int32(0);               // Durability
    val_buf.write_int32(0);               // MaxDurability

    // CreatePlayedTime, Context, CreateTime
    val_buf.write_int32(0);
    val_buf.write_int32(0);
    val_buf.write_int64(0);

    // Owner conditional block 3
    val_buf.write_int64(0);               // ArtifactXP
    val_buf.write_uint8(0);               // ItemAppearanceModID

    // ArtifactPowers.Size, Gems.Size
    val_buf.write_int32(0);
    val_buf.write_int32(0);

    // Owner conditional block 4
    val_buf.write_uint32(0);              // DynamicFlags2

    // ItemBonusKey: ItemID + BonusCount
    val_buf.write_int32(0);               // ItemID
    val_buf.write_int32(0);               // BonusListIDs.Count

    // Owner conditional block 5
    val_buf.write_uint16(0);              // DEBUGItemLevel

    // ItemModList (dynamic) — 6 bits for size = 0, then FlushBits
    val_buf.write_bits(0, 6);
    val_buf.flush_bits();

    // Write values block with size prefix
    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

// ── VALUES update (UpdateType::Values) ─────────────────────────────

/// Write a player VALUES update block.
///
/// Wire format:
/// ```text
/// [u8]  UpdateType = 0 (Values)
/// [PackedGuid] player GUID
/// [u32] values data size
///   [u8] updateFieldFlags (0x01 = Owner)
///   ObjectData.WriteUpdate (4-bit mask, no changes)
///   UnitData.WriteUpdate (8 blocks, VirtualItems at bits 167-170)
///   PlayerData.WriteUpdate (4 blocks, VisibleItems at bits 61-80)
///   ActivePlayerData.WriteUpdate (48 blocks, InvSlots at bits 124-265)
/// ```
fn write_player_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    inv_slot_changes: &[(u8, ObjectGuid)],
    visible_item_changes: &[(u8, i32, u16, u16)],
    virtual_item_changes: &[(u8, i32, u16, u16)],
    stat_changes: Option<&PlayerStatChanges>,
    coinage_change: Option<u64>,
) {
    // UpdateType = Values (0)
    buf.write_uint8(UpdateType::Values as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // Build values data into temp buffer for size prefix.
    //
    // C# Player.BuildValuesUpdate writes:
    //   [u32] ChangedObjectTypeMask — which TypeId sections have changes
    //   [section data for each changed TypeId]
    //
    // TypeId enum: Object=0, Unit=5, Player=6, ActivePlayer=7
    let mut val_buf = WorldPacket::new_empty();

    // Compute which sections have changes
    let has_unit = !virtual_item_changes.is_empty() || stat_changes.is_some();
    let has_player = !visible_item_changes.is_empty();
    let has_active_player =
        !inv_slot_changes.is_empty() || stat_changes.is_some() || coinage_change.is_some();

    let mut type_mask: u32 = 0;
    if has_unit { type_mask |= 1 << 5; }           // TypeId::Unit = 5
    if has_player { type_mask |= 1 << 6; }         // TypeId::Player = 6
    if has_active_player { type_mask |= 1 << 7; }  // TypeId::ActivePlayer = 7

    val_buf.write_uint32(type_mask);

    // Write only sections that have changes (C# checks HasChanged per TypeId)
    if has_unit {
        write_unit_data_values_update(&mut val_buf, virtual_item_changes, stat_changes);
    }
    if has_player {
        write_player_data_values_update(&mut val_buf, visible_item_changes);
    }
    if has_active_player {
        write_active_player_data_values_update(
            &mut val_buf, inv_slot_changes, stat_changes, coinage_change,
        );
    }

    // Write with size prefix
    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

/// UnitData VALUES update: VirtualItems[3] and/or stat fields.
///
/// C# UnitData.WriteUpdate format:
///   WriteBits(blocksMask, 8) — which of 8 blocks have changes
///   for each active block: WriteBits(block, 32)
///   [dynamic arrays if block 0 active]
///   FlushBits()
///   [field values in C# field definition order]
///
/// Field write order (C# UnitData.WriteUpdate):
///   Block 0: Health(5), MaxHealth(6)
///   Block 1: MinDamage(52→20), MaxDamage(53→21)
///   Block 2: BaseMana(75→11), BaseHealth(76→12), AttackPower(81→17),
///            RangedAttackPower(85→21), MinRangedDamage(91→27), MaxRangedDamage(92→28)
///   Block 3: Power parent(116→20)
///   Block 4: Power[0](137→9), MaxPower[0](147→19)
///   Block 5: VirtualItems(167-170→7-10), Stats(174-179→14-19),
///            StatPosBuff(180-184→20-24), Resistances(190-191→30-31)
fn write_unit_data_values_update(
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
        blocks[2] |= (1 << 0) | (1 << 11) | (1 << 12) | (1 << 17) | (1 << 21) | (1 << 27) | (1 << 28);
        blocks[3] |= (1 << 20) | (1 << 21) | (1 << 31);
        blocks[4] |= (1 << 9) | (1 << 19) | (1 << 29);
        blocks[5] |= (1 << 14) | (1 << 15) | (1 << 16) | (1 << 17) | (1 << 18) | (1 << 19)
            | (1 << 20) | (1 << 21) | (1 << 22) | (1 << 23) | (1 << 24)
            | (1 << 30) | (1 << 31);
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

    // Dynamic arrays: block 0 bit 0 set → C# enters dynamic array check,
    // but bits 1-4 are NOT set, so nothing to write.
    buf.flush_bits();

    // ── Field values in C# definition order ──
    // Blocks 0-4: only stat fields
    if let Some(sc) = stat_changes {
        // Block 0: Health, MaxHealth
        buf.write_int64(sc.health);
        buf.write_int64(sc.max_health);

        // Block 1: MinDamage, MaxDamage
        buf.write_float(sc.min_damage);
        buf.write_float(sc.max_damage);

        // Block 2: BaseMana, BaseHealth, AttackPower, RangedAttackPower,
        //          MinRangedDamage, MaxRangedDamage
        buf.write_int32(sc.base_mana);
        buf.write_int32(sc.base_health);
        buf.write_int32(sc.attack_power);
        buf.write_int32(sc.ranged_attack_power);
        buf.write_float(sc.min_ranged_damage);
        buf.write_float(sc.max_ranged_damage);

        // Blocks 3-4: Power interleaved loop (index 0)
        // C# writes PowerRegenFlat, PowerRegenInterrupted, Power, MaxPower, ModPowerRegen
        buf.write_float(sc.mana_regen);        // PowerRegenFlatModifier[0]
        buf.write_float(sc.mana_regen_combat); // PowerRegenInterruptedFlatModifier[0]
        buf.write_int32(sc.power0);            // Power[0]
        buf.write_int32(sc.max_power0);        // MaxPower[0]
        buf.write_float(sc.mana_regen_mp5);   // ModPowerRegen[0]
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

    // Stats/StatPosBuff/StatNegBuff INTERLEAVED per index (C# lines 1728-1744),
    // then Resistances — after VirtualItems in block 5
    if let Some(sc) = stat_changes {
        for i in 0..5 {
            buf.write_int32(sc.stats[i]);          // Stats[i]
            buf.write_int32(sc.stat_pos_buff[i]);  // StatPosBuff[i]
            // StatNegBuff[i] bits not set → skip
        }
        buf.write_int32(sc.armor); // Resistances[0]
    }
}

/// PlayerData VALUES update: VisibleItems[19] (equipment display).
///
/// C# PlayerData.WriteUpdate format:
///   WriteBits(blocksMask, 4) — which of 4 blocks have changes
///   for each active block: WriteBits(block, 32)
///   WriteBit(noQuestLogChangesMask) — ALWAYS present after block masks
///   [dynamic array masks if block 0 active: Customizations, ArenaCooldowns, etc.]
///   FlushBits()
///   [dynamic array values]
///   [field values]
///   FlushBits() at end
///
/// VisibleItems: parent=61, elements=62-80. Span blocks 1-2.
fn write_player_data_values_update(
    buf: &mut WorldPacket,
    visible_item_changes: &[(u8, i32, u16, u16)],
) {
    let mut blocks = [0u32; 4];

    // Parent bit 61 = block 1 (61/32=1), bit 61%32=29
    blocks[1] |= 1 << 29;

    for &(slot, _, _, _) in visible_item_changes {
        if slot >= 19 { continue; }
        let bit = 62 + slot as u32;
        let block_idx = (bit / 32) as usize;
        let bit_in_block = bit % 32;
        if block_idx < 4 {
            blocks[block_idx] |= 1 << bit_in_block;
        }
    }

    let mut blocks_mask: u32 = 0;
    for i in 0..4 {
        if blocks[i] != 0 {
            blocks_mask |= 1 << i;
        }
    }

    buf.write_bits(blocks_mask, 4);
    for i in 0..4 {
        if blocks[i] != 0 {
            buf.write_bits(blocks[i], 32);
        }
    }

    // C# PlayerData.WriteUpdate ALWAYS writes this bit after block masks:
    // bool noQuestLogChangesMask = data.WriteBit(IsQuestLogChangesMaskSkipped());
    // For us, quest log never changed = true (skip it)
    buf.write_bit(true);

    // No dynamic arrays changed (block 0 is not set for VisibleItems-only changes)
    buf.flush_bits();

    // Write VisibleItem values in slot order
    for slot in 0..19u8 {
        if let Some(&(_, item_id, app_mod, item_visual)) =
            visible_item_changes.iter().find(|&&(s, _, _, _)| s == slot)
        {
            // VisibleItem.WriteUpdate: 4-bit mask + flush + data
            buf.write_bits(0x0Fu32, 4);
            buf.flush_bits();
            buf.write_int32(item_id);
            buf.write_uint16(app_mod);
            buf.write_uint16(item_visual);
        }
    }
    buf.flush_bits();
}

/// ActivePlayerData VALUES update: InvSlots[141] + combat stats.
///
/// C# ActivePlayerData.WriteUpdate format:
///   WriteUInt32(blocksMask group 0) — byte-aligned u32 for first 32 blocks
///   WriteBits(blocksMask group 1, 16) — 16 bits for remaining 16 blocks
///   for each active block: WriteBits(block, 32)
///   [bit fields and dynamic arrays if their blocks are active]
///   FlushBits()
///   [field values]
///
/// InvSlots: parent=124, elements=125-265. Span multiple blocks.
///
/// ActivePlayerData secondary stats (from stat_changes):
///   Parent 0:            bits 36-37 (expertise)  → block 0 bit 0, block 1 bits 4-5
///   Parent 38:           bits 39-69 (all 31 fields) → block 1 bits 6-31, block 2 bits 0-5
///   ModDamageDonePos[7]: parent=269, bits=277-283 → block 8 bits 13,21-27
///   CombatRatings[32]:   parent=574, bits=575-606 → block 17 bits 30-31, block 18 bits 0-30
///
/// C# WriteUpdate order: parent 0 → parent 38 → InvSlots(124) → ModDamageDone(269) → CombatRatings(574)
fn write_active_player_data_values_update(
    buf: &mut WorldPacket,
    inv_slot_changes: &[(u8, ObjectGuid)],
    stat_changes: Option<&PlayerStatChanges>,
    coinage_change: Option<u64>,
) {
    let mut blocks = [0u32; 48];

    // Coinage: block 0 bit 28 (ActivePlayerData.Coinage = new(0, 28))
    if coinage_change.is_some() {
        blocks[0] |= 1 << 28;
    }

    // InvSlots: parent bit 124 = block 3 bit 28
    if !inv_slot_changes.is_empty() {
        blocks[3] |= 1 << 28;
        for &(slot, _) in inv_slot_changes {
            if (slot as u32) >= 141 { continue; }
            let bit = 125 + slot as u32;
            let block_idx = (bit / 32) as usize;
            let bit_in_block = bit % 32;
            if block_idx < 48 {
                blocks[block_idx] |= 1 << bit_in_block;
            }
        }
    }

    // Secondary stats from stat_changes
    if stat_changes.is_some() {
        // Parent 0 section: MainhandExpertise(bit 36→b1:4), OffhandExpertise(bit 37→b1:5)
        blocks[0] |= 1 << 0;
        blocks[1] |= (1 << 4) | (1 << 5);

        // Parent 38 section: ALL 31 fields (bits 39-69)
        // parent=38→b1:6, bits 39-63→b1:7-31, bits 64-69→b2:0-5
        blocks[1] |= 0xFFFF_FFC0; // bits 6-31
        blocks[2] |= 0x3F;        // bits 0-5

        // Parent 269 section (block 8): SpellCritPercentage[7] + ModDamageDonePos[7]
        // parent=269→bit13, SpellCrit[0-6]=270-276→bits14-20, ModDmgPos[0-6]=277-283→bits21-27
        blocks[8] |= (1 << 13) | (0x7F << 14) | (0x7F << 21);

        // CombatRatings[32]: parent bit 574 (block 17 bit 30), CR[0] bit 575 (block 17 bit 31)
        blocks[17] |= (1 << 30) | (1 << 31);
        // CR[1-31]: bits 576-606 → block 18 bits 0-30
        blocks[18] |= 0x7FFF_FFFF;
    }

    // Group masks (which blocks have changes)
    let mut group0: u32 = 0;
    let mut group1: u32 = 0;
    for i in 0..32 {
        if blocks[i] != 0 { group0 |= 1 << i; }
    }
    for i in 32..48 {
        if blocks[i] != 0 { group1 |= 1 << (i - 32); }
    }

    // C#: WriteUInt32 for group 0 (byte-aligned), WriteBits for group 1 (16 bits)
    buf.write_uint32(group0);
    buf.write_bits(group1, 16);

    // Write block masks for blocks with changes
    for i in 0..48 {
        if blocks[i] != 0 {
            buf.write_bits(blocks[i], 32);
        }
    }

    // No dynamic arrays (bit 0 not set) — two FlushBits per C# WriteUpdate structure
    buf.flush_bits();

    // ── Field values in C# WriteUpdate order ──

    // Block 0: Coinage (bit 28) — written before all other ActivePlayerData fields.
    // C# ref: ActivePlayerData.Coinage = new(0, 28) → written in block-0 field pass.
    if let Some(coinage) = coinage_change {
        buf.write_int64(coinage as i64);
    }

    // Parent 0 section: expertise (bits 36-37) — BEFORE parent 38
    if let Some(sc) = stat_changes {
        buf.write_float(sc.mainhand_expertise);     // bit 36: MainhandExpertise
        buf.write_float(sc.offhand_expertise);      // bit 37: OffhandExpertise
    }

    // Parent 38 section: ALL 31 fields (bits 39-69) in C# definition order
    if let Some(sc) = stat_changes {
        buf.write_float(sc.ranged_expertise);        // bit 39: RangedExpertise
        buf.write_float(sc.combat_rating_expertise); // bit 40: CombatRatingExpertise
        buf.write_float(sc.block_pct);               // bit 41: BlockPercentage
        buf.write_float(sc.dodge_pct);               // bit 42: DodgePercentage
        buf.write_float(sc.dodge_from_attr);         // bit 43: DodgePercentageFromAttribute
        buf.write_float(sc.parry_pct);               // bit 44: ParryPercentage
        buf.write_float(sc.parry_from_attr);         // bit 45: ParryPercentageFromAttribute
        buf.write_float(sc.crit_pct);                // bit 46: CritPercentage
        buf.write_float(sc.ranged_crit_pct);         // bit 47: RangedCritPercentage
        buf.write_float(sc.offhand_crit_pct);        // bit 48: OffhandCritPercentage
        buf.write_int32(sc.shield_block);            // bit 49: ShieldBlock
        buf.write_float(sc.shield_block_crit_pct);   // bit 50: ShieldBlockCritPercentage
        buf.write_float(0.0);                        // bit 51: Mastery
        buf.write_float(0.0);                        // bit 52: Speed
        buf.write_float(0.0);                        // bit 53: Avoidance
        buf.write_float(0.0);                        // bit 54: Sturdiness
        buf.write_int32(0);                          // bit 55: Versatility
        buf.write_float(0.0);                        // bit 56: VersatilityBonus
        buf.write_float(0.0);                        // bit 57: PvpPowerDamage
        buf.write_float(0.0);                        // bit 58: PvpPowerHealing
        buf.write_int32(sc.spell_power);             // bit 59: ModHealingDonePos
        buf.write_float(sc.mod_healing_pct);         // bit 60: ModHealingPercent
        buf.write_float(sc.mod_healing_done_pct);    // bit 61: ModHealingDonePercent
        buf.write_float(sc.mod_periodic_healing_pct); // bit 62: ModPeriodicHealingDonePercent
        buf.write_float(sc.mod_spell_power_pct);     // bit 63: ModSpellPowerPercent
        buf.write_float(0.0);                        // bit 64: ModResiliencePercent
        buf.write_float(-1.0);                       // bit 65: OverrideSpellPowerByAPPercent
        buf.write_float(-1.0);                       // bit 66: OverrideAPBySpellPowerPercent
        buf.write_int32(0);                          // bit 67: ModTargetResistance
        buf.write_int32(0);                          // bit 68: ModTargetPhysicalResistance
        buf.write_uint32(0);                         // bit 69: LocalFlags
    }

    // Parent 124 section: InvSlots
    for slot in 0..141u8 {
        if let Some(&(_, ref guid)) =
            inv_slot_changes.iter().find(|&&(s, _)| s == slot)
        {
            buf.write_packed_guid(guid);
        }
    }

    // Parent 269 section: SpellCritPercentage[7] + ModDamageDonePos[7]
    // C# interleaves SpellCritPct/ModDmgDonePos/ModDmgDoneNeg/ModDmgDonePct per school.
    // Both SpellCritPct bits (270-276) and ModDmgDonePos bits (277-283) are set.
    if let Some(sc) = stat_changes {
        for i in 0..7 {
            buf.write_float(sc.spell_crit_pct[i]);  // SpellCritPercentage[i]
            if i == 0 {
                buf.write_int32(0); // Physical school: no spell power
            } else {
                buf.write_int32(sc.spell_power); // Magic schools 1-6
            }
            // ModDamageDoneNeg[i] bits 284-290: NOT set → skip
            // ModDamageDonePercent[i] bits 291-297: NOT set → skip
        }
    }

    // Parent 574 section: CombatRatings[0-31]
    if let Some(sc) = stat_changes {
        for i in 0..32 {
            buf.write_int32(sc.combat_ratings[i]);
        }
    }
}

/// Write a creature VALUES update block containing only health + max_health.
///
/// C# UnitData field positions:
///   `Health    = new(0, 5)` → block 0, bit 5
///   `MaxHealth = new(0, 6)` → block 0, bit 6
///   Bit 0 is the parent/dynamic-array indicator bit.
///
/// Wire format:
/// ```text
/// [u8]  UpdateType = 0 (Values)
/// [PackedGuid] creature GUID
/// [u32] data_size
///   [u8]  flags = 0x00 (not owner)
///   [u32] ChangedObjectTypeMask = 1<<5 (TypeId::Unit)
///   UnitData block masks (8 words): only block 0 is non-zero = 0x61 (bits 0|5|6)
///   block 0 values: Health (i64), MaxHealth (i64)
/// ```
fn write_creature_health_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    health: i64,
    max_health: i64,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();

    // UpdateFieldFlag: 0x00 (not the owner)
    val_buf.write_uint8(0x00);

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

impl UpdateObject {
    /// Build a single-creature health VALUES update packet.
    pub fn creature_health_update(
        guid: ObjectGuid,
        health: i64,
        max_health: i64,
        map_id: u16,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CreatureHealthUpdate { guid, health, max_health }],
        }
    }

    /// Build an UpdateObject that hard-destroys objects (they no longer exist).
    pub fn destroy_objects(guids: Vec<ObjectGuid>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: 0, // no create/update blocks
            destroy_guids: guids,
            out_of_range_guids: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// Build an UpdateObject that removes objects from the client's view
    /// because they moved out of range (they still exist in the world).
    /// C#: WorldObject.BuildOutOfRangeUpdateBlock → UpdateData.AddOutOfRangeGUID
    pub fn out_of_range_objects(guids: Vec<ObjectGuid>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: 0, // no create/update blocks
            destroy_guids: Vec::new(),
            out_of_range_guids: guids,
            blocks: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_object_create_player_serializes() {
        let guid = ObjectGuid::create_player(1, 42);
        let pos = Position::new(-8949.95, -132.493, 83.5312, 0.0);

        let pkt = UpdateObject::create_player(guid, 1, 1, 0, 1, 49, &pos, 0, 12, true, [(0, 0, 0); 19], [ObjectGuid::EMPTY; 141], PlayerCombatStats::default(), Vec::new());
        let bytes = pkt.to_bytes();
        // Should be a substantial packet (many KB with ActivePlayerData)
        assert!(bytes.len() > 1000, "Packet too small: {} bytes", bytes.len());
    }

    #[test]
    fn update_object_out_of_range() {
        let pkt = UpdateObject {
            map_id: 0,
            num_updates: 0,
            destroy_guids: Vec::new(),
            out_of_range_guids: vec![
                ObjectGuid::create_player(1, 1),
                ObjectGuid::create_player(1, 2),
            ],
            blocks: Vec::new(),
        };
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 10);
    }

    #[test]
    fn movement_block_default_speeds() {
        let mv = MovementBlock::default();
        assert!((mv.walk_speed - 2.5).abs() < 0.01);
        assert!((mv.run_speed - 7.0).abs() < 0.01);
        assert!((mv.swim_speed - 4.72222).abs() < 0.01);
    }

    #[test]
    fn player_create_data_faction() {
        assert_eq!(PlayerCreateData::faction_for_race(1), 1);     // Human
        assert_eq!(PlayerCreateData::faction_for_race(2), 2);     // Orc
        assert_eq!(PlayerCreateData::faction_for_race(10), 1610); // BloodElf
        assert_eq!(PlayerCreateData::faction_for_race(11), 1629); // Draenei
    }

    #[test]
    fn update_object_envelope_format() {
        // Verify the top-level format: opcode + NumObjUpdates + MapID + Data
        let guid = ObjectGuid::create_player(1, 1);
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let pkt = UpdateObject::create_player(guid, 1, 1, 0, 1, 49, &pos, 0, 12, true, [(0, 0, 0); 19], [ObjectGuid::EMPTY; 141], PlayerCombatStats::default(), Vec::new());
        let bytes = pkt.to_bytes();

        // opcode (2 bytes)
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);

        // NumObjUpdates (u32 at offset 2)
        let num_updates = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(num_updates, 1);

        // MapID (u16 at offset 6)
        let map_id = u16::from_le_bytes([bytes[6], bytes[7]]);
        assert_eq!(map_id, 0);
    }

    #[test]
    fn update_object_destroy_and_oor() {
        let pkt = UpdateObject {
            map_id: 0,
            num_updates: 0,
            destroy_guids: vec![ObjectGuid::create_player(1, 10)],
            out_of_range_guids: vec![ObjectGuid::create_player(1, 20)],
            blocks: Vec::new(),
        };
        let bytes = pkt.to_bytes();
        // Should contain destroy + oor data
        assert!(bytes.len() > 20);
    }

    #[test]
    fn create_player_non_self() {
        // Non-self player should be smaller (no ActivePlayerData)
        let guid = ObjectGuid::create_player(1, 42);
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let self_pkt = UpdateObject::create_player(guid, 1, 1, 0, 1, 49, &pos, 0, 12, true, [(0, 0, 0); 19], [ObjectGuid::EMPTY; 141], PlayerCombatStats::default(), Vec::new());
        let other_pkt = UpdateObject::create_player(guid, 1, 1, 0, 1, 49, &pos, 0, 12, false, [(0, 0, 0); 19], [ObjectGuid::EMPTY; 141], PlayerCombatStats::default(), Vec::new());
        let self_bytes = self_pkt.to_bytes();
        let other_bytes = other_pkt.to_bytes();
        // Self packet should be much larger due to ActivePlayerData
        assert!(
            self_bytes.len() > other_bytes.len() + 1000,
            "Self ({}) should be much larger than other ({})",
            self_bytes.len(),
            other_bytes.len()
        );
    }

    #[test]
    fn power_type_mapping() {
        assert_eq!(power_type_for_class(1), 1); // Warrior → Rage
        assert_eq!(power_type_for_class(2), 0); // Paladin → Mana
        assert_eq!(power_type_for_class(4), 3); // Rogue → Energy
        assert_eq!(power_type_for_class(6), 5); // DK → Runic Power
    }

    #[test]
    fn creature_create_serializes() {
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 1234, 5678,
        );
        let pos = Position::new(-8949.0, -132.0, 83.0, 0.0);
        let data = CreatureCreateData {
            guid,
            entry: 1234,
            display_id: 856,
            native_display_id: 856,
            health: 500,
            max_health: 500,
            level: 5,
            faction_template: 14,
            npc_flags: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 12,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
        };
        let block = UpdateObject::create_creature_block(data, &pos);
        let pkt = UpdateObject::create_creatures(vec![block], 0);
        let bytes = pkt.to_bytes();
        // Creature packet should be much smaller than player (no PlayerData/ActivePlayerData)
        assert!(bytes.len() > 100, "Creature packet too small: {} bytes", bytes.len());
        assert!(bytes.len() < 2000, "Creature packet too large: {} bytes", bytes.len());
    }

    #[test]
    fn creature_smaller_than_player() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let creature_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 100, 1,
        );
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);

        let player_pkt = UpdateObject::create_player(
            player_guid, 1, 1, 0, 1, 49, &pos, 0, 12, false,
            [(0, 0, 0); 19], [ObjectGuid::EMPTY; 141],
            PlayerCombatStats::default(), Vec::new(),
        );

        let creature_data = CreatureCreateData {
            guid: creature_guid,
            entry: 100,
            display_id: 856,
            native_display_id: 856,
            health: 100,
            max_health: 100,
            level: 1,
            faction_template: 14,
            npc_flags: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 12,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
        };
        let block = UpdateObject::create_creature_block(creature_data, &pos);
        let creature_pkt = UpdateObject::create_creatures(vec![block], 0);

        let player_bytes = player_pkt.to_bytes();
        let creature_bytes = creature_pkt.to_bytes();

        // Creature has no PlayerData, so it should be smaller than even a non-self player
        assert!(
            creature_bytes.len() < player_bytes.len(),
            "Creature ({}) should be smaller than non-self player ({})",
            creature_bytes.len(),
            player_bytes.len()
        );
    }

    #[test]
    fn creature_batched_multiple() {
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let mut blocks = Vec::new();
        for i in 0..5 {
            let guid = ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 100, i,
            );
            let data = CreatureCreateData {
                guid,
                entry: 100,
                display_id: 856,
                native_display_id: 856,
                health: 100,
                max_health: 100,
                level: 1,
                faction_template: 14,
                npc_flags: 0,
                unit_flags: 0,
                unit_flags2: 0,
                unit_flags3: 0,
                scale: 1.0,
                unit_class: 1,
                base_attack_time: 2000,
                ranged_attack_time: 0,
                zone_id: 12,
                speed_walk_rate: 1.0,
                speed_run_rate: 1.14286,
            };
            blocks.push(UpdateObject::create_creature_block(data, &pos));
        }
        let pkt = UpdateObject::create_creatures(blocks, 0);
        let bytes = pkt.to_bytes();

        // 5 creatures should be 5x the single creature data
        assert!(bytes.len() > 500, "Batched packet too small: {} bytes", bytes.len());

        // Check num_updates = 5
        let num_updates = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(num_updates, 5);
    }

    #[test]
    fn creature_npc_flags_written_correctly() {
        // Verify that NpcFlags value appears in the creature's values block.
        // NpcFlags=1 (Gossip) should be written as 0x01000000 in the packet.
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature, 0, 1, 0, 1, 3296, 1,
        );
        let pos = Position::new(1600.0, -4400.0, 10.0, 0.0);
        let data = CreatureCreateData {
            guid,
            entry: 3296,
            display_id: 4500,
            native_display_id: 4500,
            health: 500,
            max_health: 500,
            level: 55,
            faction_template: 85,
            npc_flags: 1, // Gossip flag
            unit_flags: 32768,
            unit_flags2: 2048,
            unit_flags3: 0,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 1637,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
        };
        let block = UpdateObject::create_creature_block(data, &pos);
        let pkt = UpdateObject::create_creatures(vec![block], 1);
        let bytes = pkt.to_bytes();

        // Find NpcFlags=1 in the packet bytes.
        // The values block contains:
        //   [u8 flags=0x00]
        //   [i32 EntryId] [u32 DynamicFlags] [f32 Scale]  (ObjectData: 4+4+4=12 bytes)
        //   [i64 Health] [i64 MaxHealth] [i32 DisplayId]   (UnitData: 8+8+4=20 bytes)
        //   [u32 NpcFlags[0]] [u32 NpcFlags[1]]            (UnitData: 4+4=8 bytes)
        // So NpcFlags[0] starts at offset 1+12+20 = 33 from values block start.
        // The value 1 in little-endian is [0x01, 0x00, 0x00, 0x00].
        // Search for this pattern preceded by DisplayId (4500 = 0x94110000 LE).
        let display_le = 4500u32.to_le_bytes();
        let npc_le = 1u32.to_le_bytes();
        let mut found = false;
        for i in 0..bytes.len().saturating_sub(8) {
            if bytes[i..i+4] == display_le && bytes[i+4..i+8] == npc_le {
                found = true;
                // Also check NpcFlags[1] = 0
                assert_eq!(bytes[i+8..i+12], [0, 0, 0, 0], "NpcFlags[1] should be 0");
                break;
            }
        }
        assert!(found, "NpcFlags=1 not found after DisplayId={} in packet ({} bytes). \
            This means NpcFlags are not being written correctly!", 4500, bytes.len());
    }

    #[test]
    fn active_player_movement_block_adds_721_bytes() {
        // Self-view packets include a 721-byte ActivePlayer block in
        // BuildMovementUpdate: 1 byte (3 bits + flush) + 180 action buttons (720 bytes).
        // Non-self packets don't have this block.
        let guid = ObjectGuid::create_player(1, 42);
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let self_pkt = UpdateObject::create_player(guid, 1, 1, 0, 1, 49, &pos, 0, 12, true, [(0, 0, 0); 19], [ObjectGuid::EMPTY; 141], PlayerCombatStats::default(), Vec::new());
        let other_pkt = UpdateObject::create_player(guid, 1, 1, 0, 1, 49, &pos, 0, 12, false, [(0, 0, 0); 19], [ObjectGuid::EMPTY; 141], PlayerCombatStats::default(), Vec::new());
        let self_bytes = self_pkt.to_bytes();
        let other_bytes = other_pkt.to_bytes();

        // The difference between self and non-self should include:
        // - 721 bytes from ActivePlayer movement block
        // - plus the ActivePlayerData values block difference
        // The ActivePlayer movement block alone is 721 bytes.
        let diff = self_bytes.len() - other_bytes.len();
        assert!(
            diff > 721,
            "Self/non-self difference ({}) should be > 721 (ActivePlayer block)",
            diff
        );
    }
}
