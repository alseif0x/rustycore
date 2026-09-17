//! `aura_types` constants, moved out of the spell module root under #646.
//!
//! Every value is unchanged.

pub const SPELL_AURA_CONTROL_VEHICLE: i32 = 236;
pub const SPELL_AURA_DUMMY: i32 = 0;
/// C++ `AuraType::SPELL_AURA_SCHOOL_ABSORB`.
pub const SPELL_AURA_SCHOOL_ABSORB: i32 = 69;
pub const SPELL_AURA_SCHOOL_IMMUNITY: i32 = 39;
pub const SPELL_AURA_DUMMY_ABSORB: i32 = 3;
pub const SPELL_AURA_PERIODIC_DAMAGE: i32 = 3;
pub const SPELL_AURA_MOD_CONFUSE: i32 = 5;
pub const SPELL_AURA_MOD_FEAR: i32 = 7;
pub const SPELL_AURA_PERIODIC_HEAL: i32 = 8;
pub const SPELL_AURA_MOD_THREAT: i32 = 10;
pub const SPELL_AURA_MOD_TAUNT: i32 = 11;
pub const SPELL_AURA_MOD_STUN: i32 = 12;
pub const SPELL_AURA_MOD_DAMAGE_DONE: i32 = 13;
pub const SPELL_AURA_MOD_DAMAGE_TAKEN: i32 = 14;
pub const SPELL_AURA_MOD_STEALTH: i32 = 16;
pub const SPELL_AURA_MOD_STEALTH_DETECT: i32 = 17;
pub const SPELL_AURA_MOD_INVISIBILITY: i32 = 18;
pub const SPELL_AURA_MOD_RESISTANCE: i32 = 22;
/// C++ `AuraType::SPELL_AURA_MOD_BASE_RESISTANCE` (`SpellAuraDefines.h:178`):
/// flat resistance added through the `TOTAL_VALUE` unit modifier, the same
/// route as `SPELL_AURA_MOD_RESISTANCE`.
pub const SPELL_AURA_MOD_BASE_RESISTANCE: i32 = 83;
/// C++ `AuraType::SPELL_AURA_MOD_RESISTANCE_PCT` (`SpellAuraDefines.h:196`):
/// `TOTAL_PCT` resistance multiplier selected by the school mask.
pub const SPELL_AURA_MOD_RESISTANCE_PCT: i32 = 101;
/// C++ `AuraType::SPELL_AURA_MOD_BASE_RESISTANCE_PCT`
/// (`SpellAuraDefines.h:237`): `BASE_PCT` resistance multiplier.
pub const SPELL_AURA_MOD_BASE_RESISTANCE_PCT: i32 = 142;
/// C++ `AuraType::SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT`
/// (`SpellAuraDefines.h:277`): percentage of the `MiscValueB` stat added as
/// resistance for the schools in `MiscValue`.
pub const SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT: i32 = 182;
/// C++ `AuraType::SPELL_AURA_MOD_BONUS_ARMOR_PCT` (`SpellAuraDefines.h:561`):
/// final multiplier on bonus armor.
pub const SPELL_AURA_MOD_BONUS_ARMOR_PCT: i32 = 466;
pub const SPELL_AURA_MOD_ROOT: i32 = 26;
pub const SPELL_AURA_MOD_SILENCE: i32 = 27;
pub const SPELL_AURA_MOD_STAT: i32 = 29;
pub const SPELL_AURA_REFLECT_SPELLS: i32 = 28;
pub const SPELL_AURA_MOD_INCREASE_SPEED: i32 = 31;
pub const SPELL_AURA_MODIFY_DAMAGE_PERCENT_TAKEN: i32 = 31;
pub const SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED: i32 = 32;
pub const SPELL_AURA_MOD_DECREASE_SPEED: i32 = 33;
pub const SPELL_AURA_MOD_INCREASE_HEALTH: i32 = 34;
pub const SPELL_AURA_MOD_SHAPESHIFT: i32 = 36;
pub const SPELL_AURA_DAMAGE_IMMUNITY: i32 = 40;
pub const SPELL_AURA_PROC_TRIGGER_SPELL: i32 = 42;
pub const SPELL_AURA_PROC_TRIGGER_DAMAGE: i32 = 43;
/// C++ `AuraType::SPELL_AURA_MOD_PARRY_PERCENT` (`SpellAuraDefines.h:173`):
/// flat parry percentage fed into `Player::UpdateParryPercentage`.
pub const SPELL_AURA_MOD_PARRY_PERCENT: i32 = 47;
/// C++ `AuraType::SPELL_AURA_MOD_DODGE_PERCENT` (`SpellAuraDefines.h:175`):
/// flat dodge percentage fed into `Player::UpdateDodgePercentage`.
pub const SPELL_AURA_MOD_DODGE_PERCENT: i32 = 49;
pub const SPELL_AURA_MOD_BLOCK_PERCENT: i32 = 51;
pub const SPELL_AURA_MOD_WEAPON_CRIT_PERCENT: i32 = 52;
pub const SPELL_AURA_MOD_HIT_CHANCE: i32 = 54;
pub const SPELL_AURA_TRANSFORM: i32 = 56;
pub const SPELL_AURA_MOD_SPELL_CRIT_CHANCE: i32 = 57;
/// C++ `AuraType::SPELL_AURA_MOD_CRIT_PCT` (`SpellAuraDefines.h:385`): flat
/// critical percentage added to every weapon and spell critical chance.
pub const SPELL_AURA_MOD_CRIT_PCT: i32 = 290;
pub const SPELL_AURA_MOD_INCREASE_SWIM_SPEED: i32 = 58;
pub const SPELL_AURA_MOD_SCALE: i32 = 61;
pub const SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK: i32 = 65;
/// C++ `AuraType::SPELL_AURA_HASTE_SPELLS` (`SpellAuraDefines.h:287`), handled
/// like `SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK` for the cast time.
pub const SPELL_AURA_HASTE_SPELLS: i32 = 216;
pub const SPELL_AURA_MOD_POWER_COST_SCHOOL_PCT: i32 = 72;
/// C++ `AuraType::SPELL_AURA_MOD_POWER_COST_SCHOOL` (`SpellAuraDefines.h:168`).
pub const SPELL_AURA_MOD_POWER_COST_SCHOOL: i32 = 73;
pub const SPELL_AURA_REFLECT_SPELLS_SCHOOL: i32 = 74;
pub const SPELL_AURA_MECHANIC_IMMUNITY: i32 = 77;
pub const SPELL_AURA_MOUNTED: i32 = 78;
pub const SPELL_AURA_MOD_DAMAGE_PERCENT_DONE: i32 = 79;
/// C++ `AuraType::SPELL_AURA_MOD_REGEN` (`SpellAuraDefines.h:179`).
pub const SPELL_AURA_MOD_REGEN: i32 = 84;
pub const SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN: i32 = 87;
/// C++ `AuraType::SPELL_AURA_MOD_HEALTH_REGEN_PERCENT` (`SpellAuraDefines.h:183`).
pub const SPELL_AURA_MOD_HEALTH_REGEN_PERCENT: i32 = 88;
pub const SPELL_AURA_PERIODIC_DAMAGE_PERCENT: i32 = 89;
pub const SPELL_AURA_MOD_DETECT_RANGE: i32 = 91;
pub const SPELL_AURA_SPELL_MAGNET: i32 = 96;
pub const SPELL_AURA_MOD_ATTACK_POWER: i32 = 99;
/// C++ `AuraType::SPELL_AURA_MOD_RANGED_ATTACK_POWER`
/// (`SpellAuraDefines.h:219`): flat ranged attack power, skipped for
/// `CLASSMASK_WAND_USERS`.
pub const SPELL_AURA_MOD_RANGED_ATTACK_POWER: i32 = 124;
pub const SPELL_AURA_ADD_FLAT_MODIFIER: i32 = 107;
pub const SPELL_AURA_ADD_PCT_MODIFIER: i32 = 108;
pub const SPELL_AURA_MOD_POWER_REGEN: i32 = 85;
pub const SPELL_AURA_MOD_POWER_REGEN_PERCENT: i32 = 110;
pub const SPELL_AURA_INTERCEPT_MELEE_RANGED_ATTACKS: i32 = 111;
pub const SPELL_AURA_OVERRIDE_CLASS_SCRIPTS: i32 = 112;
/// C++ `AuraType::SPELL_AURA_MOD_REGEN_DURING_COMBAT` (`SpellAuraDefines.h:211`).
pub const SPELL_AURA_MOD_REGEN_DURING_COMBAT: i32 = 116;
pub const SPELL_AURA_MOD_MECHANIC_RESISTANCE: i32 = 117;
pub const SPELL_AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS: i32 = 127;
pub const SPELL_AURA_MOD_SPEED_ALWAYS: i32 = 129;
pub const SPELL_AURA_MOD_INCREASE_HEALTH_PERCENT: i32 = 133;
pub const SPELL_AURA_MOD_MANA_REGEN_INTERRUPT: i32 = 134;
pub const SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS: i32 = 130;
pub const SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE: i32 = 137;
/// C++ `AuraType::SPELL_AURA_MOD_ATTACKSPEED` (`SpellAuraDefines.h:104`):
/// `AuraEffect::HandleModAttackSpeed` (`SpellAuraEffects.cpp:4353-4361`) scales
/// the main-hand attack time.
pub const SPELL_AURA_MOD_ATTACKSPEED: i32 = 9;
pub const SPELL_AURA_MOD_MELEE_HASTE: i32 = 138;
pub const SPELL_AURA_FORCE_REACTION: i32 = 139;
pub const SPELL_AURA_MOD_RANGED_HASTE: i32 = 140;
pub const SPELL_AURA_MOD_DETECTED_RANGE: i32 = 152;
pub const SPELL_AURA_MOD_REPUTATION_GAIN: i32 = 156;
/// C++ `AuraType::SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT` (`SpellAuraDefines.h:256`).
pub const SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT: i32 = 161;
pub const SPELL_AURA_MOD_ATTACK_POWER_PCT: i32 = 166;
/// C++ `AuraType::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE`
/// (`SpellAuraDefines.h:154`): `Unit::MeleeDamageBonusDone` (`Unit.cpp:7568`)
/// adds the effect amount whose `GetMiscValue` intersects the victim's creature
/// type mask.
pub const SPELL_AURA_MOD_DAMAGE_DONE_CREATURE: i32 = 59;
/// C++ `AuraType::SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS`
/// (`SpellAuraDefines.h:197`): the melee attack-power bonus by creature type
/// (`Unit.cpp:7584`).
pub const SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS: i32 = 102;
/// C++ `AuraType::SPELL_AURA_MOD_RANGED_ATTACK_POWER_VERSUS`
/// (`SpellAuraDefines.h:226`): the ranged attack-power bonus by creature type
/// (`Unit.cpp:7580`).
pub const SPELL_AURA_MOD_RANGED_ATTACK_POWER_VERSUS: i32 = 131;
/// C++ `AuraType::SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT`
/// (`SpellAuraDefines.h:262`): ranged attack power percentage, skipped for
/// `CLASSMASK_WAND_USERS`.
pub const SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT: i32 = 167;
/// C++ `AuraType::SPELL_AURA_MOD_POWER_DISPLAY` (`SpellAuraDefines.h:274`):
/// `Unit::CalculateDisplayPowerType` (`Unit.cpp:5568-5573`) selects the
/// displayed power type from the first active effect's `GetMiscValue`.
pub const SPELL_AURA_MOD_POWER_DISPLAY: i32 = 179;
pub const SPELL_AURA_MOD_SPEED_NOT_STACK: i32 = 171;
pub const SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK: i32 = 172;
pub const SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE: i32 = 184;
pub const SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH: i32 = 183;
pub const SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER: i32 = 306;
pub const SPELL_AURA_MOD_ATTACKER_MELEE_CRIT_CHANCE: i32 = 187;
pub const SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE: i32 = 197;
pub const SPELL_AURA_MOD_COMBAT_RESULT_CHANCE: i32 = 248;
pub const SPELL_AURA_MOD_ENEMY_DODGE: i32 = 251;
pub const SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED: i32 = 191;
pub const SPELL_AURA_MOD_MELEE_RANGED_HASTE: i32 = 192;
/// C++ `AuraType::SPELL_AURA_MELEE_SLOW` (`SpellAuraDefines.h:288`):
/// `AuraEffect::HandleModCombatSpeedPct` (`SpellAuraEffects.cpp:4330-4351`)
/// scales every attack time (and the cast time).
pub const SPELL_AURA_MELEE_SLOW: i32 = 193;
/// C++ `AuraType::SPELL_AURA_MOD_MELEE_HASTE_2` (`SpellAuraDefines.h:312`),
/// handled like `SPELL_AURA_MOD_MELEE_HASTE`.
pub const SPELL_AURA_MOD_MELEE_HASTE_2: i32 = 217;
/// C++ `AuraType::SPELL_AURA_MOD_SPEED_SLOW_ALL` (`SpellAuraDefines.h:347`),
/// handled like `SPELL_AURA_MELEE_SLOW`.
pub const SPELL_AURA_MOD_SPEED_SLOW_ALL: i32 = 252;
/// C++ `AuraType::SPELL_AURA_MOD_MELEE_RANGED_HASTE_2`
/// (`SpellAuraDefines.h:437`), handled like `SPELL_AURA_MOD_MELEE_RANGED_HASTE`.
pub const SPELL_AURA_MOD_MELEE_RANGED_HASTE_2: i32 = 342;
/// C++ `AuraType::SPELL_AURA_MOD_XP_PCT`.
pub const SPELL_AURA_MOD_XP_PCT: i32 = 200;
pub const SPELL_AURA_FLY: i32 = 201;
pub const SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED: i32 = 206;
pub const SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED: i32 = 207;
pub const SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED: i32 = 208;
pub const SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS: i32 = 209;
pub const SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK: i32 = 211;
pub const SPELL_AURA_ADD_PCT_MODIFIER_BY_SPELL_LABEL: i32 = 218;
pub const SPELL_AURA_MOD_MANA_REGEN_FROM_STAT: i32 = 219;
pub const SPELL_AURA_MOD_DETAUNT: i32 = 221;
pub const SPELL_AURA_PERIODIC_DUMMY: i32 = 226;
pub const SPELL_AURA_PROC_TRIGGER_SPELL_WITH_VALUE: i32 = 231;
pub const SPELL_AURA_MOD_EXPERTISE: i32 = 240;
/// C++ `AuraType::SPELL_AURA_PREVENT_REGENERATE_POWER` (`SpellAuraDefines.h:389`).
/// Its effect amount is the `Powers` value whose regeneration it blocks.
pub const SPELL_AURA_PREVENT_REGENERATE_POWER: i32 = 294;
pub const SPELL_AURA_ABILITY_IGNORE_AURASTATE: i32 = 262;
pub const SPELL_AURA_MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER: i32 = 270;
pub const SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER: i32 = 271;
pub const SPELL_AURA_PROVIDE_SPELL_FOCUS: i32 = 281;
/// C++ `AuraType::SPELL_AURA_PREVENT_DURABILITY_LOSS` (`SpellAuraDefines.h:384`).
pub const SPELL_AURA_PREVENT_DURABILITY_LOSS: i32 = 289;
pub const SPELL_AURA_MOD_MINIMUM_SPEED: i32 = 305;
pub const SPELL_AURA_MOD_MELEE_HASTE_3: i32 = 319;
/// C++ `AuraType::SPELL_AURA_MOD_DURABILITY_LOSS` (`SpellAuraDefines.h:433`).
pub const SPELL_AURA_MOD_DURABILITY_LOSS: i32 = 338;
/// C++ `AuraType::SPELL_AURA_MOD_AUTOATTACK_DAMAGE`
/// (`SpellAuraDefines.h:439`): `Unit::MeleeDamageBonusDone` (`Unit.cpp:7620-7627`)
/// adds each active effect's percentage to the auto-attack damage.
pub const SPELL_AURA_MOD_AUTOATTACK_DAMAGE: i32 = 344;
pub const SPELL_AURA_MOD_SPEED_NO_CONTROL: i32 = 373;
pub const SPELL_AURA_MOD_MANA_REGEN_PCT: i32 = 379;
pub const SPELL_AURA_SCHOOL_HEAL_ABSORB: i32 = 301;
pub const SPELL_AURA_IGNORE_SPELL_COOLDOWN: i32 = 383;
/// C++ `AuraType::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE`
/// (`SpellAuraDefines.h:398`): `Unit::SpellDamagePctDone` multiplies spell damage
/// by the aura multiplier when the victim carries the aura state in its
/// `GetMiscValue`.
pub const SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE: i32 = 303;
/// C++ `AuraType::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS`
/// (`SpellAuraDefines.h:263`): `Unit::SpellDamagePctDone` multiplies spell damage
/// by the aura multiplier matching the victim's creature type bit
/// (`Unit::GetCreatureTypeMask`, `Unit.cpp:8796-8800`).
pub const SPELL_AURA_MOD_DAMAGE_DONE_VERSUS: i32 = 168;
/// C++ `AuraType::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC`
/// (`SpellAuraDefines.h:344`): `Unit::SpellDamagePctDone` (`Unit.cpp:6734-6740`)
/// multiplies spell damage when the victim carries any aura whose mechanic
/// matches this effect's `GetMiscValue` (`Unit::HasAuraWithMechanic`,
/// `Unit.cpp:4714-4729`).
pub const SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC: i32 = 249;
/// C++ `AuraType::SPELL_AURA_MOD_DAMAGE_DONE_FOR_MECHANIC`
/// (`SpellAuraDefines.h:371`): `Unit::SpellDamagePctDone` (`Unit.cpp:6742-6746`)
/// adds the summed percentage of every aura whose `GetMiscValue` equals the
/// cast effect's mechanic, falling back to the spell's own mechanic.
pub const SPELL_AURA_MOD_DAMAGE_DONE_FOR_MECHANIC: i32 = 276;
/// C++ `AuraType::SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH`
/// (`SpellAuraDefines.h:449`): `Unit::SpellHealingPctDone` (`Unit.cpp:7224-7227`)
/// scales healing done by the target's missing health percentage.
pub const SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH: i32 = 354;
/// C++ `AuraType::SPELL_AURA_MOD_HEALING_PCT` (`SpellAuraDefines.h:213`):
/// `Unit::SpellHealingBonusTaken` (`Unit.cpp:7231-7239`) applies the most
/// positive and most negative active amount to healing the unit receives.
pub const SPELL_AURA_MOD_HEALING_PCT: i32 = 118;
/// C++ `AuraType::SPELL_AURA_MOD_HEALING` (`SpellAuraDefines.h:210`): the
/// victim-side flat healing modifier that `Unit::SpellHealingBonusDone`
/// (`Unit.cpp:7123-7124`) adds through
/// `GetTotalAuraModifierByMiscMask`.
pub const SPELL_AURA_MOD_HEALING: i32 = 115;
/// C++ `AuraType::SPELL_AURA_MOD_HEALING_DONE` (`SpellAuraDefines.h:230`):
/// flat healing bonus read by `Unit::SpellBaseHealingBonusDone`
/// (`Unit.cpp:7282-7315`).
pub const SPELL_AURA_MOD_HEALING_DONE: i32 = 135;
/// C++ `AuraType::SPELL_AURA_MOD_HEALING_DONE_PERCENT`
/// (`SpellAuraDefines.h:231`): `HandleModHealingDonePct`
/// (`SpellAuraEffects.cpp:3645-3654`) recomputes
/// `Player::UpdateHealingDonePercentMod` (`StatSystem.cpp:588-599`), the product
/// of `1 + amount/100` published as `ModHealingDonePercent`.
pub const SPELL_AURA_MOD_HEALING_DONE_PERCENT: i32 = 136;
/// C++ `AuraType::SPELL_AURA_MOD_TARGET_RESISTANCE`
/// (`SpellAuraDefines.h:218`): `HandleModTargetResistance`
/// (`SpellAuraEffects.cpp:3507-3530`) adds the amount to
/// `ModTargetPhysicalResistance` when the effect covers
/// `SPELL_SCHOOL_MASK_NORMAL` and to `ModTargetResistance` when it covers the
/// full `SPELL_SCHOOL_MASK_SPELL`.
pub const SPELL_AURA_MOD_TARGET_RESISTANCE: i32 = 123;
/// C++ `AuraType::SPELL_AURA_MOD_AUTOATTACK_CRIT_CHANCE` (`SpellAuraDefines.h:429`):
/// flat auto-attack critical chance, read by `Unit::RollMeleeOutcomeAgainst`.
pub const SPELL_AURA_MOD_AUTOATTACK_CRIT_CHANCE: i32 = 334;
/// C++ `AuraType::SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY`
/// (`SpellAuraDefines.h:553`): removes `MeleeSpellMissChance`'s +19% dual-wield
/// miss penalty.
pub const SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY: i32 = 458;

