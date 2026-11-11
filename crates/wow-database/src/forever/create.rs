//! Serialized GUID ownership and durable target character creation.
use super::{ForeverSessionRepository, PreparedStatement, field};
use crate::transaction::SqlTransaction;
use std::fmt::Write;
use std::sync::LazyLock;
use wow_persistence::forever::creation::save::{
    CreateError, CreatedCharacter, CreationIdentity, NewAction, NewCharacter, NewCustomization,
    NewSpell, NewVisualItem,
};

mod auras;
mod auxiliary_character;
mod bank_tabs;
mod commits;
mod cuf_profiles;
mod currency;
mod equipment_sets;
mod garrison;
mod items;
mod journal;
mod lease;
mod mail;
mod player_data;
mod progress;
mod quests;
pub(super) mod recovery;
mod session;
mod specializations;
mod spell_history;
mod statistics;
mod stored_locations;
mod traits;
mod transmog_outfits;
mod writer;

#[cfg(test)]
#[path = "../../unit_tests/forever/create.rs"]
mod tests;

const CHARACTER_INSERT: &str = "INSERT INTO characters (guid, account, name, race, class, gender, level, xp, money, inventorySlots, inventoryBagFlags, bagSlotFlags1, bagSlotFlags2, bagSlotFlags3, bagSlotFlags4, bagSlotFlags5, bankSlots, bankTabs, bankBagFlags, restState, playerFlags, playerFlagsEx, map, instance_id, dungeonDifficulty, raidDifficulty, legacyRaidDifficulty, position_x, position_y, position_z, orientation, trans_x, trans_y, trans_z, trans_o, transguid, taximask, createTime, createMode, cinematic, totaltime, leveltime, rest_bonus, logout_time, is_logout_resting, resettalents_cost, resettalents_time, primarySpecialization, extra_flags, summonedPetNumber, at_login, death_expire_time, taxi_path, totalKills, todayKills, yesterdayKills, chosenTitle, watchedFaction, drunk, health, power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, latency, activeTalentGroup, lootSpecId, exploredZones, knownTitles, actionBars, lastLoginBuild, personalTabardEmblemStyle, personalTabardEmblemColor, personalTabardBorderStyle, personalTabardBorderColor, personalTabardBackgroundColor, transmogOutfitEquippedId, transmogOutfitLocked) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)";
const CUSTOMIZATION_INSERT: &str = "INSERT INTO character_customizations (guid, chrCustomizationOptionID, chrCustomizationChoiceID) VALUES (?, ?, ?)";
const CUSTOMIZATION_DELETE: &str = "DELETE FROM character_customizations WHERE guid = ?";
const SPELL_INSERT: &str =
    "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)";
const SPELL_FAVORITE_DELETE: &str =
    "DELETE FROM character_spell_favorite WHERE guid = ? AND spell = ?";
const SPELL_FAVORITE_INSERT: &str =
    "INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)";
const VISUAL_CACHE_DELETE: &str =
    "DELETE FROM character_select_screen_equipment_cache WHERE guid = ?";

const VISUAL_SLOTS: [&str; 19] = [
    "head", "neck", "shoulder", "body", "chest", "waist", "legs", "feet", "wrists", "hands",
    "finger1", "finger2", "trinket1", "trinket2", "back", "mainHand", "offHand", "ranged",
    "tabard",
];
const VISUAL_FIELDS: [&str; 8] = [
    "EquippedItemID",
    "VisibleItemID",
    "Subclass",
    "InvType",
    "DisplayID",
    "DisplayEnchantID",
    "SecondaryItemModifiedAppearanceID",
    "SheatheCategory",
];

#[derive(Default)]
pub(super) struct CreationIds {
    player: Option<u64>,
    item: Option<u64>,
    recovery_required: bool,
}

struct CreationReservation {
    ids: tokio::sync::OwnedMutexGuard<CreationIds>,
    identity: CreationIdentity,
    item_start: u64,
    name: String,
    realm_count: u32,
}

