//! Player update-value structs state definitions, part 2 of 5.
//!
//! Separated from the player.rs root under #650. Behaviour is preserved.

use super::*;

impl PlayerCreateData {
    /// Get the faction template for a race.
    pub fn faction_for_race(race: u8) -> i32 {
        match race {
            1 => 1,     // Human
            2 => 2,     // Orc
            3 => 3,     // Dwarf
            4 => 4,     // NightElf
            5 => 5,     // Undead
            6 => 6,     // Tauren
            7 => 115,   // Gnome
            8 => 116,   // Troll
            10 => 1610, // BloodElf
            11 => 1629, // Draenei
            22 => 1,    // Worgen → Human faction
            _ => 1,
        }
    }

    /// Get the max power value for slot 0, using real mana for caster classes.
    ///
    /// - Warrior (1): rage = 1000 (stored as 10×)
    /// - Rogue (4): energy = 100
    /// - DK (6): runic power = 1000 (stored as 10×)
    /// - All others: mana from C++ `GtBaseMP`
    pub(in crate::packets::update) fn max_power_for_slot0(&self) -> i32 {
        match self.class {
            1 => 1000,                 // Warrior: rage
            4 => 100,                  // Rogue: energy
            6 => 1000,                 // DK: runic power
            _ => self.max_mana as i32, // Casters: real mana from DB
        }
    }

    pub(in crate::packets::update) fn current_power_for_slot0(&self) -> i32 {
        self.current_power0
            .clamp(0, self.max_power_for_slot0().max(0))
    }

    pub(in crate::packets::update) fn base_mana_for_create_like_cpp(&self) -> i32 {
        if power_type_for_class(self.class) == 0 {
            self.base_mana.max(0)
        } else {
            0
        }
    }

