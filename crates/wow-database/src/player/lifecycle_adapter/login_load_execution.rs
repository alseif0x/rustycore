use super::*;

impl MariaDbPlayerLifecycleAdapterLikeCpp {
    pub(super) fn execute_account_collection_load_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = account_collection_load_statements_like_cpp(request);
            match request {
                AccountCollectionLoadRequestLikeCpp::Mounts { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMountLoadRowLikeCpp {
                                        mount_spell_id: result.try_read::<i32>(0).unwrap_or(0),
                                        flags: result.try_read::<u8>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Mounts(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::Toys { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountToyLoadRowLikeCpp {
                                        item_id: result.try_read::<i32>(0).unwrap_or(0),
                                        is_favorite: result.try_read::<bool>(1).unwrap_or(false),
                                        has_fanfare: result.try_read::<bool>(2).unwrap_or(false),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Toys(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::Heirlooms { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountHeirloomLoadRowLikeCpp {
                                        item_id: result.try_read::<i32>(0).unwrap_or(0),
                                        flags: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Heirlooms(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::ItemAppearances { .. } => {
                    let appearance_blocks = match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMaskBlockLikeCpp {
                                        block_index: result.try_read::<u32>(0).unwrap_or(0),
                                        mask: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionRowsLikeCpp::Loaded(rows)
                        }
                        Err(error) => AccountCollectionRowsLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    };
                    let favorite_appearance_ids = match self.login_db.query(&statements[1]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(result.try_read::<u32>(0).unwrap_or(0));
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionRowsLikeCpp::Loaded(rows)
                        }
                        Err(error) => AccountCollectionRowsLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    };
                    AccountCollectionLoadOutcomeLikeCpp::Loaded(
                        AccountCollectionLoadedLikeCpp::ItemAppearances {
                            appearance_blocks,
                            favorite_appearance_ids,
                        },
                    )
                }
                AccountCollectionLoadRequestLikeCpp::TransmogIllusions { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMaskBlockLikeCpp {
                                        block_index: result.try_read::<u32>(0).unwrap_or(0),
                                        mask: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::TransmogIllusions {
                                    illusion_blocks: rows,
                                },
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
            }
        })
    }

    pub(super) fn execute_login_auxiliary_load_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_login_auxiliary_load_statement_like_cpp(request);
            let mut result = match self.character_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            let loaded = match request {
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Mail { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerMailLoadRowLikeCpp {
                                mail_id: result.try_read(0).unwrap_or(0),
                                message_type: result.try_read(1).unwrap_or(0),
                                sender: result.try_read(2).unwrap_or(0),
                                receiver: result.try_read(3).unwrap_or(0),
                                expire_time: result.try_read(6).unwrap_or(0),
                                deliver_time: result.try_read(7).unwrap_or(0),
                                checked_flags: result.try_read(10).unwrap_or(0),
                                stationery_id: result.try_read(11).unwrap_or(0),
                                template_id: result.try_read(12).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Mail(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCustomizationLoadRowLikeCpp {
                                option_id: result.try_read::<u32>(0).unwrap_or(0),
                                choice_id: result.try_read::<u32>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Customizations(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(result.try_read::<u32>(0).unwrap_or(0));
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CompletedAchievements(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerInstanceTimeRestrictionLoadRowLikeCpp {
                                instance_id: result.try_read::<u32>(0).unwrap_or(0),
                                release_time: result.try_read::<u64>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerSpellCooldownLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                item_id: result.try_read::<u32>(1).unwrap_or(0),
                                cooldown_end: result.try_read::<i64>(2).unwrap_or(0),
                                category_id: result.try_read::<u32>(3).unwrap_or(0),
                                category_end: result.try_read::<i64>(4).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::SpellCooldowns(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerSpellChargeLoadRowLikeCpp {
                                category_id: result.try_read::<u32>(0).unwrap_or(0),
                                recharge_start: result.try_read::<i64>(1).unwrap_or(0),
                                recharge_end: result.try_read::<i64>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::SpellCharges(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerTraitEntryLoadRowLikeCpp {
                                trait_config_id: result.try_read::<i32>(0),
                                trait_node_id: result.try_read::<i32>(1),
                                trait_node_entry_id: result.try_read::<i32>(2),
                                rank: result.try_read::<i32>(3),
                                granted_ranks: result.try_read::<i32>(4),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerTraitConfigLoadRowLikeCpp {
                                id: result.try_read::<i32>(0),
                                config_type: result.try_read::<i32>(1),
                                chr_specialization_id: result.try_read::<i32>(2),
                                combat_config_flags: result.try_read::<i32>(3),
                                local_identifier: result.try_read::<i32>(4),
                                skill_line_id: result.try_read::<i32>(5),
                                trait_system_id: result.try_read::<i32>(6),
                                name: result.try_read::<String>(7),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetStableLoadRowLikeCpp {
                                pet_number: result.try_read::<u32>(0).unwrap_or(0),
                                creature_id: result.try_read::<u32>(1).unwrap_or(0),
                                display_id: result.try_read::<u32>(2).unwrap_or(0),
                                level: result.try_read::<u8>(3).unwrap_or(1),
                                experience: result.try_read::<u32>(4).unwrap_or(0),
                                react_state: result.try_read::<u8>(5).unwrap_or(0),
                                slot: result.try_read::<i16>(6).unwrap_or(-1),
                                name: result.read_string(7),
                                was_renamed: result.try_read::<bool>(8).unwrap_or(false),
                                health: result.try_read::<u32>(9).unwrap_or(1),
                                mana: result.try_read::<u32>(10).unwrap_or(0),
                                action_bar: result.try_read::<String>(11).unwrap_or_default(),
                                last_save_time: result.try_read::<u32>(12).unwrap_or(0),
                                created_by_spell_id: result.try_read::<u32>(13).unwrap_or(0),
                                pet_type: result.try_read::<u8>(14).unwrap_or(0),
                                specialization_id: result.try_read::<u16>(15).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetStable(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetAuraLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(1).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(2).unwrap_or(0),
                                recalculate_mask: result.try_read::<u32>(3).unwrap_or(0),
                                difficulty: result.try_read::<u8>(4).unwrap_or(0),
                                stack_count: result.try_read::<u8>(5).unwrap_or(0),
                                max_duration_ms: result.try_read::<i32>(6).unwrap_or(0),
                                remain_time_ms: result.try_read::<i32>(7).unwrap_or(0),
                                remain_charges: result.try_read::<u8>(8).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetAuras(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetAuraEffectLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(1).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(2).unwrap_or(0),
                                effect_index: result.try_read::<u8>(3).unwrap_or(0),
                                amount: result.try_read::<i32>(4).unwrap_or(0),
                                base_amount: result.try_read::<i32>(5).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetAuraEffects(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                active: result.try_read::<u8>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellCooldownLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                cooldown_end_unix_secs: result.try_read::<i64>(1).unwrap_or(0),
                                category_id: result.try_read::<u32>(2).unwrap_or(0),
                                category_end_unix_secs: result.try_read::<i64>(3).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCooldowns(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellChargeLoadRowLikeCpp {
                                category_id: result.try_read::<u32>(0).unwrap_or(0),
                                recharge_start_unix_secs: result.try_read::<i64>(1).unwrap_or(0),
                                recharge_end_unix_secs: result.try_read::<i64>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCharges(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        rows.push(PlayerPetDeclinedNamesLoadRowLikeCpp {
                            names: [
                                result.read_string(0),
                                result.read_string(1),
                                result.read_string(2),
                                result.read_string(3),
                                result.read_string(4),
                            ],
                        });
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetDeclinedNames(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(result.try_read::<u32>(0).unwrap_or(0));
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentSets { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerEquipmentSetLoadRowLikeCpp {
                                set_guid: result.try_read::<u64>(0).unwrap_or(0),
                                set_id: result.try_read::<u8>(1).unwrap_or(0),
                                name: result.try_read(2).unwrap_or_default(),
                                icon: result.try_read(3).unwrap_or_default(),
                                ignore_mask: result.try_read::<u32>(4).unwrap_or(0),
                                assigned_spec_index: result.try_read::<i32>(5).unwrap_or(-1),
                                item_low_guids: (0..19)
                                    .map(|slot| result.try_read::<u64>(6 + slot).unwrap_or(0))
                                    .collect(),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentSets(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TransmogOutfits { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            let set_guid = result
                                .try_read::<i64>(0)
                                .and_then(nonnegative_i64_to_u64_like_cpp)
                                .or_else(|| result.try_read::<u64>(0))
                                .unwrap_or(0);
                            let ignore_mask = result
                                .try_read::<i32>(4)
                                .and_then(nonnegative_i32_to_u32_like_cpp)
                                .or_else(|| result.try_read::<u32>(4))
                                .unwrap_or(0);
                            rows.push(PlayerTransmogOutfitLoadRowLikeCpp {
                                set_guid,
                                set_id: result.try_read::<u8>(1).unwrap_or(0),
                                name: result.try_read(2).unwrap_or_default(),
                                icon: result.try_read(3).unwrap_or_default(),
                                ignore_mask,
                                appearances: (0..19)
                                    .map(|slot| result.try_read::<i32>(5 + slot).unwrap_or(0))
                                    .collect(),
                                enchants: [
                                    result.try_read::<i32>(24).unwrap_or(0),
                                    result.try_read::<i32>(25).unwrap_or(0),
                                ],
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::TransmogOutfits(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCufProfileLoadRowLikeCpp {
                                id: result.try_read::<u8>(0).unwrap_or(0),
                                name: result.try_read(1).unwrap_or_default(),
                                frame_height: result.try_read::<u16>(2).unwrap_or(0),
                                frame_width: result.try_read::<u16>(3).unwrap_or(0),
                                sort_by: result.try_read::<u8>(4).unwrap_or(0),
                                health_text: result.try_read::<u8>(5).unwrap_or(0),
                                bool_options: result.try_read::<u32>(6).unwrap_or(0),
                                top_point: result.try_read::<u8>(7).unwrap_or(0),
                                bottom_point: result.try_read::<u8>(8).unwrap_or(0),
                                left_point: result.try_read::<u8>(9).unwrap_or(0),
                                top_offset: result.try_read::<u16>(10).unwrap_or(0),
                                bottom_offset: result.try_read::<u16>(11).unwrap_or(0),
                                left_offset: result.try_read::<u16>(12).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CufProfiles(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCurrencyLoadRowLikeCpp {
                                currency_id: result.try_read::<u16>(0).unwrap_or(0),
                                quantity: result.try_read::<u32>(1).unwrap_or(0),
                                weekly_quantity: result.try_read::<u32>(2).unwrap_or(0),
                                tracked_quantity: result.try_read::<u32>(3).unwrap_or(0),
                                increased_cap_quantity: result.try_read::<u32>(4).unwrap_or(0),
                                earned_quantity: result.try_read::<u32>(5).unwrap_or(0),
                                flags: result.try_read::<u8>(6).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Currencies(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerSpellLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                active: result.try_read::<u8>(1).unwrap_or(1),
                                disabled: result.try_read::<u8>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Spells(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(result.try_read::<u32>(0).unwrap_or(0));
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::SpellFavorites(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            let value = result.try_read::<u16>(1).unwrap_or(0);
                            rows.push(PlayerSkillLoadRowLikeCpp {
                                skill_id: result.try_read::<u16>(0).unwrap_or(0),
                                value,
                                max: result.try_read::<u16>(2).unwrap_or(value),
                                profession_slot: result.try_read::<i8>(3).unwrap_or(-1),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Skills(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerTalentLoadRowLikeCpp {
                                talent_id: result.try_read::<u32>(0).unwrap_or(0),
                                rank: result.try_read::<u8>(1).unwrap_or(0),
                                talent_group: result.try_read::<u8>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Talents(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerGlyphLoadRowLikeCpp {
                                talent_group: result.try_read::<u8>(0).unwrap_or(0),
                                glyph_slot: result.try_read::<u8>(1).unwrap_or(0),
                                glyph_id: result.try_read::<u16>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Glyphs(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerActionButtonLoadRowLikeCpp {
                                button: result.read(0),
                                action: result.try_read::<u32>(1).unwrap_or(0),
                                button_type: result.try_read::<u8>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::ActionButtons(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerReputationLoadRowLikeCpp {
                                faction_id: result.try_read::<u16>(0).unwrap_or(0),
                                standing: result.try_read::<i32>(1).unwrap_or(0),
                                flags: result.try_read::<u16>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Reputation(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCharacterAuraLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(2).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(3).unwrap_or(0),
                                recalculate_mask: result.try_read::<u32>(4).unwrap_or(0),
                                difficulty: result.try_read::<u8>(5).unwrap_or(0),
                                stack_count: result.try_read::<u8>(6).unwrap_or(1),
                                max_duration_ms: result.try_read::<i32>(7).unwrap_or(0),
                                remain_time_ms: result.try_read::<i32>(8).unwrap_or(0),
                                remain_charges: result.try_read::<u8>(9).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuras(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCharacterAuraEffectLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(2).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(3).unwrap_or(0),
                                effect_index: result.try_read::<u8>(4).unwrap_or(0),
                                amount: result.try_read::<i32>(5).unwrap_or(0),
                                base_amount: result.try_read::<i32>(6).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuraEffects(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerEquipmentInventoryLoadRowLikeCpp {
                                slot: result.read(0),
                                item: player_inventory_item_load_row_like_cpp(&result, 1),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentInventory(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerBagInventoryLoadRowLikeCpp {
                                bag_slot: result.read(0),
                                inner_slot: result.read(1),
                                item: player_inventory_item_load_row_like_cpp(&result, 2),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::BagInventory(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerVoidStorageLoadRowLikeCpp {
                                item_id: result.try_read::<u64>(0).unwrap_or(0),
                                item_entry: result.try_read::<u32>(1).unwrap_or(0),
                                slot: result.try_read::<u8>(2).unwrap_or(u8::MAX),
                                creator_guid: result.try_read::<u64>(3).unwrap_or(0),
                                fixed_scaling_level: result.try_read::<u32>(4).unwrap_or(0),
                                random_properties_id: result.try_read::<i32>(5).unwrap_or(0),
                                random_properties_seed: result.try_read::<i32>(6).unwrap_or(0),
                                context: result.try_read::<u8>(7).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::VoidStorage(rows)
                }
            };
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(loaded)
        })
    }
}
