// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character create/delete/rename/customise, corpse and resurrection.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{CharStatements, SqlTransaction, WorldStatements};

use super::*;

impl WorldSession {
    /// Handle CMSG_CREATE_CHARACTER — create a new character.
    pub async fn handle_create_character(&mut self, pkt: CreateCharacter) {
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
                return;
            }
        };

        // Validate name length
        if pkt.name.len() < 2 || pkt.name.len() > 12 {
            self.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ERROR,
                guid: ObjectGuid::EMPTY,
            });
            return;
        }

        // Validate name characters (alphanumeric only)
        if !pkt.name.chars().all(|c| c.is_ascii_alphabetic()) {
            self.send_packet(&CreateChar {
                code: response_codes::CHAR_CREATE_ERROR,
                guid: ObjectGuid::EMPTY,
            });
            return;
        }

        // Check name uniqueness
        let mut name_stmt = char_db.prepare(CharStatements::SEL_CHECK_NAME);
        name_stmt.set_string(0, &pkt.name);

        if let Ok(result) = char_db.query(&name_stmt).await {
            if !result.is_empty() {
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_NAME_IN_USE,
                    guid: ObjectGuid::EMPTY,
                });
                return;
            }
        }

        // Check account character limit
        let mut count_stmt = char_db.prepare(CharStatements::SEL_SUM_CHARS);
        count_stmt.set_u32(0, self.account_id);

        if let Ok(result) = char_db.query(&count_stmt).await {
            if !result.is_empty() {
                let count: i64 = result.try_read(0).unwrap_or(0);
                if count >= MAX_CHARACTERS_PER_ACCOUNT as i64 {
                    self.send_packet(&CreateChar {
                        code: response_codes::CHAR_CREATE_ACCOUNT_LIMIT,
                        guid: ObjectGuid::EMPTY,
                    });
                    return;
                }
            }
        }

        // Generate new GUID
        let new_guid_counter = match self.guid_generator() {
            Some(generator) => generator.generate(),
            None => {
                warn!("No GUID generator available");
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
                return;
            }
        };

        // Get start position
        let (map_id, x, y, z, o) = start_position(pkt.race);
        let sex = if pkt.sex < 0 { 0u8 } else { pkt.sex as u8 };

        // C++ `Player::Create` calls `InitStatsForLevel`, then
        // `UpdateMaxHealth`/`SetFullHealth` and `SetFullPower(POWER_MANA)`.
        // At this point create mana is the GtBaseMP value; the intellect
        // bonus is applied later by `UpdateAllStats`.
        let empty_gear = RepresentedPlayerGearStatsLikeCpp::default();
        let (health, mana) = self
            .player_stat_system_projection_like_cpp(pkt.race, pkt.class, 1, &empty_gear)
            .map(|projection| {
                (
                    max_health_u32_like_cpp(projection.max_health),
                    projection.base_mana.max(0) as u32,
                )
            })
            .unwrap_or_else(|| default_health_mana(pkt.class));
        let power1 = default_character_power1_like_cpp(pkt.class, mana);

        let create_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Insert character using the full Trinity-style persistence row. Fields that the
        // simplified path previously left to DB defaults are bound explicitly here.
        let mut ins_stmt = char_db.prepare(CharStatements::INS_CHARACTER);
        ins_stmt.set_u64(0, new_guid_counter as u64); // guid (bigint unsigned)
        ins_stmt.set_u32(1, self.account_id); // account
        ins_stmt.set_string(2, &pkt.name); // name
        ins_stmt.set_u8(3, pkt.race); // race
        ins_stmt.set_u8(4, pkt.class); // class
        ins_stmt.set_u8(5, sex); // gender
        ins_stmt.set_u8(6, 1); // level
        ins_stmt.set_u64(7, 0); // xp
        ins_stmt.set_u64(8, 0); // money
        ins_stmt.set_u32(9, u32::from(INVENTORY_DEFAULT_SIZE)); // inventorySlots
        ins_stmt.set_u32(10, 0); // bankSlots
        ins_stmt.set_u8(
            11,
            initial_character_rest_state_like_cpp(
                self.is_a_recruiter_like_cpp(),
                self.recruiter_id_like_cpp(),
            ),
        ); // restState, C++ Player::Create
        ins_stmt.set_u32(12, 0); // playerFlags
        ins_stmt.set_u32(13, 0); // playerFlagsEx
        ins_stmt.set_i32(14, map_id); // map
        ins_stmt.set_u32(15, 0); // instance_id
        bind_create_character_difficulties_like_cpp(&mut ins_stmt);
        ins_stmt.set_f32(19, x); // position_x
        ins_stmt.set_f32(20, y); // position_y
        ins_stmt.set_f32(21, z); // position_z
        ins_stmt.set_f32(22, o); // orientation
        ins_stmt.set_f32(23, 0.0); // trans_x
        ins_stmt.set_f32(24, 0.0); // trans_y
        ins_stmt.set_f32(25, 0.0); // trans_z
        ins_stmt.set_f32(26, 0.0); // trans_o
        ins_stmt.set_u64(27, 0); // transguid
        ins_stmt.set_string(28, ""); // taximask
        ins_stmt.set_i64(29, create_time); // createTime
        ins_stmt.set_u8(30, 0); // createMode
        ins_stmt.set_u8(31, 0); // cinematic
        ins_stmt.set_u32(32, 0); // totaltime
        ins_stmt.set_u32(33, 0); // leveltime
        ins_stmt.set_f32(34, 0.0); // rest_bonus
        ins_stmt.set_u64(35, create_time.max(0) as u64); // logout_time
        ins_stmt.set_u8(36, 0); // is_logout_resting
        ins_stmt.set_u32(37, 0); // resettalents_cost
        ins_stmt.set_u32(38, 0); // resettalents_time
        ins_stmt.set_u8(39, 0); // activeTalentGroup
        ins_stmt.set_u8(40, 0); // bonusTalentGroups
        ins_stmt.set_u32(41, 0); // extra_flags
        ins_stmt.set_u32(42, 0); // summonedPetNumber
        ins_stmt.set_u32(43, 0x20); // at_login (AT_LOGIN_FIRST)
        ins_stmt.set_u32(44, 0); // death_expire_time
        ins_stmt.set_string(45, ""); // taxi_path
        ins_stmt.set_u32(46, 0); // totalKills
        ins_stmt.set_u32(47, 0); // todayKills
        ins_stmt.set_u32(48, 0); // yesterdayKills
        ins_stmt.set_u32(49, 0); // chosenTitle
        ins_stmt.set_i32(50, 0); // watchedFaction
        ins_stmt.set_u8(51, 0); // drunk
        ins_stmt.set_u32(52, health); // health
        ins_stmt.set_u32(53, power1); // power1
        ins_stmt.set_u32(54, 0); // power2
        ins_stmt.set_u32(55, 0); // power3
        ins_stmt.set_u32(56, 0); // power4
        ins_stmt.set_u32(57, 0); // power5
        ins_stmt.set_u32(58, 0); // power6
        ins_stmt.set_u32(59, 0); // power7
        ins_stmt.set_u32(60, 0); // power8
        ins_stmt.set_u32(61, 0); // power9
        ins_stmt.set_u32(62, 0); // power10
        ins_stmt.set_u32(63, 0); // latency
        ins_stmt.set_u32(64, 0); // lootSpecId
        ins_stmt.set_string(65, ""); // exploredZones
        ins_stmt.set_string(66, ""); // equipmentCache
        ins_stmt.set_string(67, ""); // knownTitles
        ins_stmt.set_u8(68, 0); // actionBars
        ins_stmt.set_u32(69, self.build); // lastLoginBuild

        match char_db.execute(&ins_stmt).await {
            Ok(_) => {
                // Insert customizations into character_customizations table
                for c in &pkt.customizations {
                    let mut cust_stmt = char_db.prepare(CharStatements::INS_CHAR_CUSTOMIZATION);
                    cust_stmt.set_u64(0, new_guid_counter as u64);
                    cust_stmt.set_i32(1, c.option_id);
                    cust_stmt.set_i32(2, c.choice_id);
                    if let Err(e) = char_db.execute(&cust_stmt).await {
                        warn!("Failed to insert customization for guid {new_guid_counter}: {e}");
                    }
                }

                let guid = ObjectGuid::create_player(self.realm_id(), new_guid_counter);
                info!(
                    "Character '{}' created (guid={}, {} customizations) for account {}",
                    pkt.name,
                    new_guid_counter,
                    pkt.customizations.len(),
                    self.account_id
                );

                // Insert initial action buttons from playercreateinfo_action
                if let Some(world_db) = self.world_db().map(Arc::clone) {
                    let action_stmt =
                        world_db.prepare(WorldStatements::SEL_PLAYER_CREATEINFO_ACTION);
                    if let Ok(mut action_result) = world_db.query(&action_stmt).await {
                        let mut action_count = 0u32;
                        loop {
                            let a_race: u8 = action_result.read(0);
                            let a_class: u8 = action_result.read(1);
                            if a_race == pkt.race && a_class == pkt.class {
                                let button: u8 = action_result.read(2);
                                let action: i32 = action_result.try_read(3).unwrap_or(0);
                                let btn_type: u8 = action_result.try_read(4).unwrap_or(0);
                                if action > 0 {
                                    let mut ins =
                                        char_db.prepare(CharStatements::INS_CHARACTER_ACTION);
                                    ins.set_u64(0, new_guid_counter as u64);
                                    ins.set_u8(1, button);
                                    ins.set_i32(2, action);
                                    ins.set_u8(3, btn_type);
                                    if let Err(e) = char_db.execute(&ins).await {
                                        warn!("Failed to insert action button {button}: {e}");
                                    } else {
                                        action_count += 1;
                                    }
                                }
                            }
                            if !action_result.next_row() {
                                break;
                            }
                        }
                        if action_count > 0 {
                            info!(
                                "Inserted {action_count} initial action buttons for '{}'",
                                pkt.name
                            );
                        }
                    }
                }

                // Update realmcharacters count in login DB
                self.update_realm_characters().await;

                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_SUCCESS,
                    guid,
                });
            }
            Err(e) => {
                warn!("Failed to create character: {e}");
                self.send_packet(&CreateChar {
                    code: response_codes::CHAR_CREATE_ERROR,
                    guid: ObjectGuid::EMPTY,
                });
            }
        }
    }

    /// Handle CMSG_CHAR_DELETE — delete a character.
    pub async fn handle_char_delete(&mut self, pkt: CharDelete) {
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                return;
            }
        };

        // Verify the character belongs to this account
        if !self.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to delete non-owned character {:?}",
                self.account_id, pkt.guid
            );
            self.send_packet(&DeleteChar {
                code: response_codes::CHAR_DELETE_FAILED,
            });
            return;
        }

        // Double-check in DB
        let mut check_stmt = char_db.prepare(CharStatements::SEL_CHAR_DEL_CHECK);
        check_stmt.set_u32(0, pkt.guid.counter() as u32);
        check_stmt.set_u32(1, self.account_id);

        if let Ok(result) = char_db.query(&check_stmt).await {
            if result.is_empty() {
                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                return;
            }
        }

        // Delete
        let mut del_stmt = char_db.prepare(CharStatements::DEL_CHARACTER);
        del_stmt.set_u32(0, pkt.guid.counter() as u32);

        match char_db.execute(&del_stmt).await {
            Ok(_) => {
                info!(
                    "Character {:?} deleted for account {}",
                    pkt.guid, self.account_id
                );
                self.remove_legit_character(&pkt.guid);

                // Update realmcharacters count in login DB
                self.update_realm_characters().await;

                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_SUCCESS,
                });
            }
            Err(e) => {
                warn!("Failed to delete character: {e}");
                self.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
            }
        }
    }

    pub(super) fn represented_character_rename_name_result_like_cpp(name: &str) -> u8 {
        if name.is_empty() {
            return CHAR_NAME_NO_NAME_LIKE_CPP;
        }
        if name.len() < 2 {
            return CHAR_NAME_TOO_SHORT_LIKE_CPP;
        }
        if name.len() > 12 {
            return CHAR_NAME_TOO_LONG_LIKE_CPP;
        }
        if !name.chars().all(|c| c.is_ascii_alphabetic()) {
            return CHAR_NAME_INVALID_CHARACTER_LIKE_CPP;
        }

        RESPONSE_SUCCESS_LIKE_CPP
    }

    fn send_character_rename_like_cpp(
        &self,
        result: u8,
        guid: ObjectGuid,
        new_name: impl Into<String>,
    ) {
        let name = new_name.into();
        self.send_packet(&CharacterRenameResult {
            result,
            name,
            guid: (result == RESPONSE_SUCCESS_LIKE_CPP).then_some(guid),
        });
    }

    /// Handle CMSG_CHARACTER_RENAME_REQUEST.
    pub async fn handle_character_rename_request(&mut self, pkt: CharacterRenameRequest) {
        if !self.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to rename non-owned character {:?}",
                self.account_id, pkt.guid
            );
            self.kick(
                "WorldSession::HandleCharRenameOpcode rename character from a different account",
            );
            return;
        }

        let name_result = Self::represented_character_rename_name_result_like_cpp(&pkt.new_name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_character_rename_like_cpp(name_result, pkt.guid, pkt.new_name);
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_character_rename_like_cpp(
                    CHAR_CREATE_ERROR_LIKE_CPP,
                    pkt.guid,
                    pkt.new_name,
                );
                return;
            }
        };

        let mut free_name_stmt = char_db.prepare(CharStatements::SEL_FREE_NAME);
        free_name_stmt.set_u64(0, pkt.guid.counter() as u64);
        free_name_stmt.set_string(1, &pkt.new_name);

        let result = match char_db.query(&free_name_stmt).await {
            Ok(result) => result,
            Err(error) => {
                warn!("Character rename free-name query failed: {error}");
                self.send_character_rename_like_cpp(
                    CHAR_CREATE_ERROR_LIKE_CPP,
                    pkt.guid,
                    pkt.new_name,
                );
                return;
            }
        };

        if result.is_empty() {
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
            return;
        }

        let old_name: String = result.read_string(0);
        let mut at_login_flags: u16 = result.try_read(1).unwrap_or(0);
        if (at_login_flags & AT_LOGIN_RENAME_LIKE_CPP) == 0 {
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
            return;
        }

        at_login_flags &= !AT_LOGIN_RENAME_LIKE_CPP;

        let mut tx = SqlTransaction::new();
        let mut update_name = char_db.prepare(CharStatements::UPD_CHAR_NAME_AT_LOGIN);
        update_name.set_string(0, &pkt.new_name);
        update_name.set_u16(1, at_login_flags);
        update_name.set_u64(2, pkt.guid.counter() as u64);
        tx.append(update_name);

        let mut delete_declined = char_db.prepare(CharStatements::DEL_CHAR_DECLINED_NAME);
        delete_declined.set_u64(0, pkt.guid.counter() as u64);
        tx.append(delete_declined);

        if let Err(error) = char_db.commit_transaction(tx).await {
            warn!("Character rename transaction failed: {error}");
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
            return;
        }

        info!(
            "Account {} renamed character {:?} from {} to {}",
            self.account_id, pkt.guid, old_name, pkt.new_name
        );
        self.send_character_rename_like_cpp(RESPONSE_SUCCESS_LIKE_CPP, pkt.guid, pkt.new_name);
    }

    fn send_char_customize_failure_like_cpp(&self, result: u8, guid: ObjectGuid) {
        self.send_packet(&CharCustomizeFailure { result, guid });
    }

    fn send_char_customize_success_like_cpp(&self, request: &CharCustomize) {
        self.send_packet(&CharCustomizeSuccess {
            guid: request.guid,
            sex_id: request.sex_id,
            customizations: request.customizations.clone(),
            name: request.name.clone(),
        });
    }

    /// Handle CMSG_CHAR_CUSTOMIZE.
    pub async fn handle_char_customize(&mut self, request: CharCustomize) {
        if !self.is_legit_character(&request.guid) {
            warn!(
                "Account {} tried to customize non-owned character {:?}",
                self.account_id, request.guid
            );
            self.kick("WorldSession::HandleCharCustomize Trying to customise character of another account");
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => {
                self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
                return;
            }
        };

        let mut info_stmt = char_db.prepare(CharStatements::SEL_CHAR_CUSTOMIZE_INFO);
        info_stmt.set_u64(0, request.guid.counter() as u64);
        let result = match char_db.query(&info_stmt).await {
            Ok(result) => result,
            Err(error) => {
                warn!("Character customize info query failed: {error}");
                self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
                return;
            }
        };

        if result.is_empty() {
            self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
            return;
        }

        let old_name: String = result.read_string(0);
        let _race: u8 = result.try_read(1).unwrap_or(0);
        let _class: u8 = result.try_read(2).unwrap_or(0);
        let _gender: u8 = result.try_read(3).unwrap_or(0);
        let mut at_login_flags: u16 = result.try_read(4).unwrap_or(0);
        if (at_login_flags & AT_LOGIN_CUSTOMIZE_LIKE_CPP) == 0 {
            self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
            return;
        }

        let name_result = Self::represented_character_rename_name_result_like_cpp(&request.name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_char_customize_failure_like_cpp(name_result, request.guid);
            return;
        }

        if request.name != old_name {
            let mut name_stmt = char_db.prepare(CharStatements::SEL_CHECK_NAME);
            name_stmt.set_string(0, &request.name);
            match char_db.query(&name_stmt).await {
                Ok(existing) if !existing.is_empty() => {
                    self.send_char_customize_failure_like_cpp(
                        CHAR_CREATE_NAME_IN_USE_LIKE_CPP,
                        request.guid,
                    );
                    return;
                }
                Err(error) => {
                    warn!("Character customize name query failed: {error}");
                    self.send_char_customize_failure_like_cpp(
                        CHAR_CREATE_ERROR_LIKE_CPP,
                        request.guid,
                    );
                    return;
                }
                _ => {}
            }
        }

        at_login_flags &= !AT_LOGIN_CUSTOMIZE_LIKE_CPP;

        let mut tx = SqlTransaction::new();
        let mut delete_customizations =
            char_db.prepare(CharStatements::DEL_CHARACTER_CUSTOMIZATIONS);
        delete_customizations.set_u64(0, request.guid.counter() as u64);
        tx.append(delete_customizations);

        for customization in &request.customizations {
            let mut insert_customization = char_db.prepare(CharStatements::INS_CHAR_CUSTOMIZATION);
            insert_customization.set_u64(0, request.guid.counter() as u64);
            insert_customization.set_i32(1, customization.option_id);
            insert_customization.set_i32(2, customization.choice_id);
            tx.append(insert_customization);
        }

        let mut update_name = char_db.prepare(CharStatements::UPD_CHAR_NAME_AT_LOGIN);
        update_name.set_string(0, &request.name);
        update_name.set_u16(1, at_login_flags);
        update_name.set_u64(2, request.guid.counter() as u64);
        tx.append(update_name);

        let mut delete_declined = char_db.prepare(CharStatements::DEL_CHAR_DECLINED_NAME);
        delete_declined.set_u64(0, request.guid.counter() as u64);
        tx.append(delete_declined);

        if let Err(error) = char_db.commit_transaction(tx).await {
            warn!("Character customize transaction failed: {error}");
            self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
            return;
        }

        info!(
            "Account {} customized character {:?} from {} to {}",
            self.account_id, request.guid, old_name, request.name
        );
        self.send_char_customize_success_like_cpp(&request);
    }

    /// Handle CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS.
    ///
    /// The client sends this when it wants to know if character undelete is
    /// available. We always respond with "no cooldown" (undelete available).
    pub async fn handle_get_undelete_cooldown_status(&mut self) {
        self.send_packet(&wow_packet::packets::misc::UndeleteCooldownStatusResponse::no_cooldown());
    }

    /// Handle CMSG_ALTER_APPEARANCE.
    ///
    /// C++ `HandleAlterAppearance` validates customization DB2 requirements,
    /// requires the player to be sitting on a nearby barber chair, checks
    /// `GetBarberShopCost`, sends `SMSG_BARBER_SHOP_RESULT`, then mutates
    /// player gender/customizations and criteria.
    ///
    /// Rust currently represents barber-chair use and stand-state, but does
    /// not yet own the full ChrCustomization/BarberShop cost/runtime mutation.
    /// This seam preserves packet/dispatch, the C++ not-on-chair result, and
    /// records accepted requests without fabricating the full appearance change.
    pub async fn handle_alter_appearance(&mut self, mut pkt: WorldPacket) {
        let request = match AlterAppearance::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad AlterAppearance: {error}");
                return;
            }
        };

        if !self.represented_is_on_barber_chair_like_cpp() {
            self.send_packet(&BarberShopResult {
                result: BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP,
            });
            return;
        }

        let cost = 0;
        self.send_packet(&BarberShopResult {
            result: BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
        });
        self.record_represented_alter_appearance_like_cpp(RepresentedAlterAppearanceLikeCpp {
            new_sex: request.new_sex,
            customizations: request.customizations,
            customized_race: request.customized_race,
            customized_chr_model_id: request.customized_chr_model_id,
            cost,
        });
    }

    /// Handle CMSG_CONFIRM_BARBERS_CHOICE.
    ///
    /// C++ `HandleConfirmBarbersChoice` converts the barber rows into
    /// `ChrCustomizationChoice`, checks `GetBarberShopCost`, sends only the
    /// no-money failure, and otherwise mutates money/customizations/criteria
    /// without a success packet. Rust records the accepted request until the
    /// Player customization/cost/criteria runtime is canonical.
    pub async fn handle_confirm_barbers_choice(&mut self, mut pkt: WorldPacket) {
        let request = match ConfirmBarbersChoice::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad ConfirmBarbersChoice: {error}");
                return;
            }
        };

        let cost = 0;
        self.record_represented_confirm_barbers_choice_like_cpp(
            RepresentedConfirmBarbersChoiceLikeCpp {
                customizations: request.customizations,
                cost,
            },
        );
    }

    /// Handle CMSG_SET_PLAYER_DECLINED_NAMES.
    ///
    /// C++ resolves the target character through `sCharacterCache`, requires a
    /// Cyrillic base name, normalizes all five declined forms, validates them
    /// with `ObjectMgr::CheckDeclinedNames`, then replaces the
    /// `character_declinedname` row and returns success. Rust does not yet
    /// carry that character-cache / locale-validation runtime through this
    /// session path, so this bounded seam preserves the parse/dispatch and the
    /// C++ error-result branch instead of fabricating persisted declined names.
    pub async fn handle_set_player_declined_names(&mut self, mut pkt: WorldPacket) {
        let request = match SetPlayerDeclinedNames::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad SetPlayerDeclinedNames: {error}");
                return;
            }
        };

        self.send_packet(&SetPlayerDeclinedNamesResult {
            player: request.player,
            result_code: DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
        });
    }

    pub async fn handle_query_corpse_location(&mut self, query: QueryCorpseLocationFromClient) {
        // C++ sends an invalid CorpseLocation when the queried player is missing,
        // has no corpse, or is not in the querying player's raid. Rust does not
        // yet have the live corpse/raid lookup needed for the valid branch.
        self.send_packet(&CorpseLocation::not_found_like_cpp(query.player));
    }

    pub async fn handle_query_corpse_transport(&mut self, query: QueryCorpseTransport) {
        // C++ always sends CorpseTransportQuery. Position/facing remain default
        // unless the queried player is in raid and has a corpse on this transport.
        self.send_packet(&CorpseTransportQuery::not_found_like_cpp(query.player));
    }

    /// CMSG_HEARTH_AND_RESURRECT — battlefield hearth/resurrection escape.
    /// C++ ref: `WorldSession::HandleHearthAndResurrect`.
    pub async fn handle_hearth_and_resurrect(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = HearthAndResurrect::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "HearthAndResurrect parse failed: {error}"
            );
            return;
        }

        if self.is_in_taxi_flight_like_cpp() {
            return;
        }

        let (_, area_id) = self.player_zone_area_like_cpp();
        let Some(area_table_store) = self.area_table_store() else {
            debug!(
                account = self.account_id,
                area_id, "HearthAndResurrect ignored without represented AreaTableStore"
            );
            return;
        };
        let Some(area_entry) = area_table_store.get(area_id) else {
            return;
        };
        if !area_entry.allow_hearth_and_resurrect_from_area_like_cpp() {
            return;
        }

        // C++ first lets Battlefield own the leave flow when one exists. Rust
        // has no battlefield manager attached to WorldSession yet, so this
        // represented branch covers the AreaTable/homebind path only.
        self.apply_represented_resurrection_percent_like_cpp(1.0);
        if let Some(homebind) = self.represented_homebind_like_cpp() {
            self.teleport_to(homebind.map_id, homebind.position).await;
        }
    }

    /// C++ `Player::LoadFromDB`: `CHAR_SEL_CHARACTER_CUSTOMIZATIONS`.
    ///
    /// The rows are copied into `PlayerData::Customizations` before
    /// `PlayerData::WriteCreate`, and each element serializes as
    /// `(ChrCustomizationOptionID, ChrCustomizationChoiceID)` uint32s.
    pub(crate) async fn load_player_customizations_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Vec<ChrCustomizationChoiceValuesUpdate> {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return Vec::new();
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Customizations(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    player_guid = guid.counter(),
                    "Failed to load character customizations: {reason}"
                );
                return Vec::new();
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    player_guid = guid.counter(),
                    "Player lifecycle port returned the wrong auxiliary login data for customizations"
                );
                return Vec::new();
            }
        };
        let customizations = rows
            .into_iter()
            .map(|row| ChrCustomizationChoiceValuesUpdate {
                option_id: row.option_id,
                choice_id: row.choice_id,
            })
            .collect::<Vec<_>>();

        info!(
            player_guid = guid.counter(),
            customizations = customizations.len(),
            "Loaded character customizations like C++"
        );
        customizations
    }

    pub(super) fn load_default_graveyard_homebind_like_cpp(
        &self,
        race: u8,
    ) -> Option<CharacterLoginLocationLikeCpp> {
        let [primary_safe_loc_id, neutral_pandaren_safe_loc_id] =
            default_graveyard_safe_loc_ids_for_race_like_cpp(race);
        let primary_safe_loc_id = primary_safe_loc_id?;
        let store = self.world_safe_loc_store_like_cpp()?;
        store
            .get(primary_safe_loc_id)
            .or_else(|| neutral_pandaren_safe_loc_id.and_then(|id| store.get(id)))
            .map(|safe_loc| CharacterLoginLocationLikeCpp {
                map_id: safe_loc.map_id,
                bind_area_id: None,
                position: safe_loc.position,
            })
    }

    /// Load the map's persisted corpses once, including the two auxiliary
    /// tables consumed by C++ `Map::LoadCorpseData` before `AddCorpse`.
    pub(super) async fn load_map_corpse_data_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> MapCorpseLoadOutcomeLikeCpp {
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return MapCorpseLoadOutcomeLikeCpp::default();
        };
        {
            let Ok(manager) = manager.lock() else {
                warn!(
                    map_id,
                    instance_id, "Cannot inspect canonical map corpse-load state: lock poisoned"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            };
            let Some(map) = manager.find_map(u32::from(map_id), instance_id) else {
                warn!(
                    map_id,
                    instance_id, "Cannot load C++ map corpses: canonical map is unavailable"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            };
            if map.map().corpse_data_loaded_like_cpp() {
                return MapCorpseLoadOutcomeLikeCpp {
                    already_loaded: true,
                    ..Default::default()
                };
            }
        }

        let Some(port) = self.map_corpse_persistence_port_like_cpp().map(Arc::clone) else {
            return MapCorpseLoadOutcomeLikeCpp::default();
        };
        let (corpse_rows, phase_rows, customization_rows) = match port
            .load_map_corpses_like_cpp(wow_persistence::MapCorpseLoadRequestLikeCpp {
                map_id: u32::from(map_id),
                instance_id,
            })
            .await
        {
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Loaded {
                corpses,
                phases,
                customizations,
            } => (corpses, phases, customizations),
            wow_persistence::MapCorpseLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id, "C++ Map::LoadCorpseData base query failed: {reason}"
                );
                return MapCorpseLoadOutcomeLikeCpp::default();
            }
        };

        let mut rows = Vec::with_capacity(corpse_rows.len());
        let mut invalid_type_rows = 0u32;
        for row in corpse_rows {
            let corpse_type = match row.corpse_type {
                1 => Some(CorpseType::ResurrectablePve),
                2 => Some(CorpseType::ResurrectablePvp),
                // C++ rejects bones and values >= MAX_CORPSE_TYPE.
                _ => None,
            };
            if let Some(corpse_type) = corpse_type {
                rows.push(LoadedMapCorpseRowLikeCpp {
                    position: Position::new(row.pos_x, row.pos_y, row.pos_z, row.orientation),
                    map_id: row.map_id,
                    display_id: row.display_id,
                    items: parse_corpse_items_like_cpp(&row.item_cache),
                    race: row.race,
                    class: row.class,
                    sex: row.sex,
                    flags: u32::from(row.flags),
                    dynamic_flags: u32::from(row.dynamic_flags),
                    ghost_time: i64::from(row.ghost_time),
                    corpse_type,
                    instance_id: row.instance_id,
                    owner_db_guid: row.owner_guid,
                });
            } else {
                invalid_type_rows = invalid_type_rows.saturating_add(1);
            }
        }

        let mut phases = HashMap::<u64, BTreeSet<u32>>::new();
        let mut customizations = HashMap::<u64, Vec<CorpseCustomizationChoice>>::new();
        match phase_rows {
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(phase_rows) => {
                for row in phase_rows {
                    phases
                        .entry(row.owner_guid)
                        .or_default()
                        .insert(row.phase_id);
                }
            }
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id,
                    "C++ Map::LoadCorpseData phase query failed; continuing without phases: {reason}"
                );
            }
        }
        match customization_rows {
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(customization_rows) => {
                for row in customization_rows {
                    customizations.entry(row.owner_guid).or_default().push(
                        CorpseCustomizationChoice {
                            option_id: row.option_id,
                            choice_id: row.choice_id,
                        },
                    );
                }
            }
            wow_persistence::MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    map_id,
                    instance_id,
                    "C++ Map::LoadCorpseData customization query failed; continuing without customizations: {reason}"
                );
            }
        }

        let faction_templates_by_race = rows
            .iter()
            .filter_map(|row| {
                self.faction_template_for_race_like_cpp(row.race)
                    .map(|faction| (row.race, faction))
            })
            .collect::<HashMap<_, _>>();
        let Ok(mut manager) = manager.lock() else {
            warn!(
                map_id,
                instance_id, "Cannot materialize canonical map corpses: lock poisoned"
            );
            return MapCorpseLoadOutcomeLikeCpp {
                invalid_type_rows,
                ..Default::default()
            };
        };
        let Some(map) = manager.find_map_mut(u32::from(map_id), instance_id) else {
            warn!(
                map_id,
                instance_id, "Cannot materialize C++ map corpses: canonical map disappeared"
            );
            return MapCorpseLoadOutcomeLikeCpp {
                invalid_type_rows,
                ..Default::default()
            };
        };
        let mut outcome = materialize_loaded_map_corpses_like_cpp(
            map.map_mut(),
            self.realm_id(),
            rows,
            &phases,
            &customizations,
            &faction_templates_by_race,
        );
        outcome.invalid_type_rows = outcome.invalid_type_rows.saturating_add(invalid_type_rows);
        outcome
    }
}