struct CreationLease {
    // Release/close database owners before releasing the in-memory gate when
    // an unused operation or unpolled save is dropped.
    writer: writer::CreationWriter,
    auth_transaction: sqlx::Transaction<'static, sqlx::MySql>,
    reservation: CreationReservation,
}

fn next(counter: &mut Option<u64>, maximum: Option<u64>) -> Result<u64, CreateError> {
    let value = match *counter {
        Some(value) => value,
        None => maximum
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(CreateError::InvalidRow)?,
    };
    // 02245dcd ObjectGuidSequenceGenerator.cpp::CheckGeneratedGuidValue
    // rejects at GetMaxCounter(high) - 1, including cached allocations.
    if value == 0 || value >= 0xFFFFFFFFFF - 1 {
        return Err(CreateError::InvalidRow);
    }
    *counter = Some(value + 1);
    Ok(value)
}

impl CreationReservation {
    fn validate(&self, character: &NewCharacter) -> Result<u32, CreateError> {
        if character.guid != self.identity.guid
            || character.account != self.identity.account
            || character.realm != self.identity.realm
            || character.name != self.name
        {
            return Err(CreateError::InvalidRow);
        }
        // Validate the LoginDB timestamp before CharacterDB can commit. A
        // rejected conversion after that point is already a partial save.
        let last_played_time =
            u32::try_from(character.state.save_time).map_err(|_| CreateError::InvalidRow)?;
        if character
            .state
            .taxi_path
            .as_ref()
            .is_some_and(|path| path.nodes.len() < 2)
        {
            return Err(CreateError::InvalidRow);
        }
        items::validate(&character.items)?;
        if self.ids.recovery_required {
            return Err(CreateError::RecoveryRequired);
        }
        let item_end = self.ids.item.ok_or(CreateError::InvalidRow)?;
        if character
            .items
            .iter()
            .any(|item| item.guid < self.item_start || item.guid >= item_end)
        {
            return Err(CreateError::InvalidRow);
        }
        Ok(last_played_time)
    }
}

impl CreationLease {
    async fn persist(
        self: Box<Self>,
        character: NewCharacter,
    ) -> Result<CreatedCharacter, CreateError> {
        let Self {
            mut reservation,
            mut writer,
            auth_transaction,
        } = *self;
        let last_played_time = reservation.validate(&character)?;
        let guid = reservation.identity.guid;
        let mut transaction = SqlTransaction::new();
        append_character_save(&mut transaction, guid, &character);
        // CharacterHandler appends the surname after Player::SaveToDB.
        if !character.surname.is_empty() {
            let mut statement =
                PreparedStatement::new("UPDATE characters SET surname=? WHERE guid=?");
            statement.set_string(0, character.surname.clone());
            statement.set_u64(1, guid);
            transaction.append_expect_rows_affected(statement, 1);
        }
        let journal = journal::Entry {
            guid,
            account: character.account,
            realm: character.realm,
            save_time: last_played_time,
        };
        // The marker and all represented character rows commit together.
        transaction.append(journal.insert());
        let next_count = reservation
            .realm_count
            .checked_add(1)
            .and_then(|count| u8::try_from(count).ok())
            .ok_or(CreateError::InvalidRow)?;
        let mut login = SqlTransaction::new();
        login.append(writer::delete_last_played(journal));
        login.append(writer::insert_last_played(journal, character.name));
        login.append(writer::realm_count(journal, next_count));
        // Construct both batches before entering the cancellation fence; the
        // coordinator polls CharacterDB first and LoginDB only after success.
        let mut steps = commits::DatabaseCommits {
            writer: &mut writer,
            character: Some(transaction),
            login: Some((login, auth_transaction)),
            cleanup: Some(journal.cleanup()),
        };
        commits::commit(&mut reservation.ids, &mut steps).await?;
        Ok(CreatedCharacter { guid })
    }
}

