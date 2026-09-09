//! Login operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

// Server endpoints (overridable via env)
pub(crate) fn bnet_host() -> String {
    std::env::var("BNET_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}
pub(crate) fn bnet_port() -> u16 {
    std::env::var("BNET_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8081)
}
pub(crate) fn realm_id() -> u32 {
    std::env::var("REALM_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}
pub(crate) fn auth_db_url() -> Result<String> {
    database_url("WOW_BOT_AUTH_DB_URL", "LoginDatabaseInfo")
}
pub(crate) fn validate_detour_live_fixture_before_login(
    bot: &config::BotConfig,
    options: &DetourChaseCaptureOptions,
) -> Result<()> {
    use mysql::prelude::Queryable;

    validate_detour_fixture_identity(&bot.account, bot.character_guid)?;

    let auth_opts = mysql::Opts::from_url(&auth_db_url()?)
        .map_err(|error| anyhow!("Bad auth DB URL: {error}"))?;
    let mut auth = mysql::Conn::new(auth_opts)
        .map_err(|error| anyhow!("Connect to auth DB for detour fixture: {error}"))?;
    let auth_identity: Option<(Option<String>, u8)> = auth
        .exec_first(
            "SELECT ba.email, a.online \
             FROM account a LEFT JOIN battlenet_accounts ba ON ba.id = a.battlenet_account \
             WHERE a.id = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Validate detour fixture auth identity: {error}"))?;
    let (email, game_account_online) = auth_identity
        .ok_or_else(|| anyhow!("No auth.account row for detour bot id {}", bot.account_id))?;
    if game_account_online != 0
        || !email
            .as_deref()
            .is_some_and(|email| email.eq_ignore_ascii_case(&bot.account))
    {
        bail!(
            "detour fixture game account {} is online or not linked to configured identity {}",
            bot.account_id,
            bot.account
        );
    }

    let character_opts = mysql::Opts::from_url(&characters_db_url()?)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB for detour fixture: {error}"))?;
    let character: Option<(u32, u32, f64, f64, f64, f64, u8)> = characters
        .exec_first(
            "SELECT account, map, position_x, position_y, position_z, orientation, online \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Validate detour fixture character: {error}"))?;
    let (account_id, map_id, x, y, z, orientation, online) = character
        .ok_or_else(|| anyhow!("No characters row for detour guid {}", bot.character_guid))?;
    if account_id != bot.account_id
        || map_id != u32::from(options.map_id)
        || online != 0
        || !detour_fixture_float_matches(x, options.player_start_x)
        || !detour_fixture_float_matches(y, options.player_start_y)
        || !detour_fixture_float_matches(z, options.player_start_z)
        || !detour_fixture_float_matches(orientation, options.player_start_orientation)
    {
        bail!(
            "detour fixture character {} does not match pinned offline account/map/start position",
            bot.character_guid
        );
    }

    let world_opts = mysql::Opts::from_url(&world_db_url()?)
        .map_err(|error| anyhow!("Bad world DB URL: {error}"))?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB for detour fixture: {error}"))?;
    let creature: Option<(u32, u32, f64, f64, f64)> = world
        .exec_first(
            "SELECT id, map, position_x, position_y, position_z \
             FROM creature WHERE guid = ?",
            (options.target_spawn_guid,),
        )
        .map_err(|error| anyhow!("Validate detour fixture creature spawn: {error}"))?;
    let (entry, map_id, x, y, z) = creature.ok_or_else(|| {
        anyhow!(
            "No world.creature row for detour spawn {}",
            options.target_spawn_guid
        )
    })?;
    if entry != options.target_entry
        || map_id != u32::from(options.map_id)
        || !detour_fixture_float_matches(x, options.target_x)
        || !detour_fixture_float_matches(y, options.target_y)
        || !detour_fixture_float_matches(z, options.target_z)
    {
        bail!(
            "detour creature spawn {} does not match pinned entry/map/start position",
            options.target_spawn_guid
        );
    }
    let nearby: Vec<u64> = world
        .exec(
            "SELECT guid FROM creature \
             WHERE id = ? AND map = ? \
               AND SQRT(POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) <= ? \
             ORDER BY guid",
            (
                options.target_entry,
                u32::from(options.map_id),
                options.target_x,
                options.target_y,
                options.target_z,
                DETOUR_CHASE_TARGET_MATCH_RADIUS,
            ),
        )
        .map_err(|error| anyhow!("Validate isolated detour creature radius: {error}"))?;
    if nearby != [options.target_spawn_guid] {
        bail!(
            "detour fixture radius is ambiguous: expected only spawn {}, found {:?}",
            options.target_spawn_guid,
            nearby
        );
    }
    Ok(())
}
pub(crate) fn validate_creature_spell_live_fixture_before_login(
    bot: &config::BotConfig,
    options: &CreatureSpellCaptureOptions,
) -> Result<()> {
    use mysql::prelude::Queryable;

    if !bot
        .account
        .eq_ignore_ascii_case(CREATURE_SPELL_FIXTURE_ACCOUNT)
        || bot.account_id != CREATURE_SPELL_FIXTURE_ACCOUNT_ID
        || bot.character_guid != CREATURE_SPELL_FIXTURE_CHARACTER_GUID
    {
        bail!(
            "creature spell fixture requires account {CREATURE_SPELL_FIXTURE_ACCOUNT}/{} and character {}",
            CREATURE_SPELL_FIXTURE_ACCOUNT_ID,
            CREATURE_SPELL_FIXTURE_CHARACTER_GUID
        );
    }
    if options.fixture_manifest_sha256 != CREATURE_SPELL_FIXTURE_MANIFEST_SHA256 {
        bail!("creature spell capture options lost their pinned manifest identity");
    }

    let auth_opts = mysql::Opts::from_url(&auth_db_url()?)
        .map_err(|error| anyhow!("Bad auth DB URL: {error}"))?;
    let mut auth = mysql::Conn::new(auth_opts)
        .map_err(|error| anyhow!("Connect to auth DB for creature spell fixture: {error}"))?;
    let auth_identity: Option<(Option<String>, u8)> = auth
        .exec_first(
            "SELECT ba.email, a.online \
             FROM account a LEFT JOIN battlenet_accounts ba ON ba.id = a.battlenet_account \
             WHERE a.id = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Validate creature spell fixture auth identity: {error}"))?;
    let (email, game_account_online) = auth_identity.ok_or_else(|| {
        anyhow!(
            "No auth.account row for creature spell bot id {}",
            bot.account_id
        )
    })?;
    if game_account_online != 0
        || !email
            .as_deref()
            .is_some_and(|email| email.eq_ignore_ascii_case(CREATURE_SPELL_FIXTURE_ACCOUNT))
    {
        bail!("creature spell fixture game/BNet identity is not pinned and offline");
    }

    let character_opts = mysql::Opts::from_url(&characters_db_url()?)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB for creature spell fixture: {error}"))?;
    let character: Option<(u32, String, u8, u8, u8, u32, u32, u32, f64, f64, f64, f64)> =
        characters
            .exec_first(
                "SELECT account, name, race, class, level, map, zone, instance_id, \
                    position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ? AND online = 0",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Validate creature spell fixture character: {error}"))?;
    let Some((owner, name, race, class, level, map, zone, instance_id, x, y, z, orientation)) =
        character
    else {
        bail!("creature spell fixture character is missing or online");
    };
    let persisted_state: Option<(u32, f64, f64, f64, f64, u64, Option<String>, i64)> = characters
        .exec_first(
            "SELECT health, trans_x, trans_y, trans_z, trans_o, transguid, \
                    taxi_path, death_expire_time \
             FROM characters WHERE guid = ? AND online = 0",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Validate creature spell persisted character state: {error}"))?;
    let (health, trans_x, trans_y, trans_z, trans_o, transport, taxi_path, death_expire_time) =
        persisted_state
            .ok_or_else(|| anyhow!("creature spell fixture character changed during preflight"))?;
    let corpse_count: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM corpse WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check creature spell fixture corpse state: {error}"))?
        .unwrap_or(0);
    let ghost_state: Option<(u64, u64)> = characters
        .exec_first(
            "SELECT \
               (SELECT COUNT(*) FROM character_aura WHERE guid = ? AND spell = ?), \
               (SELECT COUNT(*) FROM character_aura_effect WHERE guid = ? AND spell = ?)",
            (
                bot.character_guid,
                CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID,
                bot.character_guid,
                CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID,
            ),
        )
        .map_err(|error| anyhow!("Check creature spell fixture ghost aura state: {error}"))?;
    let (ghost_aura_count, ghost_effect_count) = ghost_state
        .ok_or_else(|| anyhow!("creature spell fixture ghost aura query returned no row"))?;
    validate_creature_spell_no_persisted_ghost_state(ghost_aura_count, ghost_effect_count)?;
    if owner != bot.account_id
        || name != "Lfgheal"
        || race != 1
        || class != 2
        || level != 80
        || map != u32::from(CREATURE_SPELL_FIXTURE_MAP_ID)
        || zone != 0
        || instance_id != 0
        || !detour_fixture_float_matches(x, CREATURE_SPELL_CHARACTER_START_X)
        || !detour_fixture_float_matches(y, CREATURE_SPELL_FIXTURE_Y)
        || !detour_fixture_float_matches(z, CREATURE_SPELL_FIXTURE_Z)
        || !detour_fixture_float_matches(orientation, CREATURE_SPELL_CHARACTER_ORIENTATION)
        || health != 50_000
        || trans_x != 0.0
        || trans_y != 0.0
        || trans_z != 0.0
        || trans_o != 0.0
        || transport != 0
        || taxi_path.as_deref().is_some_and(|path| !path.is_empty())
        || death_expire_time != 0
        || corpse_count != 0
    {
        bail!("creature spell fixture character does not match the journal-owned pre-login state");
    }

    let world_opts = mysql::Opts::from_url(&world_db_url()?)
        .map_err(|error| anyhow!("Bad world DB URL: {error}"))?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB for creature spell fixture: {error}"))?;
    let shape: Option<(u64, u64, u64, u64, u64)> = world
        .exec_first(
            "SELECT \
               (SELECT COUNT(*) FROM creature_template ct \
                 JOIN creature_template_difficulty ctd \
                   ON ctd.Entry = ct.entry AND ctd.DifficultyID = 0 \
                 WHERE ct.entry = ? AND ct.name = 'Cabal Interrogator' \
                   AND ct.AIName = 'CombatAI' AND ct.ScriptName = '' \
                   AND ct.VerifiedBuild = 52237 \
                   AND ctd.MinLevel = 64 AND ctd.MaxLevel = 65 \
                   AND ctd.HealthScalingExpansion = 0 \
                   AND ctd.HealthModifier = 1 AND ctd.ManaModifier = 1 \
                   AND ctd.ArmorModifier = 1 AND ctd.DamageModifier = 1 \
                   AND ctd.CreatureDifficultyID = 18203 \
                   AND ctd.TypeFlags = 0 AND ctd.TypeFlags2 = 0 \
                   AND ctd.LootID = 22378 AND ctd.PickPocketLootID = 22378 \
                   AND ctd.SkinLootID = 0 AND ctd.GoldMin = 153 AND ctd.GoldMax = 205 \
                   AND ctd.StaticFlags1 = 1048576 \
                   AND ctd.StaticFlags2 = 0 AND ctd.StaticFlags3 = 0 \
                   AND ctd.StaticFlags4 = 0 AND ctd.StaticFlags5 = 0 \
                   AND ctd.StaticFlags6 = 0 AND ctd.StaticFlags7 = 0 \
                   AND ctd.StaticFlags8 = 0), \
               (SELECT COUNT(*) FROM creature_template_difficulty \
                 WHERE Entry = ?), \
               (SELECT COUNT(*) FROM creature WHERE id = ?), \
               (SELECT COUNT(*) FROM creature \
                 WHERE guid = ? AND id = ? AND map = 530 \
                   AND position_x = CAST(-2764.52 AS FLOAT) \
                   AND position_y = CAST(5431.19 AS FLOAT) \
                   AND position_z = CAST(-34.4548 AS FLOAT)), \
               (SELECT COUNT(*) FROM creature_template_spell \
                 WHERE CreatureID = ? AND `Index` = 0 AND Spell = ? \
                   AND VerifiedBuild = 41031)",
            (
                CREATURE_SPELL_FIXTURE_ENTRY,
                CREATURE_SPELL_FIXTURE_ENTRY,
                CREATURE_SPELL_FIXTURE_ENTRY,
                CREATURE_SPELL_FIXTURE_SPAWN_GUID,
                CREATURE_SPELL_FIXTURE_ENTRY,
                CREATURE_SPELL_FIXTURE_ENTRY,
                CREATURE_SPELL_FIXTURE_SPELL_ID,
            ),
        )
        .map_err(|error| anyhow!("Validate creature spell world fixture: {error}"))?;
    if shape != Some((1, 1, 1, 1, 1)) {
        bail!("creature spell world fixture is not the exact temporary 22378/78686/15691 state");
    }
    Ok(())
}
pub(crate) fn login_known_spells_ready(
    login_verified: bool,
    require_known_spells: bool,
    known_spells_seen: bool,
) -> bool {
    login_verified && (!require_known_spells || known_spells_seen)
}
pub(crate) fn parse_login_known_spells_expectation(raw: &str) -> Result<Vec<u32>> {
    let mut spells = raw
        .split(',')
        .map(str::trim)
        .filter(|spell| !spell.is_empty())
        .map(|spell| {
            spell
                .parse::<u32>()
                .with_context(|| format!("invalid spell ID '{spell}'"))
        })
        .collect::<Result<Vec<_>>>()?;
    if spells.is_empty() {
        bail!("WOW_BOT_LOGIN_EXPECT_KNOWN_SPELLS must contain at least one spell ID");
    }
    if spells.contains(&0) {
        bail!("WOW_BOT_LOGIN_EXPECT_KNOWN_SPELLS contains invalid spell ID 0");
    }
    spells.sort_unstable();
    if let Some(duplicate) = spells.windows(2).find(|pair| pair[0] == pair[1]) {
        bail!(
            "WOW_BOT_LOGIN_EXPECT_KNOWN_SPELLS contains duplicate spell ID {}",
            duplicate[0]
        );
    }
    Ok(spells)
}
pub(crate) fn decode_login_known_spells_like_cpp(
    payload: &[u8],
) -> Result<LoginKnownSpellsLikeCpp> {
    if payload.len() < 9 {
        bail!(
            "SMSG_SEND_KNOWN_SPELLS body is {} bytes; need at least 9",
            payload.len()
        );
    }
    let bit_byte = payload[0];
    if bit_byte & 0x7F != 0 {
        bail!(
            "SMSG_SEND_KNOWN_SPELLS InitialLogin byte has non-canonical padding bits: 0x{bit_byte:02X}"
        );
    }
    let initial_login = bit_byte & 0x80 != 0;
    let known_count =
        u32::from_le_bytes(payload[1..5].try_into().expect("four-byte slice")) as usize;
    let favorite_count =
        u32::from_le_bytes(payload[5..9].try_into().expect("four-byte slice")) as usize;
    let spell_count = known_count
        .checked_add(favorite_count)
        .ok_or_else(|| anyhow!("SMSG_SEND_KNOWN_SPELLS spell counts overflow usize"))?;
    let expected_len = spell_count
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(9))
        .ok_or_else(|| anyhow!("SMSG_SEND_KNOWN_SPELLS body length overflows usize"))?;
    if payload.len() != expected_len {
        bail!(
            "SMSG_SEND_KNOWN_SPELLS counts require {expected_len} bytes but body has {}",
            payload.len()
        );
    }

    let mut cursor = 9;
    let mut read_spells = |count: usize, label: &str| -> Result<Vec<u32>> {
        let mut spells = Vec::with_capacity(count);
        for index in 0..count {
            let end = cursor + 4;
            let spell = u32::from_le_bytes(
                payload[cursor..end]
                    .try_into()
                    .expect("validated exact body length"),
            );
            cursor = end;
            if spell == 0 {
                bail!("{label}[{index}] has invalid spell ID 0");
            }
            spells.push(spell);
        }
        spells.sort_unstable();
        if let Some(duplicate) = spells.windows(2).find(|pair| pair[0] == pair[1]) {
            bail!("{label} contains duplicate spell ID {}", duplicate[0]);
        }
        Ok(spells)
    };

    let known_spells = read_spells(known_count, "KnownSpells")?;
    let favorite_spells = read_spells(favorite_count, "FavoriteSpells")?;
    if let Some(spell) = favorite_spells
        .iter()
        .find(|spell| known_spells.binary_search(spell).is_err())
    {
        bail!("FavoriteSpells contains spell ID {spell} absent from KnownSpells");
    }

    Ok(LoginKnownSpellsLikeCpp {
        initial_login,
        known_spells,
        favorite_spells,
    })
}
pub(crate) fn validate_realm_character_count(
    auth_conn: &mut mysql::Conn,
    bot: &config::BotConfig,
    character_count: u64,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let expected = u8::try_from(character_count).map_err(|_| {
        anyhow!("Character count {character_count} exceeds realmcharacters capacity")
    })?;
    let actual: Option<u8> = auth_conn
        .exec_first(
            "SELECT numchars FROM realmcharacters WHERE acctid = ? AND realmid = ?",
            (bot.account_id, realm_id()),
        )
        .map_err(|e| anyhow!("Load realmcharacters for account {}: {e}", bot.account_id))?;
    if actual != Some(expected) {
        bail!(
            "realmcharacters for account {} must remain {}, found {actual:?}; create-only provisioning will not rewrite it",
            bot.account_id,
            expected
        );
    }
    Ok(())
}
pub(crate) fn quest_smoke_needs_prelogin_db_setup(quest_options: &QuestSmokeOptions) -> bool {
    quest_options.reset_before_run
        || quest_options.relocate_before_login
        || quest_options.set_level_before_login.is_some()
        || quest_options.set_race_before_login.is_some()
        || quest_options.set_class_before_login.is_some()
        || quest_options.objective_persist
}
pub(crate) fn record_quest_objective_login_signal(
    op: u16,
    payload: &[u8],
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) {
    if !quest_options.objective_persist || op != 0x27CB {
        return;
    }
    let Some(quest_id) = quest_options.expected_quest_id else {
        return;
    };
    if !payload
        .windows(4)
        .any(|window| window == quest_id.to_le_bytes())
    {
        return;
    }

    result.quest_objective_update_seen = true;
    result.quest_objective_update_has_expected = quest_options
        .objective_seed
        .iter()
        .filter(|row| row.data > 0)
        .all(|row| {
            u16::try_from(row.data).is_ok_and(|data| {
                payload
                    .windows(2)
                    .any(|window| window == data.to_le_bytes())
            })
        });
}
pub(crate) fn record_equipment_set_login_signal(
    opcode: u16,
    payload: &[u8],
    options: &EquipmentSetSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if opcode != SMSG_LOAD_EQUIPMENT_SET {
        return Ok(());
    }
    if result.equipment_set_login_count.is_some() {
        bail!("received duplicate SMSG_LOAD_EQUIPMENT_SET during one login");
    }
    let sets = parse_load_equipment_sets(payload)?;
    result.equipment_set_login_count = Some(u32::try_from(sets.len())?);
    result.equipment_set_load_seen = true;
    if options.phase == EquipmentSetSmokePhase::VerifyRelog {
        let expected_guid = options
            .expected_guid
            .context("equipment-set relog phase missing expected GUID")?;
        let expected = EquipmentSetWire {
            set_type: options.set_type,
            guid: expected_guid,
            set_id: options.set_id,
            ignore_mask: EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP,
            pieces: [[0; 16]; EQUIPMENT_SET_SLOTS_LIKE_CPP],
            appearances: [0; EQUIPMENT_SET_SLOTS_LIKE_CPP],
            enchants: [0; 2],
            secondary_appearances_and_slots: [0; 4],
            assigned_spec_index: -1,
            set_name: options.set_name.clone(),
            set_icon: options.set_icon.clone(),
        };
        result.equipment_set_relogin_verified = sets.as_slice() == std::slice::from_ref(&expected);
        if !result.equipment_set_relogin_verified {
            warn!(
                "Equipment-set relog mismatch for {}: expected {:?}, loaded {:?}",
                result.account, expected, sets
            );
        }
    }
    Ok(())
}
pub(crate) fn validate_rested_xp_instance_post_realm_opcode(opcode: u16) -> Result<()> {
    if opcode == SMSG_LOG_XP_GAIN {
        bail!(
            "SMSG_LOG_XP_GAIN was duplicated/misrouted to instance after it arrived on realm; C++ routes it only on realm"
        );
    }
    Ok(())
}
pub(crate) async fn observe_instance_after_realm_xp(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    result: &mut BotRunResult,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + RESTED_XP_INSTANCE_OBSERVATION_WINDOW;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            remaining,
            Duration::from_secs(5),
            "rested-XP post-realm instance observation",
        )
        .await?
        else {
            return Ok(());
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 instance post-realm rested-XP drain {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        validate_rested_xp_instance_post_realm_opcode(opcode)?;
    }
}
pub(crate) fn prepare_quest_smoke_before_login(
    bot: &config::BotConfig,
    quest_options: &QuestSmokeOptions,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let target = resolve_quest_target_for_bot(bot, quest_options)?;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;

    if quest_options.reset_before_run {
        let quest_id = quest_options
            .expected_quest_id
            .ok_or_else(|| anyhow!("Quest reset requested without expected quest id"))?;
        reset_bot_quest_state(&mut conn, bot.character_guid, quest_id)?;
        info!(
            "Quest smoke reset character {} quest {}",
            bot.character_guid, quest_id
        );
    }

    if quest_options.relocate_before_login {
        let offset_x = 2.0_f64;
        conn.exec_drop(
            "UPDATE characters \
             SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ?",
            (
                u32::from(target.map_id),
                target.x + offset_x,
                target.y,
                target.z,
                target.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|e| {
            anyhow!(
                "Relocate character {} near quest target: {}",
                bot.character_guid,
                e
            )
        })?;
        info!(
            "Quest smoke relocated character {} near creature {} ({}, {}, {})",
            bot.character_guid,
            target.spawn_guid,
            target.x + offset_x,
            target.y,
            target.z
        );
    }

    if let Some(level) = quest_options.set_level_before_login {
        set_bot_character_level(&mut conn, bot.character_guid, level)?;
        info!(
            "Quest smoke set character {} level to {}",
            bot.character_guid, level
        );
    }

    if quest_options.set_race_before_login.is_some()
        || quest_options.set_class_before_login.is_some()
    {
        set_bot_character_race_class(
            &mut conn,
            bot.character_guid,
            quest_options.set_race_before_login,
            quest_options.set_class_before_login,
        )?;
        info!(
            "Quest smoke set character {} race={:?} class={:?}",
            bot.character_guid,
            quest_options.set_race_before_login,
            quest_options.set_class_before_login
        );
    }

    if quest_options.objective_persist {
        let quest_id = quest_options
            .expected_quest_id
            .ok_or_else(|| anyhow!("Quest objective persistence requested without quest id"))?;
        seed_bot_quest_objective_state(
            &mut conn,
            bot.character_guid,
            quest_id,
            quest_options.objective_status,
            &quest_options.objective_seed,
        )?;
        info!(
            "Quest smoke seeded character {} quest {} objectives {:?}",
            bot.character_guid, quest_id, quest_options.objective_seed
        );
    }

    Ok(())
}
/// Resolve the WoW account for a battle.net email, write `K_64` into
/// account.session_key_bnet, and load the same build auth seed the worldserver
/// uses for CMSG_AUTH_SESSION verification.
///
/// Uses synchronous mysql crate (one short transaction per bot); kept off the
/// runtime via spawn_blocking is unnecessary because run_bot is sequential.
pub(crate) fn prepare_world_auth_context(
    email: &str,
    session_key: &[u8],
    realm_id: u32,
) -> Result<WorldAuthDbContext> {
    use mysql::prelude::Queryable;
    let db_url = auth_db_url()?;
    let opts = mysql::Opts::from_url(&db_url).map_err(|e| anyhow!("Bad DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to auth DB failed: {}", e))?;

    let username: Option<String> = conn
        .exec_first(
            "SELECT a.username FROM account a \
             JOIN battlenet_accounts ba ON a.battlenet_account = ba.id \
             WHERE ba.email = ?",
            (email,),
        )
        .map_err(|e| anyhow!("Lookup username for {}: {}", email, e))?;
    let username = username.ok_or_else(|| anyhow!("No account for email {}", email))?;

    conn.exec_drop(
        "UPDATE account SET session_key_bnet = ? WHERE username = ?",
        (session_key, &username),
    )
    .map_err(|e| anyhow!("UPDATE session_key_bnet for {}: {}", username, e))?;

    let realm_build: Option<(u32,)> = conn
        .exec_first("SELECT gamebuild FROM realmlist WHERE id = ?", (realm_id,))
        .map_err(|e| anyhow!("Lookup realmlist.gamebuild for realm {}: {}", realm_id, e))?;
    let realm_build = realm_build
        .map(|(build,)| build)
        .ok_or_else(|| anyhow!("No realmlist row for realm {}", realm_id))?;

    let seed_row: Option<(Option<String>,)> = conn
        .exec_first(
            "SELECT win64AuthSeed FROM build_info WHERE build = ?",
            (realm_build,),
        )
        .map_err(|e| {
            anyhow!(
                "Lookup build_info.win64AuthSeed for build {}: {}",
                realm_build,
                e
            )
        })?;
    let seed_hex = seed_row
        .and_then(|(seed,)| seed)
        .ok_or_else(|| anyhow!("No win64AuthSeed for build {}", realm_build))?;
    let win64_auth_seed = parse_win64_auth_seed(&seed_hex, realm_build)?;

    Ok(WorldAuthDbContext {
        username,
        realm_build,
        win64_auth_seed,
    })
}
pub(crate) fn parse_win64_auth_seed(seed_hex: &str, build: u32) -> Result<[u8; 16]> {
    if seed_hex.len() != 32 {
        bail!(
            "Invalid win64AuthSeed for build {}: expected 32 hex chars, got {}",
            build,
            seed_hex.len()
        );
    }

    let mut seed = [0u8; 16];
    for (i, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&seed_hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| anyhow!("Invalid win64AuthSeed hex for build {}: {}", build, e))?;
    }
    Ok(seed)
}
/// Compute the 24-byte auth digest used in CMSG_AUTH_SESSION
pub(crate) fn compute_auth_digest(
    local: &[u8; 16],
    server: &[u8; 16],
    session_key: &[u8],
    auth_seed: &[u8; 16],
) -> [u8; 24] {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};

    type HmacSha256 = Hmac<sha2::Sha256>;

    // SHA256(session_key || build_info.win64AuthSeed)
    let mut hasher = Sha256::new();
    hasher.update(session_key);
    hasher.update(auth_seed);
    let digest_key = hasher.finalize();

    // HMAC(digest_key, local || server || AUTH_CHECK_SEED)
    let mut mac = HmacSha256::new_from_slice(&digest_key).unwrap();
    mac.update(local);
    mac.update(server);
    mac.update(&AUTH_CHECK_SEED);
    let result = mac.finalize().into_bytes();

    let mut digest = [0u8; 24];
    digest.copy_from_slice(&result[..24]);
    digest
}
pub(crate) fn derive_realm_session_key(
    session_key: &[u8],
    local: &[u8; 16],
    server: &[u8; 16],
) -> [u8; 40] {
    let key_data: [u8; 64] = session_key
        .try_into()
        .expect("session_key must be 64 bytes");
    srp6_auth::calculate_session_key(&key_data, server, local)
}
pub(crate) fn compute_continued_auth_digest(
    key: i64,
    local: &[u8; 16],
    server: &[u8; 16],
    session_key: &[u8],
) -> [u8; 24] {
    use hmac::{Hmac, Mac};

    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut mac = HmacSha256::new_from_slice(session_key).unwrap();
    mac.update(&key.to_le_bytes());
    mac.update(local);
    mac.update(server);
    mac.update(&CONTINUED_SESSION_SEED);
    let result = mac.finalize().into_bytes();

    let mut digest = [0u8; 24];
    digest.copy_from_slice(&result[..24]);
    digest
}
pub(crate) fn build_cmsg_auth_continued_session(
    connect_to_key: i64,
    local: &[u8; 16],
    digest: &[u8; 24],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(56);
    data.extend_from_slice(&0i64.to_le_bytes());
    data.extend_from_slice(&connect_to_key.to_le_bytes());
    data.extend_from_slice(local);
    data.extend_from_slice(digest);
    data
}
/// Build CMSG_AUTH_SESSION packet data
pub(crate) fn build_cmsg_auth_session(
    realm_id: u32,
    local: &[u8; 16],
    digest: &[u8; 24],
    ticket: &str,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(128);

    // dos_response (u64)
    data.extend_from_slice(&0u64.to_le_bytes());
    // region_id (u32)
    data.extend_from_slice(&0u32.to_le_bytes());
    // battlegroup_id (u32)
    data.extend_from_slice(&0u32.to_le_bytes());
    // realm_id (u32)
    data.extend_from_slice(&realm_id.to_le_bytes());
    // local_challenge (16 bytes)
    data.extend_from_slice(local);
    // digest (24 bytes)
    data.extend_from_slice(digest);
    // UseIPv6 flag (1 byte)
    data.push(0x00);
    // ticket (string with length prefix)
    let ticket_bytes = ticket.as_bytes();
    data.extend_from_slice(&(ticket_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(ticket_bytes);

    data
}
/// Build player login data (packed GUID + farClip)
pub(crate) fn build_player_login(guid: u64, realm_id: u32, far_clip: f32) -> Vec<u8> {
    let (low, high) = create_player_guid_raw(guid, realm_id);
    let (low_mask, low_bytes) = pack_u64(low);
    let (high_mask, high_bytes) = pack_u64(high);

    let mut data = Vec::with_capacity(2 + low_bytes.len() + high_bytes.len() + 4);
    data.push(low_mask);
    data.push(high_mask);
    data.extend_from_slice(&low_bytes);
    data.extend_from_slice(&high_bytes);
    data.extend_from_slice(&far_clip.to_le_bytes());

    data
}
