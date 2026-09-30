//! Represented spell rules shared by both owners and compatibility facades.
use super::*;

pub fn has_nonzero_power_cost(
    spell: &SpellInfoFacts,
) -> bool {
    // `SpellInfo::PowerCosts` retains zero-valued SpellPower rows. Their mere
    // presence does not make `Spell::m_powerCost` nonzero; in particular live
    // 15691 has a type-3 row whose flat, per-level, periodic, percentage,
    // max-percentage, periodic-percentage and optional values are all zero.
    // Any nonzero cost input still fails closed until Creature power
    // calculation/check/deduction is represented.
    spell.power_costs.iter().any(|cost| {
        cost.mana_cost != 0
            || cost.mana_cost_per_level != 0
            || cost.mana_per_second != 0
            || cost.power_cost_pct != 0.0
            || cost.power_cost_max_pct != 0.0
            || cost.power_pct_per_second != 0.0
            || cost.required_aura_spell_id != 0
            || cost.optional_cost != 0
    })
}

pub fn single_unit_topology(
    spell: &SpellInfoFacts,
    target_guid: ObjectGuid,
    recipient_guid: ObjectGuid,
    requires_projectile_payload: bool,
) -> Result<(), SpellTopologyError> {
    // Cast-time completion belongs to M3.1. M2.6 must fail closed rather than
    // emitting START and an immediate, premature GO for a non-instant spell.
    if spell.cast_time_ms != 0 {
        return Err(SpellTopologyError::NonInstant);
    }
    if requires_projectile_payload {
        return Err(SpellTopologyError::ProjectileOrAmmo);
    }
    if spell.requires_spell_focus != 0
        || has_nonzero_power_cost(spell)
    {
        // Focus discovery and Creature power-cost calculation/deduction are
        // not part of M2.6. Emitting GO when C++ CheckCast/CheckPower would
        // fail would be a false successful cast, so keep this slice closed.
        return Err(SpellTopologyError::EffectOrTarget);
    }
    if target_guid != recipient_guid {
        return Err(SpellTopologyError::EffectOrTarget);
    }

    // M2.6 owns AI selection, cooldowns and cast wire. Damage calculation is
    // deliberately left to M3.2: raw EffectBasePoints is not CalcValue and
    // omits dice, scaling, bonuses, mitigation, hit, absorb and resist. Until
    // that pipeline exists, only a topology whose START/GO target lists can be
    // proven from hydrated metadata is emitted, with no fabricated health
    // mutation. TargetA=6 is C++ TARGET_UNIT_TARGET_ENEMY; TargetB must be
    // empty, and chained/radius/triggered effects would add unsupported target
    // or follow-up topology.
    let mut represented_effects = 0usize;
    for effect in spell.effects.iter().filter(|effect| effect.effect != 0) {
        if is_noop(effect.effect) {
            continue;
        }
        if effect.effect != 2
            || effect.implicit_target_1 != 6
            || effect.implicit_target_2 != 0
            || effect.chain_targets != 0
            || effect.effect_radius_index_1 != 0
            || effect.effect_trigger_spell != 0
        {
            return Err(SpellTopologyError::EffectOrTarget);
        }
        represented_effects += 1;
    }
    if represented_effects == 0 {
        return Err(SpellTopologyError::EffectOrTarget);
    }
    Ok(())
}