fn append_character_save(transaction: &mut SqlTransaction, guid: u64, character: &NewCharacter) {
    append_creation_through_skills(transaction, guid, character);
    stored_locations::append(transaction, guid, &character.stored_aura_locations);
    progress::append(
        transaction,
        guid,
        &character.achievements,
        &character.reputation,
        &character.quest_criteria,
    );
    equipment_sets::append(transaction, guid, &character.equipment_sets);
    transmog_outfits::append(transaction, guid, &character.transmog_outfits);
    append_visual_cache(transaction, guid, &character.visual_items);
    session::append(transaction, character.account, &character.session_save);
    currency::append(transaction, guid, &character.currencies);
    cuf_profiles::append(transaction, guid, &character.cuf_profiles);
    player_data::append(transaction, guid, &character.player_data);
    bank_tabs::append(transaction, guid, &character.bank_tabs);
    garrison::append(transaction, guid, character.garrison.as_ref());
    statistics::append(transaction, guid, character.statistics.as_ref());
}

fn append_creation_through_skills(
    transaction: &mut SqlTransaction,
    guid: u64,
    character: &NewCharacter,
) {
    append_creation_through_actions(transaction, guid, character);
    auras::append(transaction, guid, &character.auras);
    for skill in &character.skills {
        let mut statement = PreparedStatement::new(
            "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?,?,?,?,?)",
        );
        statement.set_u64(0, guid);
        statement.set_u16(1, skill.skill);
        statement.set_u16(2, skill.value);
        statement.set_u16(3, skill.maximum);
        statement.set_i8(4, skill.profession_slot);
        transaction.append(statement);
    }
}

fn append_creation_through_actions(
    transaction: &mut SqlTransaction,
    guid: u64,
    character: &NewCharacter,
) {
    append_creation_prefix(transaction, guid, character);
    for action in &character.actions {
        transaction.append(action_statement(guid, action));
    }
}

fn append_creation_prefix(transaction: &mut SqlTransaction, guid: u64, character: &NewCharacter) {
    append_character_and_inventory(transaction, guid, character);
    quests::append(transaction, guid, &character.quests);
    specializations::append(transaction, guid, &character.specializations);
    traits::append(transaction, guid, &character.traits);
    append_spells(transaction, guid, &character.spells);
    spell_history::append(transaction, guid, &character.spell_history);
}

fn append_character_and_inventory(
    transaction: &mut SqlTransaction,
    guid: u64,
    character: &NewCharacter,
) {
    transaction.append(auxiliary_character::fishing_delete_statement(guid));
    transaction.append(character_statement(guid, character));
    auxiliary_character::append_fishing(transaction, guid, character.fishing_steps);
    mail::append(transaction, &character.mail);
    append_customizations(transaction, guid, &character.customizations);
    auxiliary_character::append_battleground(transaction, guid, &character.battleground);
    items::append(transaction, guid, &character.items);
}

fn action_statement(guid: u64, action: &NewAction) -> PreparedStatement {
    let mut statement = PreparedStatement::new(
        "INSERT INTO character_action (guid, spec, traitConfigId, button, action, type) VALUES (?,?,?,?,?,?)",
    );
    statement.set_u64(0, guid);
    statement.set_u8(1, action.spec);
    statement.set_i32(2, action.trait_config);
    statement.set_u8(3, action.button);
    statement.set_u64(4, action.action);
    statement.set_u8(5, action.kind);
    statement
}

fn append_customizations(
    transaction: &mut SqlTransaction,
    guid: u64,
    customizations: &[NewCustomization],
) {
    transaction.append(customizations_delete_statement(guid));
    for customization in customizations {
        transaction.append(customization_insert_statement(guid, customization));
    }
}

fn customizations_delete_statement(guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(CUSTOMIZATION_DELETE);
    statement.set_u64(0, guid);
    statement
}