    /// Write the complete values block for CREATE (no change masks).
    ///
    /// Format: `[u32 size][u8 flags][ObjectData][UnitData][PlayerData][ActivePlayerData?]`
    pub fn write_values_create(&self, pkt: &mut WorldPacket, is_self: bool) {
        // Build into a temp buffer so we can prefix with size
        let mut buf = WorldPacket::new_empty();

        // C++ refs:
        // - `Player::BuildValuesCreate` writes TypeId sections in Object, Unit,
        //   Player, ActivePlayer order.
        // - `WorldObject::GetUpdateFieldFlagsFor` returns Owner|PartyMember
        //   for the self receiver, which enables self-only PlayerData fields.
        let flags: u8 = if is_self { 0x03 } else { 0x00 }; // 0x01=Owner 0x02=PartyMember
        buf.write_uint8(flags);

        let object_start = buf.data().len();
        self.write_object_data(&mut buf);
        let unit_start = buf.data().len();
        self.write_unit_data(&mut buf, flags);
        let player_start = buf.data().len();
        self.write_player_data(&mut buf, flags);
        let active_start = buf.data().len();
        if is_self {
            self.write_active_player_data(&mut buf);
        }
        let end_pos = buf.data().len();

        if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
            eprintln!(
                "RUST_UPDATEOBJECT player_values guid={:?} self={} flags=0x{:X} totalValues={} object={} unit={} player={} active={}",
                self.guid,
                is_self,
                flags,
                end_pos,
                unit_start - object_start,
                player_start - unit_start,
                active_start - player_start,
                end_pos - active_start
            );
        }

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32); // Size prefix
        pkt.write_bytes(&data);
    }

    // ── ObjectFieldData.WriteCreate ─────────────────────────────

    pub(super) fn write_object_data(&self, buf: &mut WorldPacket) {
        buf.write_int32(0); // EntryId (0 for players)
        buf.write_uint32(0); // DynamicFlags
        buf.write_float(1.0); // Scale
    }

    // ── UnitData.WriteCreate ────────────────────────────────────

    pub(in crate::packets::update) fn write_unit_data(&self, buf: &mut WorldPacket, flags: u8) {
        let is_owner = flags & 0x01 != 0;

        // Health / MaxHealth
        buf.write_int64(self.health);
        buf.write_int64(self.max_health);

        // DisplayId
        buf.write_int32(self.display_id as i32);

        // NpcFlags[2]
        buf.write_uint32(0);
        buf.write_uint32(0);

        // StateSpellVisualID, StateAnimID, StateAnimKitID.
        // C++ Player::Player (Player.cpp:22134) ALSO seeds StateAnimID with
        // DB2Manager::GetEmptyAnimStateID() = 1772 (DB2Stores.cpp:1765) — the Classic
        // client expects the retail AnimationData storage size for EVERY unit, including
        // the player itself. Shipping 0 makes the client index its AnimationData storage
        // out of range -> NULL deref in the render/anim worker (~4-5s in-world, ERROR #132)
        // when the player's own model animates. This is independent of nearby creatures.
        const EMPTY_ANIM_STATE_ID_LIKE_CPP: i32 = 1772;
        buf.write_int32(0);
        buf.write_int32(EMPTY_ANIM_STATE_ID_LIKE_CPP);
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
        let current_power0 = self.current_power_for_slot0();
        let max_power0 = self.max_power_for_slot0();
        for i in 0..10 {
            if i == 0 {
                buf.write_int32(current_power0);
                buf.write_int32(max_power0);
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
        buf.write_uint32(0); // Flags2
        buf.write_uint32(0); // Flags3
        // AuraState — C++ Unit::Update/ModifyAuraState applies health-based aura states to
        // EVERY alive unit incl. the player (Unit.cpp:469-476); full HP => 0x00D00000.
        buf.write_uint32(health_aura_state_like_cpp(
            self.health,
            self.max_health,
            self.health > 0,
        )); // AuraState

        // AttackRoundBaseTime[2]
        buf.write_uint32(2000); // MainHand
        buf.write_uint32(2000); // OffHand

        // RangedAttackRoundBaseTime (Owner only)
        if is_owner {
            buf.write_uint32(0);
        }

        // BoundingRadius, CombatReach, DisplayScale
        // C++ DEFAULT_PLAYER_BOUNDING_RADIUS = 0.388999998569489 (ObjectDefines.h:39),
        // set via Player::SetObjectScale -> SetBoundingRadius(scale * DEFAULT) (scale=1.0 here).
        buf.write_float(0.388_999_998_569_489); // BoundingRadius
        buf.write_float(1.5); // CombatReach
        buf.write_float(1.0); // DisplayScale

        // NativeDisplayID, NativeXDisplayScale, MountDisplayID
        buf.write_int32(self.native_display_id as i32);
        buf.write_float(1.0); // NativeXDisplayScale
        buf.write_int32(0); // MountDisplayID

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
        buf.write_int32(0);

        // TrainingPointsUsed, TrainingPointsTotal (2x i16)
        buf.write_int16(0);
        buf.write_int16(0);

        // Stats[5], StatPosBuff[5], StatNegBuff[5] (Owner only)
        if is_owner {
            for i in 0..5 {
                buf.write_int32(self.stats[i]); // Stat
                buf.write_int32(self.stat_pos_buff[i]); // StatPosBuff
                buf.write_int32(self.stat_neg_buff[i]); // StatNegBuff
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
                buf.write_int32(0); // PowerCostModifier
                buf.write_float(1.0); // PowerCostMultiplier
            }
        }

        // ResistanceBuffModsPositive[7], ResistanceBuffModsNegative[7]
        for _ in 0..7 {
            buf.write_int32(0); // Positive
            buf.write_int32(0); // Negative
        }

        // BaseMana — C++ GtBaseMP create mana for caster classes.
        buf.write_int32(self.base_mana_for_create_like_cpp());

        // C++ Player::InitStatsForLevel sets CreateHealth/BaseHealth to zero.
        if is_owner {
            buf.write_int32(0);
        }

        // SheatheState, PvpFlags, PetFlags, ShapeshiftForm
        buf.write_uint8(0); // SheatheState
        buf.write_uint8(0); // PvpFlags
        buf.write_uint8(0); // PetFlags
        buf.write_uint8(0); // ShapeshiftForm

        // AttackPower block (Owner only — 13 fields)
        if is_owner {
            buf.write_int32(self.attack_power); // AttackPower
            buf.write_int32(self.attack_power_mod_pos); // AttackPowerModPos
            buf.write_int32(0); // AttackPowerModNeg
            buf.write_float(0.0); // AttackPowerMultiplier
            buf.write_int32(self.ranged_attack_power); // RangedAttackPower
            buf.write_int32(self.ranged_attack_power_mod_pos); // RangedAttackPowerModPos
            buf.write_int32(0); // RangedAttackPowerModNeg
            buf.write_float(0.0); // RangedAttackPowerMultiplier
            buf.write_int32(0); // SetAttackSpeedAura
            buf.write_float(0.0); // Lifesteal
            buf.write_float(self.min_ranged_damage); // MinRangedDamage
            buf.write_float(self.max_ranged_damage); // MaxRangedDamage
            buf.write_float(1.0); // MaxHealthModifier
        }

        // HoverHeight + misc fields
        buf.write_float(1.0); // HoverHeight
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

        // ComboTarget (Owner only)
        if is_owner {
            write_empty_guid(buf);
        }

        // Dynamic arrays (all empty — sizes were 0 above)
    }

    // ── PlayerData.WriteCreate ──────────────────────────────────

    pub(in crate::packets::update) fn write_player_data(&self, buf: &mut WorldPacket, flags: u8) {
        let is_party = flags & 0x02 != 0; // UpdateFieldFlag::PartyMember = 0x02

        // 3 PackedGuids
        write_empty_guid(buf); // DuelArbiter
        buf.write_packed_guid(&self.wow_account); // WowAccount
        write_empty_guid(buf); // LootTargetGUID

        // PlayerFlags, PlayerFlagsEx
        buf.write_uint32(self.player_flags);
        buf.write_uint32(self.player_flags_ex);

        // GuildRankID, GuildDeleteDate, GuildLevel
        buf.write_int32(0);
        buf.write_uint32(0);
        buf.write_int32(0);

        // Customizations.Size
        buf.write_uint32(self.customizations.len() as u32);

        // PartyType[2]
        buf.write_uint8(self.party_type[0]);
        buf.write_uint8(self.party_type[1]);

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

        // QuestLog[25] — written when PartyMember flag is set.
        // For self-view, C++ `WorldObject::GetUpdateFieldFlagsFor` includes
        // `UpdateFieldFlag::PartyMember`.
        // C++ `UF::QuestLog::WriteCreate`: int64 EndTime + int32 QuestID
        // + uint32 StateFlags + uint16[24] ObjectiveProgress.
        if is_party {
            // Fill 25 slots; empty slots get quest_id=0
            let empty_slot: (u32, u32, i64, [u16; 24]) = (0, 0, 0, [0u16; 24]);
            for i in 0..25usize {
                let (quest_id, state_flags, end_time, obj_progress) =
                    self.quest_log.get(i).copied().unwrap_or(empty_slot);
                buf.write_int64(end_time); // EndTime (int64)
                buf.write_int32(quest_id as i32); // QuestID (int32)
                buf.write_uint32(state_flags); // StateFlags (uint32)
                for progress in &obj_progress {
                    // ObjectiveProgress[24] (uint16 each)
                    buf.write_uint16(*progress);
                }
            }
        }

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
        buf.write_packed_guid(&self.bnet_account);

        // VisualItemReplacements.Size
        buf.write_int32(0);

        // Field_3120[19]
        for _ in 0..19 {
            buf.write_uint32(0);
        }

        for customization in &self.customizations {
            write_chr_customization_choice_values_update(buf, customization);
        }

        // Dynamic arrays (empty — ArenaCooldowns, VisualItemReplacements)

        // DungeonScoreSummary.Write:
        //   OverallScoreCurrentSeason(f32), LadderScoreCurrentSeason(f32), Runs.Count(i32)
        buf.write_float(0.0);
        buf.write_float(0.0);
        buf.write_int32(0);
    }

    // ── ActivePlayerData.WriteCreate ────────────────────────────

    pub(in crate::packets::update) fn write_active_player_data(&self, buf: &mut WorldPacket) {
        let trace_sections = std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some();
        let active_base = buf.data().len();
        let trace = |buf: &WorldPacket, label: &str| {
            if trace_sections {
                eprintln!(
                    "RUST_UPDATEOBJECT active_section {label} offset={} size={}",
                    buf.data().len() - active_base,
                    buf.data().len()
                );
            }
        };

        // InvSlots[141]
        for i in 0..141 {
            buf.write_packed_guid(&self.inv_slots[i]);
        }
        trace(buf, "inv_slots");

        // FarsightObject, SummonedBattlePetGUID
        buf.write_packed_guid(&self.farsight_object);
        write_empty_guid(buf);
        trace(buf, "farsight_battlepet");

        // KnownTitles.Size
        buf.write_uint32(0);

        // Coinage, XP, NextLevelXP, TrialXP
        buf.write_int64(self.coinage as i64);
        buf.write_int32(self.xp);
        buf.write_int32(self.next_level_xp);
        buf.write_int32(0);

        // SkillInfo.WriteCreate: 256 entries × 7 u16s each
        for i in 0..256 {
            if i < self.skill_info.len() {
                let (id, step, rank, start, max, temp, perm) = self.skill_info[i];
                buf.write_uint16(id); // SkillLineID
                buf.write_uint16(step); // SkillStep
                buf.write_uint16(rank); // SkillRank
                buf.write_uint16(start); // SkillStartingRank
                buf.write_uint16(max); // SkillMaxRank
                buf.write_int16(temp); // SkillTempBonus
                buf.write_uint16(perm); // SkillPermBonus
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
        trace(buf, "skill");

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
        buf.write_float(self.block_pct); // Block
        buf.write_float(self.dodge_pct); // Dodge
        buf.write_float(self.dodge_from_attr); // DodgeFromAttr
        buf.write_float(self.parry_pct); // Parry
        buf.write_float(self.parry_from_attr); // ParryFromAttr
        buf.write_float(self.crit_pct); // CritPercentage
        buf.write_float(self.ranged_crit_pct); // RangedCritPercentage
        buf.write_float(self.offhand_crit_pct); // OffhandCritPercentage

        // SpellCritPercentage[7], ModDamageDonePos[7], ModDamageDoneNeg[7], ModDamageDonePercent[7]
        for school in 0..7 {
            buf.write_float(self.spell_crit_pct[school]); // SpellCritPercentage per school
            buf.write_int32(if school == 0 { 0 } else { self.spell_power }); // ModDamageDonePos
            buf.write_int32(0); // ModDamageDoneNeg
            buf.write_float(1.0); // ModDamageDonePercent
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
        trace(buf, "explored_zones");

        // RestInfo[2] (each: i32 Threshold + u8 StateID)
        // StateID: 1=Rested, 2=Normal, 6=RAFLinked — must NOT be 0 (invalid)
        for rest_info in self.rest_info {
            buf.write_int32(rest_info.threshold as i32);
            buf.write_uint8(rest_info.state_id);
        }

        // ModHealingDonePos, ModHealingPercent, ModHealingDonePercent, ModPeriodicHealingDonePercent
        buf.write_int32(self.spell_power);
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
            buf.write_int64(0); // BuybackTimestamp
        }
        trace(buf, "buyback");

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
        buf.write_int32(self.watched_faction_index);

        // CombatRatings[32]
        for rating in self.combat_ratings {
            buf.write_int32(rating);
        }
        trace(buf, "combat_ratings");

        // MaxLevel, ScalingPlayerLevelDelta, MaxCreatureScalingLevel
        buf.write_int32(self.max_level);
        buf.write_int32(self.scaling_player_level_delta);
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
        trace(buf, "quest_completed");

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
        buf.write_int32(self.heirlooms.len() as i32);
        buf.write_int32(self.heirloom_flags.len() as i32);
        buf.write_int32(self.toys.len() as i32);
        buf.write_int32(self.transmog.len() as i32);

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
        buf.write_int32(self.trait_configs.len() as i32);

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
        trace(buf, "dynamic_sizes");

        for value in &self.heirlooms {
            buf.write_int32(*value);
        }
        for value in &self.heirloom_flags {
            buf.write_uint32(*value);
        }
        for value in &self.toys {
            buf.write_int32(*value);
        }
        for value in &self.transmog {
            buf.write_uint32(*value);
        }
        trace(buf, "dynamic_payloads");

        // Remaining dynamic arrays are empty (KnownTitles, DailyQuests, etc.).

        // PvpInfo[7].WriteCreate (each: i8 Bracket + 16 i32/u32 fields + bit Disqualified)
        for _ in 0..7 {
            buf.write_int8(0); // Bracket
            buf.write_int32(0); // PvpRatingID
            buf.write_int32(0); // WeeklyPlayed
            buf.write_int32(0); // WeeklyWon
            buf.write_int32(0); // SeasonPlayed
            buf.write_int32(0); // SeasonWon
            buf.write_int32(0); // Rating
            buf.write_int32(0); // WeeklyBestRating
            buf.write_int32(0); // SeasonBestRating
            buf.write_int32(0); // PvpTierID
            buf.write_int32(0); // WeeklyBestWinPvpTierID
            buf.write_uint32(0); // Field_28
            buf.write_uint32(0); // Field_2C
            buf.write_int32(0); // WeeklyRoundsPlayed
            buf.write_int32(0); // WeeklyRoundsWon
            buf.write_int32(0); // SeasonRoundsPlayed
            buf.write_int32(0); // SeasonRoundsWon
            buf.write_bit(false); // Disqualified
            buf.flush_bits();
        }
        trace(buf, "pvp_info");

        // Trailing bits + FlushBits
        buf.flush_bits();

        // SortBagsRightToLeft, InsertItemsLeftToRight, PetStable has value
        buf.write_bit(false);
        buf.write_bit(false);
        buf.write_bits(0, 1); // PetStable.HasValue = false
        buf.flush_bits();

        // ResearchHistory.WriteCreate: CompletedProjects.Size (i32)
        buf.write_int32(0);
        trace(buf, "research_history");

        // FrozenPerksVendorItem.Write: 8 i32 + 1 i64 + 1 bit
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
        trace(buf, "frozen_perks");

        // CharacterRestrictions (size 0, no data)
        for trait_config in &self.trait_configs {
            write_trait_config_create_data(buf, trait_config);
        }
        // PetStable (not present)

        buf.flush_bits();
        trace(buf, "end");
    }
}