/// C++ `AuraType::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` (`SpellAuraDefines.h:220`).
pub const SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN: i32 = 125;
/// C++ `AuraType::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT` (`SpellAuraDefines.h:221`).
pub const SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT: i32 = 126;
/// C++ `AuraType::SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (`SpellAuraDefines.h:364`).
pub const SPELL_AURA_MOD_IGNORE_TARGET_RESIST: i32 = 269;
/// C++ `AuraType::SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER` (`SpellAuraDefines.h:438`).
pub const SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER: i32 = 343;
/// C++ `AuraType::SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT`
/// (`SpellAuraDefines.h:269`): `MiscValue` is the school mask and `MiscValueB`
/// the stat.
pub const SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT: i32 = 174;
/// C++ `AuraType::SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT`
/// (`SpellAuraDefines.h:270`): the effect `MiscValue` is the stat index.
pub const SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT: i32 = 175;
/// C++ `AuraType::SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT`
/// (`SpellAuraDefines.h:461`): `HandleOverrideSpellPowerByAttackPower`
/// (`SpellAuraEffects.cpp:3770-3781`) accumulates
/// `ActivePlayerData::OverrideSpellPowerByAPPercent`, which makes both
/// `SpellBaseDamageBonusDone` and `SpellBaseHealingBonusDone` return a
/// percentage of melee attack power instead of the gear and aura bonuses.
pub const SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT: i32 = 366;
/// C++ `AuraType::SPELL_AURA_MOD_VERSATILITY` (`SpellAuraDefines.h:566`):
/// `HandleModVersatilityByPct` (`SpellAuraEffects.cpp:3797-3808`) sums the
/// amounts into `ActivePlayerData::VersatilityBonus`.
pub const SPELL_AURA_MOD_VERSATILITY: i32 = 471;
pub const SPELL_AURA_MOD_BATTLE_PET_XP_PCT: i32 = 420;
/// C++ `AuraType::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT`
/// (`SpellAuraDefines.h:499`): `AuraEffect::HandleOverrideAttackPowerBySpellPower`
/// (`SpellAuraEffects.cpp:3785-3796`) accumulates each amount into
/// `ActivePlayerData::OverrideAPBySpellPowerPercent` and re-runs
/// `Player::UpdateAttackPowerAndDamage` for both the melee and ranged mods.
pub const SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT: i32 = 404;
pub const SPELL_AURA_MOD_MINIMUM_SPEED_RATE: i32 = 437;
pub const SPELL_AURA_MOD_ROOT_2: i32 = 455;
pub const SPELL_AURA_MOD_RESTED_XP_CONSUMPTION: i32 = 499;