fn customization_insert_statement(
    guid: u64,
    customization: &NewCustomization,
) -> PreparedStatement {
    let mut statement = PreparedStatement::new(CUSTOMIZATION_INSERT);
    statement.set_u64(0, guid);
    statement.set_u32(1, customization.option);
    statement.set_u32(2, customization.choice);
    statement
}

fn append_spells(transaction: &mut SqlTransaction, guid: u64, spells: &[NewSpell]) {
    for spell in spells {
        if !spell.dependent {
            transaction.append(spell_insert_statement(guid, spell));
        }
        transaction.append(spell_favorite_delete_statement(guid, spell.spell));
        if spell.favorite {
            transaction.append(spell_favorite_insert_statement(guid, spell.spell));
        }
    }
}

fn spell_insert_statement(guid: u64, spell: &NewSpell) -> PreparedStatement {
    let mut statement = PreparedStatement::new(SPELL_INSERT);
    statement.set_u64(0, guid);
    statement.set_u32(1, spell.spell);
    statement.set_bool(2, spell.active);
    statement.set_bool(3, spell.disabled);
    statement
}

fn spell_favorite_delete_statement(guid: u64, spell: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::new(SPELL_FAVORITE_DELETE);
    statement.set_u64(0, guid);
    statement.set_u32(1, spell);
    statement
}

fn spell_favorite_insert_statement(guid: u64, spell: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::new(SPELL_FAVORITE_INSERT);
    statement.set_u64(0, guid);
    statement.set_u32(1, spell);
    statement
}

fn character_statement(guid: u64, character: &NewCharacter) -> PreparedStatement {
    let mut statement = PreparedStatement::new(CHARACTER_INSERT);
    let state = &character.state;
    let mut index = 0;
    macro_rules! set {
        ($method:ident, $value:expr) => {{
            statement.$method(index, $value);
            index += 1;
        }};
    }
    set!(set_u64, guid);
    set!(set_u32, character.account);
    set!(set_string, character.name.clone());
    set!(set_u8, character.race);
    set!(set_u8, character.class);
    set!(set_u8, character.gender);
    set!(set_u8, character.level);
    set!(set_u32, character.experience);
    set!(set_u64, character.money);
    set!(set_u8, state.inventory_slots);
    set!(set_u32, state.inventory_bag_flags);
    for flags in state.bag_slot_flags {
        set!(set_u32, flags);
    }
    set!(set_u8, state.bank_slots);
    set!(set_u8, state.bank_tabs);
    set!(set_u32, state.bank_bag_flags);
    set!(set_u8, state.rest_state);
    set!(set_u32, state.player_flags);
    set!(set_u32, state.player_flags_ex);
    set!(set_u16, character.map);
    set!(set_u32, state.instance_id);
    set!(set_u8, state.dungeon_difficulty);
    set!(set_u8, state.raid_difficulty);
    set!(set_u8, state.legacy_raid_difficulty);
    for coordinate in character.position {
        set!(set_f32, finite(coordinate));
    }
    for coordinate in state.transport_position {
        set!(set_f32, finite(coordinate));
    }
    set!(set_u64, state.transport_guid);
    set!(
        set_string,
        decimal_words(state.taxi_mask.iter().map(|&value| u32::from(value)))
    );
    set!(set_i64, character.create_time);
    set!(set_i8, character.create_mode);
    set!(set_u8, character.cinematic);
    set!(set_u32, state.total_played_time);
    set!(set_u32, state.level_played_time);
    set!(set_f32, finite(state.rest_bonus));
    set!(set_u64, state.save_time);
    set!(set_u8, u8::from(state.logout_resting));
    set!(set_u32, state.talent_reset_cost);
    set!(set_i64, state.talent_reset_time);
    set!(set_u32, character.primary_specialization);
    set!(set_u16, state.extra_flags);
    set!(set_u32, 0);
    set!(set_u16, character.at_login);
    set!(set_i64, state.death_expire_time);
    set!(
        set_string,
        state.taxi_path.as_ref().map_or_else(String::new, |path| {
            decimal_words(
                std::iter::once(path.flight_master_faction).chain(path.nodes.iter().copied()),
            )
        })
    );
    set!(set_u32, state.lifetime_honorable_kills);
    set!(set_u16, state.today_honorable_kills);
    set!(set_u16, state.yesterday_honorable_kills);
    set!(set_u32, state.chosen_title);
    set!(set_u32, state.watched_faction);
    set!(set_u8, state.drunk);
    set!(set_u32, character.health);
    for power in character.powers {
        set!(set_u32, power);
    }
    set!(set_u32, character.latency);
    set!(set_u8, character.active_talent_group);
    set!(set_u32, state.loot_specialization);
    set!(set_string, split_words(&state.explored_zones));
    set!(set_string, split_words(&state.known_titles));
    set!(set_u8, state.action_bars);
    set!(
        set_u32,
        character
            .client_version
            .map_or(0, |[major, minor, bugfix]| {
                // ClientBuild::Info components and its arithmetic are uint32.
                major
                    .wrapping_mul(10000)
                    .wrapping_add(minor.wrapping_mul(100))
                    .wrapping_add(bugfix)
            })
    );
    for field in state.personal_tabard {
        set!(set_i32, field);
    }
    set!(set_i32, state.transmog_outfit);
    set!(set_bool, state.transmog_locked);
    debug_assert_eq!(index, 84);
    statement
}