pub(super) fn is_noop(effect: u32) -> bool {
    matches!(
        effect,
        0 /* SPELL_EFFECT_NONE */
            | 4 /* SPELL_EFFECT_PORTAL_TELEPORT */
            | 12 /* SPELL_EFFECT_PORTAL */
            | 13 /* SPELL_EFFECT_RITUAL_BASE */
            | 14 /* SPELL_EFFECT_RITUAL_SPECIALIZE */
            | 15 /* SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL */
            | 20 /* SPELL_EFFECT_DODGE */
            | 21 /* SPELL_EFFECT_EVADE */
            | 25 /* SPELL_EFFECT_WEAPON */
            | 26 /* SPELL_EFFECT_DEFENSE */
            | 35 /* SPELL_EFFECT_APPLY_AREA_AURA_PARTY */
            | 37 /* SPELL_EFFECT_SPELL_DEFENSE */
            | 39 /* SPELL_EFFECT_LANGUAGE */
            | 46 /* SPELL_EFFECT_SPAWN */
            | 48 /* SPELL_EFFECT_STEALTH */
            | 49 /* SPELL_EFFECT_DETECT */
            | 51 /* SPELL_EFFECT_FORCE_CRITICAL_HIT */
            | 52 /* SPELL_EFFECT_GUARANTEE_HIT */
            | 65 /* SPELL_EFFECT_APPLY_AREA_AURA_RAID */
            | 78 /* SPELL_EFFECT_ATTACK */
            | 80 /* SPELL_EFFECT_ADD_COMBO_POINTS */
            | 81 /* SPELL_EFFECT_CREATE_HOUSE */
            | 82 /* SPELL_EFFECT_BIND_SIGHT */
            | 91 /* SPELL_EFFECT_THREAT_ALL */
            | 105 /* SPELL_EFFECT_SURVEY */
            | 107 /* SPELL_EFFECT_SHOW_CORPSE_LOOT */
            | 112 /* SPELL_EFFECT_112 */
            | 119 /* SPELL_EFFECT_APPLY_AREA_AURA_PET */
            | 122 /* SPELL_EFFECT_122 */
            | 128 /* SPELL_EFFECT_APPLY_AREA_AURA_FRIEND */
            | 129 /* SPELL_EFFECT_APPLY_AREA_AURA_ENEMY */
            | 135 /* SPELL_EFFECT_CALL_PET */
            | 143 /* SPELL_EFFECT_APPLY_AREA_AURA_OWNER */
            | 163 /* SPELL_EFFECT_OBLITERATE_ITEM */
            | 168 /* SPELL_EFFECT_ALLOW_CONTROL_PET */
            | 175 /* SPELL_EFFECT_175 */
            | 177 /* SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA */
            | 178 /* SPELL_EFFECT_178 */
            | 180 /* SPELL_EFFECT_UPDATE_AREATRIGGER */
            | 182 /* SPELL_EFFECT_DESPAWN_AREATRIGGER */
            | 183 /* SPELL_EFFECT_183 */
            | 184 /* SPELL_EFFECT_REPUTATION_2 */
            | 185 /* SPELL_EFFECT_185 */
            | 186 /* SPELL_EFFECT_186 */
            | 187 /* SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES */
            | 188 /* SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN */
            | 189 /* SPELL_EFFECT_LOOT */
            | 190 /* SPELL_EFFECT_CHANGE_PARTY_MEMBERS */
            | 191 /* SPELL_EFFECT_TELEPORT_TO_DIGSITE */
            | 193 /* SPELL_EFFECT_START_PET_BATTLE */
            | 194 /* SPELL_EFFECT_194 */
            | 199 /* SPELL_EFFECT_DESPAWN_SUMMON */
            | 202 /* SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS */
            | 206 /* SPELL_EFFECT_ALTER_ITEM */
            | 207 /* SPELL_EFFECT_LAUNCH_QUEST_TASK */
            | 208 /* SPELL_EFFECT_SET_REPUTATION */
            | 209 /* SPELL_EFFECT_209 */
            | 210 /* SPELL_EFFECT_LEARN_GARRISON_BUILDING */
            | 211 /* SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION */
            | 214 /* SPELL_EFFECT_CREATE_GARRISON */
            | 215 /* SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS */
            | 216 /* SPELL_EFFECT_CREATE_SHIPMENT */
            | 217 /* SPELL_EFFECT_UPGRADE_GARRISON */
            | 218 /* SPELL_EFFECT_218 */
            | 220 /* SPELL_EFFECT_ADD_GARRISON_FOLLOWER */
            | 221 /* SPELL_EFFECT_ADD_GARRISON_MISSION */
            | 223 /* SPELL_EFFECT_CHANGE_ITEM_BONUSES */
            | 224 /* SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING */
            | 226 /* SPELL_EFFECT_TRIGGER_ACTION_SET */
            | 227 /* SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON */
            | 228 /* SPELL_EFFECT_228 */
            | 229 /* SPELL_EFFECT_SET_FOLLOWER_QUALITY */
            | 230 /* SPELL_EFFECT_230 */
            | 231 /* SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE */
            | 232 /* SPELL_EFFECT_REMOVE_PHASE */
            | 233 /* SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES */
            | 234 /* SPELL_EFFECT_234 */
            | 235 /* SPELL_EFFECT_235 */
            | 238 /* SPELL_EFFECT_INCREASE_SKILL */
            | 239 /* SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION */
            | 240 /* SPELL_EFFECT_GIVE_ARTIFACT_POWER */
            | 241 /* SPELL_EFFECT_241 */
            | 242 /* SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS */
            | 244 /* SPELL_EFFECT_LEARN_FOLLOWER_ABILITY */
            | 246 /* SPELL_EFFECT_FINISH_GARRISON_MISSION */
            | 247 /* SPELL_EFFECT_ADD_GARRISON_MISSION_SET */
            | 248 /* SPELL_EFFECT_FINISH_SHIPMENT */
            | 249 /* SPELL_EFFECT_FORCE_EQUIP_ITEM */
            | 250 /* SPELL_EFFECT_TAKE_SCREENSHOT */
            | 251 /* SPELL_EFFECT_SET_GARRISON_CACHE_SIZE */
            | 256 /* SPELL_EFFECT_256 */
            | 257 /* SPELL_EFFECT_257 */
            | 258 /* SPELL_EFFECT_MODIFY_KEYSTONE */
            | 259 /* SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM */
            | 260 /* SPELL_EFFECT_SUMMON_STABLED_PET */
            | 261 /* SPELL_EFFECT_SCRAP_ITEM */
            | 262 /* SPELL_EFFECT_262 */
            | 263 /* SPELL_EFFECT_REPAIR_ITEM */
            | 264 /* SPELL_EFFECT_REMOVE_GEM */
            | 265 /* SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER */
            | 266 /* SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY */
            | 268 /* SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT */
            | 269 /* SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP */
            | 270 /* SPELL_EFFECT_270 */
            | 271 /* SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM */
            | 272 /* SPELL_EFFECT_SET_COVENANT */
            | 273 /* SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY */
            | 274 /* SPELL_EFFECT_274 */
            | 275 /* SPELL_EFFECT_275 */
            | 277 /* SPELL_EFFECT_SET_CHROMIE_TIME */
            | 278 /* SPELL_EFFECT_278 */
            | 279 /* SPELL_EFFECT_LEARN_GARR_TALENT */
            | 280 /* SPELL_EFFECT_280 */
            | 281 /* SPELL_EFFECT_LEARN_SOULBIND_CONDUIT */
            | 282 /* SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY */
            | 283 /* SPELL_EFFECT_COMPLETE_CAMPAIGN */
            | 285 /* SPELL_EFFECT_MODIFY_KEYSTONE_2 */
            | 287 /* SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL */
            | 288 /* SPELL_EFFECT_CRAFT_ITEM */
            | 294 /* SPELL_EFFECT_CRAFT_LOOT */
            | 295 /* SPELL_EFFECT_SALVAGE_ITEM */
            | 296 /* SPELL_EFFECT_CRAFT_SALVAGE_ITEM */
            | 297 /* SPELL_EFFECT_RECRAFT_ITEM */
            | 298 /* SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS */
            | 299 /* SPELL_EFFECT_299 */
            | 300 /* SPELL_EFFECT_300 */
            | 301 /* SPELL_EFFECT_CRAFT_ENCHANT */
            | 302 /* SPELL_EFFECT_GATHERING */
            | 305 /* SPELL_EFFECT_305 */
            | 306 /* SPELL_EFFECT_UPDATE_INTERACTIONS */
            | 307 /* SPELL_EFFECT_307 */
            | 308 /* SPELL_EFFECT_CANCEL_PRELOAD_WORLD */
            | 309 /* SPELL_EFFECT_PRELOAD_WORLD */
            | 310 /* SPELL_EFFECT_310 */
            | 311 /* SPELL_EFFECT_ENSURE_WORLD_LOADED */
            | 312 /* SPELL_EFFECT_312 */
            | 313 /* SPELL_EFFECT_CHANGE_ITEM_BONUSES_2 */
            | 314 /* SPELL_EFFECT_ADD_SOCKET_BONUS */
            | 315 /* SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP */
    )
}