// Player::SaveToDB's finiteAlways, applied independently to every float.
fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

// PlayerTaxi.cpp streams decimal uint32 values with a trailing space.
fn decimal_words(values: impl IntoIterator<Item = u32>) -> String {
    let mut output = String::new();
    for value in values {
        // Writing to String cannot return a formatting error.
        write!(output, "{value} ").expect("String formatting");
    }
    output
}

// Player::SaveToDB writes the low uint32 first for every uint64 block.
fn split_words(values: &[u64]) -> String {
    decimal_words(
        values
            .iter()
            .flat_map(|&value| [value as u32, (value >> 32) as u32]),
    )
}

fn append_visual_cache(transaction: &mut SqlTransaction, guid: u64, items: &[NewVisualItem; 19]) {
    transaction.append(visual_cache_delete_statement(guid));
    transaction.append(visual_statement(guid, items));
}

fn visual_cache_delete_statement(guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::new(VISUAL_CACHE_DELETE);
    statement.set_u64(0, guid);
    statement
}

fn visual_statement(guid: u64, items: &[NewVisualItem; 19]) -> PreparedStatement {
    static SQL: LazyLock<String> = LazyLock::new(|| {
        let mut columns = String::from("guid");
        let mut values = String::from("?");
        for (slot, field) in VISUAL_SLOTS
            .into_iter()
            .flat_map(|slot| VISUAL_FIELDS.into_iter().map(move |field| (slot, field)))
        {
            columns.push_str(&format!(",{slot}{field}"));
            values.push_str(",?");
        }
        format!("INSERT INTO character_select_screen_equipment_cache ({columns}) VALUES ({values})")
    });
    let mut statement = PreparedStatement::new(&SQL);
    statement.set_u64(0, guid);
    for (slot, item) in items.iter().enumerate() {
        let base = 1 + slot * 8;
        statement.set_u32(base, item.item_id);
        statement.set_u32(base + 1, item.visible_item_id);
        statement.set_u8(base + 2, item.subclass);
        statement.set_u8(base + 3, item.inventory_type);
        statement.set_u32(base + 4, item.display_id);
        statement.set_u32(base + 5, item.display_enchant_id);
        statement.set_i32(base + 6, item.secondary_appearance_id);
        statement.set_u8(base + 7, item.sheathe_category);
    }
    statement
}
