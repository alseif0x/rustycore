// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow canonical owner for a full Player save snapshot and its acknowledgement.
//!
//! The map-manager guard is used only while producing owned values or applying
//! the incarnation-bound acknowledgement. No Player, map guard or callback
//! crosses this module boundary.

use std::collections::HashSet;
use std::time::Instant;

use wow_entities::{
    Player, PlayerEquipmentSetTypeLikeCpp as EntitiesPlayerEquipmentSetTypeLikeCpp,
    PlayerEquipmentSetUpdateStateLikeCpp, PlayerSpellLoadState,
};
use wow_map::{PlayerHandle, PlayerResidenceLikeCpp};
use wow_persistence::{
    PlayerActionButtonSaveLikeCpp, PlayerActionButtonsSaveLikeCpp,
    PlayerCharacterCommittedGroupsLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSnapshotSaveLikeCpp, PlayerCufProfileSaveLikeCpp,
    PlayerCufProfileSlotSaveLikeCpp, PlayerEquipmentSetSaveLikeCpp,
    PlayerEquipmentSetStateLikeCpp, PlayerEquipmentSetTypeLikeCpp,
    PlayerFallbackSpellSaveLikeCpp, PlayerGlyphSaveLikeCpp,
    PlayerInstanceLockTimeSaveLikeCpp, PlayerPlayedTimeSaveLikeCpp,
    PlayerPositionSaveLikeCpp, PlayerReputationSaveLikeCpp, PlayerSkillSaveLikeCpp,
    PlayerSpellChargeSaveLikeCpp, PlayerSpellCooldownSaveLikeCpp,
    PlayerSpellSaveGroupLikeCpp, PlayerSpellSaveLikeCpp, PlayerSpellStateLikeCpp,
    PlayerTalentSaveLikeCpp, PlayerTutorialsSaveLikeCpp, PlayerVoidStorageSaveLikeCpp,
    PlayerVoidStorageSlotSaveLikeCpp,
};

use crate::session::persistence_capabilities::{
    PlayerSaveToDbSnapshotLikeCpp, character_power_snapshot_values_like_cpp,
    loaded_character_power_snapshot_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::persistence_capabilities::CharacterPowerSnapshotLikeCpp;
use crate::session::{RepresentedPlayerSkillStateLikeCpp, SessionCore, represented_player_skill_record_like_cpp};

/// Lifecycle-owned values that join the canonical Player projection in one save request.
pub struct PlayerSaveSessionInputsLikeCpp<'a> {
    now_unix_secs: i64,
    tutorials_changed: &'a bool,
    tutorials_loaded_coherently: &'a bool,
    tutorials: &'a [u32; 8],
    tutorials_loaded_from_db: &'a bool,
    login_time: &'a Option<Instant>,
    total_played_time: &'a u32,
    level_played_time: &'a u32,
}

impl<'a> PlayerSaveSessionInputsLikeCpp<'a> {
    pub fn new_like_cpp(
        now_unix_secs: i64,
        tutorials_changed: &'a bool,
        tutorials_loaded_coherently: &'a bool,
        tutorials: &'a [u32; 8],
        tutorials_loaded_from_db: &'a bool,
        login_time: &'a Option<Instant>,
        total_played_time: &'a u32,
        level_played_time: &'a u32,
    ) -> Self {
        Self {
            now_unix_secs,
            tutorials_changed,
            tutorials_loaded_coherently,
            tutorials,
            tutorials_loaded_from_db,
            login_time,
            total_played_time,
            level_played_time,
        }
    }
}

/// Owned request and receipt captured from one canonical Player incarnation.
pub struct CapturedPlayerSaveLikeCpp {
    request: PlayerCharacterSaveRequestLikeCpp,
    header: PlayerSaveToDbSnapshotLikeCpp,
    receipt: PlayerSaveReceiptLikeCpp,
}

impl CapturedPlayerSaveLikeCpp {
    pub fn into_parts_like_cpp(
        self,
    ) -> (
        PlayerCharacterSaveRequestLikeCpp,
        PlayerSaveToDbSnapshotLikeCpp,
        PlayerSaveReceiptLikeCpp,
    ) {
        (self.request, self.header, self.receipt)
    }
}

/// Opaque, single-use acknowledgement data for the captured Player incarnation.
pub struct PlayerSaveReceiptLikeCpp {
    handle: PlayerHandle,
    owner: wow_entities::PlayerSaveAcknowledgementLikeCpp,
    expected: PlayerCharacterCommittedGroupsLikeCpp,
    tutorials: Option<PlayerTutorialsSaveLikeCpp>,
}

/// Session-owned tutorial work to publish after canonical Player acknowledgement.
pub struct AcknowledgedPlayerSaveLikeCpp {
    pub groups: PlayerCharacterCommittedGroupsLikeCpp,
    pub tutorials: Option<PlayerTutorialsSaveLikeCpp>,
}

/// Read-only canonical Player-save capture and acknowledgement capability.
pub struct PlayerSaveOwnerAccessLikeCpp<'a> {
    core: &'a SessionCore,
    talent_store: Option<&'a wow_data::TalentStore>,
    spell_store: Option<&'a wow_data::SpellStore>,
}

impl<'a> PlayerSaveOwnerAccessLikeCpp<'a> {
    pub fn new_like_cpp(
        core: &'a SessionCore,
        talent_store: Option<&'a wow_data::TalentStore>,
        spell_store: Option<&'a wow_data::SpellStore>,
    ) -> Self {
        Self {
            core,
            talent_store,
            spell_store,
        }
    }

    pub fn capture_like_cpp(
        &self,
        inputs: PlayerSaveSessionInputsLikeCpp<'_>,
    ) -> Option<CapturedPlayerSaveLikeCpp> {
        let handle = self.core.player_handle_like_cpp?;
        if self.core.player_guid()? != handle.guid() {
            return None;
        }
        let manager = self.core.canonical_map_manager.as_ref()?.lock().ok()?;
        let residence = manager.player_residence_like_cpp(handle)?;
        manager.with_player_like_cpp(handle, |player| {
            let teleport = player.teleport_state_like_cpp();
            if teleport.post_add.is_some()
                || (teleport.far_pending
                    && teleport.recovery != wow_entities::PlayerTransferRecovery::Terminal)
            {
                return None;
            }
            let header = player_save_header_like_cpp(player, residence);
            if teleport.recovery == wow_entities::PlayerTransferRecovery::Terminal {
                let world = player.unit().world();
                if !world.position().is_valid_map_coord_like_cpp()
                    || u32::from(header.map_id) != world.map_id()
                    || header.instance_id != world.instance_id()
                    || header.position != world.position()
                {
                    return None;
                }
            }
            let request = request(
                self.core,
                self.talent_store,
                self.spell_store,
                player,
                &header,
                &inputs,
            )?;
            Some(CapturedPlayerSaveLikeCpp {
                receipt: PlayerSaveReceiptLikeCpp {
                    handle,
                    owner: player.capture_save_acknowledgement_like_cpp(),
                    expected: request.committed_groups_like_cpp(),
                    tutorials: request.tutorials.clone(),
                },
                request,
                header,
            })
        })?
    }

    /// Resolve the selected session owner without exposing the Player handle.
    pub fn player_guid_like_cpp(&self) -> Option<wow_core::ObjectGuid> {
        self.core.player_guid()
    }

    /// Account identity used by the save-operation diagnostics.
    pub fn account_id_like_cpp(&self) -> u32 {
        self.core.account_id
    }

    /// Borrow the established money owner for the save's pre-capture phases.
    pub fn inventory_access_like_cpp(
        &self,
    ) -> crate::session::OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    /// Borrow the existing session packet channel for ordered save-side effects.
    pub fn packet_publication_access_like_cpp(
        &self,
    ) -> crate::session::PacketPublicationAccessLikeCpp<'_> {
        self.core.packet_publication_access_like_cpp()
    }

    pub fn header_like_cpp(core: &SessionCore) -> Option<PlayerSaveToDbSnapshotLikeCpp> {
        let handle = core.player_handle_like_cpp?;
        if core.player_guid()? != handle.guid() {
            return None;
        }
        let manager = core.canonical_map_manager.as_ref()?.lock().ok()?;
        let residence = manager.player_residence_like_cpp(handle)?;
        manager.with_player_like_cpp(handle, |player| player_save_header_like_cpp(player, residence))
    }

    /// Resolve the legacy ownerless test fixture's canonical map projection without
    /// exposing a Player or map-manager guard to the World test adapter.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_canonical_header_like_cpp(
        core: &SessionCore,
        guid: wow_core::ObjectGuid,
        level: u8,
        xp: u32,
        money: u64,
        powers: CharacterPowerSnapshotLikeCpp,
        pending_teleport_destination: Option<(u16, wow_core::Position)>,
    ) -> Option<PlayerSaveToDbSnapshotLikeCpp> {
        let manager = core.canonical_map_manager.as_ref()?.lock().ok()?;
        let mut snapshot = None;
        manager.do_for_all_maps(|managed| {
            if snapshot.is_some() {
                return;
            }
            let Some(player) = managed.map().get_typed_player(guid) else {
                return;
            };
            let (map_id, instance_id, position) =
                if let Some((map_id, position)) = pending_teleport_destination {
                    (map_id, 0, position)
                } else {
                    (
                        core.player_map_id_like_cpp(),
                        managed.instance_id(),
                        player.unit().world().position(),
                    )
                };
            let max_health = player
                .unit()
                .data()
                .max_health
                .max(1)
                .min(u64::from(u32::MAX)) as u32;
            let canonical_health = player.unit().data().health.min(u64::from(u32::MAX)) as u32;
            let health = if player.unit().is_alive() && canonical_health > 0 {
                canonical_health
            } else {
                0
            };
            snapshot = Some(PlayerSaveToDbSnapshotLikeCpp {
                guid,
                map_id,
                instance_id,
                position,
                level,
                xp,
                money,
                health,
                max_health,
                powers,
            });
        });
        snapshot
    }

    pub fn acknowledge_like_cpp(
        core: &SessionCore,
        receipt: PlayerSaveReceiptLikeCpp,
        committed: &PlayerCharacterCommittedGroupsLikeCpp,
    ) -> Option<AcknowledgedPlayerSaveLikeCpp> {
        let expected = receipt.expected;
        let groups = PlayerCharacterCommittedGroupsLikeCpp {
            player_spells: expected.player_spells && committed.player_spells,
            fallback_player_spells: expected.fallback_player_spells
                && committed.fallback_player_spells,
            player_skills: expected.player_skills && committed.player_skills,
            equipment_sets: expected.equipment_sets && committed.equipment_sets,
            tutorials_changed: expected.tutorials_changed && committed.tutorials_changed,
            tutorials_insert: expected.tutorials_insert && committed.tutorials_insert,
            reputation: expected.reputation && committed.reputation,
        };
        let handle = receipt.handle;
        if core.player_handle_like_cpp != Some(handle)
            || core.player_guid() != Some(handle.guid())
        {
            return None;
        }
        let manager = core.canonical_map_manager.as_ref()?;
        let acknowledged = manager
            .lock()
            .ok()
            .and_then(|mut manager| {
                manager.with_player_mut_like_cpp(handle, |player| {
                    player.acknowledge_saved_projection_like_cpp(
                        receipt.owner,
                        wow_entities::PlayerSavedGroupsLikeCpp {
                            spells: groups.player_spells,
                            fallback_spells: groups.fallback_player_spells,
                            skills: groups.player_skills,
                            equipment: groups.equipment_sets,
                            reputations: groups.reputation,
                        },
                    );
                })
            })
            .is_some();
        acknowledged.then_some(AcknowledgedPlayerSaveLikeCpp {
            groups,
            tutorials: receipt.tutorials,
        })
    }
}

/// Mutable save-operation capability for phases that may quarantine the session.
///
/// It retains only the selected save catalogs and a mutable borrow of SessionCore.
/// Capture reborrows the established read-only owner at the point where the save
/// request is produced; no Player or Core reference escapes this capability.
pub struct PlayerSaveOperationAccessLikeCpp<'a> {
    core: &'a mut SessionCore,
    talent_store: Option<&'a wow_data::TalentStore>,
    spell_store: Option<&'a wow_data::SpellStore>,
}

impl SessionCore {
    /// Borrow the mutable owner needed by the complete save operation.
    pub fn player_save_operation_access_like_cpp<'a>(
        &'a mut self,
        talent_store: Option<&'a wow_data::TalentStore>,
        spell_store: Option<&'a wow_data::SpellStore>,
    ) -> PlayerSaveOperationAccessLikeCpp<'a> {
        PlayerSaveOperationAccessLikeCpp {
            core: self,
            talent_store,
            spell_store,
        }
    }
}

impl PlayerSaveOperationAccessLikeCpp<'_> {
    /// Capture through the existing read-only owner using this operation's stores.
    pub fn capture_like_cpp(
        &self,
        inputs: PlayerSaveSessionInputsLikeCpp<'_>,
    ) -> Option<CapturedPlayerSaveLikeCpp> {
        PlayerSaveOwnerAccessLikeCpp::new_like_cpp(
            &*self.core,
            self.talent_store,
            self.spell_store,
        )
        .capture_like_cpp(inputs)
    }

    /// Apply a committed receipt through the existing incarnation-bound owner.
    pub fn acknowledge_like_cpp(
        &self,
        receipt: PlayerSaveReceiptLikeCpp,
        committed: &PlayerCharacterCommittedGroupsLikeCpp,
    ) -> Option<AcknowledgedPlayerSaveLikeCpp> {
        PlayerSaveOwnerAccessLikeCpp::acknowledge_like_cpp(
            &*self.core,
            receipt,
            committed,
        )
    }

    /// Resolve the selected session Player GUID without exposing Core state.
    pub fn player_guid_like_cpp(&self) -> Option<wow_core::ObjectGuid> {
        self.core.player_guid()
    }

    /// Return the account identity used by save-operation diagnostics.
    pub fn account_id_like_cpp(&self) -> u32 {
        self.core.account_id
    }

    /// Borrow the established money owner for a save-operation phase.
    pub fn inventory_access_like_cpp(
        &self,
    ) -> crate::session::OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    /// Borrow the existing session packet channel for ordered save-side effects.
    pub fn packet_publication_access_like_cpp(
        &self,
    ) -> crate::session::PacketPublicationAccessLikeCpp<'_> {
        self.core.packet_publication_access_like_cpp()
    }

    /// Whether this operation currently has no selected Player handle.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    /// Apply the existing transfer deferral to the selected canonical Player.
    /// `None` means the session has no matching owner; the inner `None` means
    /// the Player's deferred-save revision could not be advanced.
    pub fn defer_player_save_for_transfer_like_cpp(&mut self) -> Option<Option<bool>> {
        let handle = self.core.player_handle_like_cpp?;
        if self.core.player_guid() != Some(handle.guid()) {
            return None;
        }
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.defer_save_if_transfer_pending_like_cpp()
        })
    }

    /// Quarantine the session after an indeterminate save outcome.
    pub fn quarantine_like_cpp(&mut self, reason: &str) {
        self.core.kick(reason);
    }
}

fn player_save_header_like_cpp(
    player: &Player,
    residence: PlayerResidenceLikeCpp,
) -> PlayerSaveToDbSnapshotLikeCpp {
    let teleport = &player.gameplay_state().teleport;
    let destination = (teleport.recovery != wow_entities::PlayerTransferRecovery::Terminal)
        .then(|| {
            teleport
                .far_destination
                .map(|(map, position)| (u16::try_from(map).unwrap_or(u16::MAX), position))
                .or_else(|| {
                    teleport
                        .near_pending
                        .then_some(teleport.near_destination)
                        .flatten()
                })
        })
        .flatten();
    let (map_id, instance_id, position) = if let Some((map_id, position)) = destination {
        (map_id, 0, position)
    } else {
        // C++ Player.cpp:19480-19514 reads the Player's location. ResetMap
        // (Object.cpp:1814) retains map/instance even while detached.
        (
            player.unit().world().map_id() as u16,
            player.unit().world().instance_id(),
            player.unit().world().position(),
        )
    };
    let unit = player.unit();
    let max_health = unit.data().max_health.clamp(1, u64::from(u32::MAX)) as u32;
    let health = match residence {
        PlayerResidenceLikeCpp::Active(_) => {
            let health = unit.data().health.min(u64::from(u32::MAX)) as u32;
            if unit.is_alive() && health > 0 { health } else { 0 }
        }
        PlayerResidenceLikeCpp::Detached => unit.data().health.min(u64::from(max_health)) as u32,
    };
    PlayerSaveToDbSnapshotLikeCpp {
        guid: player.guid(),
        map_id,
        instance_id,
        position,
        level: unit.data().level as u8, // C++ Unit::GetLevel (Unit.h:733).
        xp: player.active_data().xp.max(0) as u32,
        money: player.money(),
        health,
        max_health,
        powers: loaded_character_power_snapshot_like_cpp(unit.data().power),
    }
}

fn request(
    core: &SessionCore,
    talent_store: Option<&wow_data::TalentStore>,
    spell_store: Option<&wow_data::SpellStore>,
    player: &Player,
    snapshot: &PlayerSaveToDbSnapshotLikeCpp,
    inputs: &PlayerSaveSessionInputsLikeCpp<'_>,
) -> Option<PlayerCharacterSaveRequestLikeCpp> {
    let game = player.gameplay_state();
    let mut player_flags = player.data().player_flags;
    let visible_resting = if game.rest.is_location_initialized_like_cpp() {
        if game.rest.is_resting_by_flag_like_cpp() {
            player_flags |= crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
        } else {
            player_flags &= !crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
        }
        game.rest.is_resting_by_flag_like_cpp()
    } else {
        player_flags & crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP != 0
    };
    let skills: std::collections::HashMap<_, _> = player
        .skill_records_like_cpp()
        .iter()
        .filter_map(crate::session::represented_player_skill_record_like_cpp)
        .map(|row| (row.skill_id, row))
        .collect();
    let guid_counter = snapshot.guid.counter() as u64;
    let powers = character_power_snapshot_values_like_cpp(&snapshot.powers);
    let (dungeon_difficulty, raid_difficulty, legacy_raid_difficulty) =
        player.difficulty_preferences_like_cpp();
    let character = PlayerCharacterSnapshotSaveLikeCpp {
        position: PlayerPositionSaveLikeCpp {
            x: snapshot.position.x,
            y: snapshot.position.y,
            z: snapshot.position.z,
            orientation: snapshot.position.orientation,
            map_id: snapshot.map_id,
            instance_id: snapshot.instance_id,
            zone_id: game.world_local.zone_id_like_cpp() as u16,
        },
        level: snapshot.level,
        xp: snapshot.xp,
        money: snapshot.money,
        rest_state: game.rest.rest_state_like_cpp(),
        player_flags: player_flags,
        rest_bonus: game.rest.rest_bonus_like_cpp(),
        logout_time: inputs.now_unix_secs.max(0) as u64,
        is_logout_resting: visible_resting,
        health: snapshot.health,
        powers,
        talent_reset_cost: game.talents.reset_talents_cost_like_cpp(),
        talent_reset_time: game.talents.reset_talents_time_secs_like_cpp(),
        explored_zones: wow_entities::explored_zones_db_string_from_blocks_like_cpp(
            player.explored_zones_blocks_like_cpp(),
        ),
        dungeon_difficulty,
        raid_difficulty,
        legacy_raid_difficulty,
    };

    let spell_runtime = Some(&game.spells);
    let spells = if let Some(spells) =
        (game.spells.rows_loaded_like_cpp() && game.spells.rows_complete_like_cpp()).then(|| {
            game.spells
                .rows_like_cpp()
                .iter()
                .map(|(&id, row)| {
                    (
                        id,
                        PlayerSpellSaveLikeCpp {
                            spell_id: row.spell_id,
                            active: row.active,
                            disabled: row.disabled,
                            dependent: row.dependent,
                            favorite: row.favorite,
                            state: match row.state {
                                PlayerSpellLoadState::Unchanged => PlayerSpellStateLikeCpp::Unchanged,
                                PlayerSpellLoadState::Changed => PlayerSpellStateLikeCpp::Changed,
                                PlayerSpellLoadState::New => PlayerSpellStateLikeCpp::New,
                                PlayerSpellLoadState::Removed => PlayerSpellStateLikeCpp::Removed,
                                PlayerSpellLoadState::Temporary => PlayerSpellStateLikeCpp::Temporary,
                            },
                        },
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        }) {
        Some(PlayerSpellSaveGroupLikeCpp::Complete {
            rows: spells.into_values().collect(),
            fallback_rows_were_present: spell_runtime
                .as_ref()
                .is_some_and(|runtime| !runtime.fallback_rows_like_cpp().is_empty()),
        })
    } else if spell_runtime
        .as_ref()
        .is_some_and(|runtime| !runtime.fallback_rows_like_cpp().is_empty())
    {
        Some(PlayerSpellSaveGroupLikeCpp::Fallback {
            rows: spell_runtime
                .as_ref()
                .expect("non-empty fallback spell runtime")
                .fallback_rows_like_cpp()
                .values()
                .map(|spell| PlayerFallbackSpellSaveLikeCpp {
                    spell_id: spell.spell_id,
                    active: spell.active,
                    dependent: spell.dependent,
                })
                .collect(),
        })
    } else {
        None
    };

    let skills = if player.skill_records_complete_like_cpp()
        && player
            .occupied_skill_slots_like_cpp()
            .is_some_and(|count| skills.len() == usize::from(count))
    {
        Some((player.non_durable_skill_tombstones_like_cpp(), &skills)).map(
            |(tombstones, records)| {
                records
                    .values()
                    .filter(|skill| {
                        skill.state != RepresentedPlayerSkillStateLikeCpp::Deleted
                            && !tombstones.contains(&skill.skill_id)
                    })
                    .map(|skill| PlayerSkillSaveLikeCpp {
                        skill_id: skill.skill_id,
                        value: skill.value,
                        max: skill.max,
                        profession_slot: skill.profession_slot,
                    })
                    .collect()
            },
        )
    } else {
        None
    };

    let talent_runtime = Some(&game.talents);
    let glyphs = if talent_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.glyphs_loaded_like_cpp())
    {
        Some(
            talent_runtime
                .as_ref()
                .expect("checked canonical glyph authority")
                .glyph_groups_like_cpp()
                .enumerate()
                .flat_map(|(talent_group, glyphs)| {
                    glyphs
                        .iter()
                        .copied()
                        .enumerate()
                        .map(move |(glyph_slot, glyph_id)| PlayerGlyphSaveLikeCpp {
                            talent_group: talent_group as u8,
                            glyph_slot: glyph_slot as u8,
                            glyph_id,
                        })
                })
                .collect(),
        )
    } else {
        None
    };

    let talents = if talent_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.talents_loaded_like_cpp())
    {
        let mut rows = Vec::new();
        for (talent_group, talents) in talent_runtime
            .as_ref()
            .expect("checked canonical talent authority")
            .talent_groups_like_cpp()
            .enumerate()
        {
            for (talent_id, rank) in talents {
                if represented_talent_for_save_like_cpp(
                    talent_store,
                    spell_store,
                    *talent_id,
                    *rank,
                )
                {
                    rows.push(PlayerTalentSaveLikeCpp {
                        talent_id: *talent_id,
                        rank: *rank,
                        talent_group: talent_group as u8,
                    });
                }
            }
        }
        Some(rows)
    } else {
        None
    };

    let spell_history = Some(&player.unit().subsystems().spells.history);
    let spell_cooldowns = if spell_history
        .as_ref()
        .is_some_and(|history| history.cooldowns_loaded)
    {
        Some(
            spell_history
                .as_ref()
                .expect("loaded history resolved above")
                .cooldowns
                .values()
                .map(|cooldown| PlayerSpellCooldownSaveLikeCpp {
                    spell_id: cooldown.spell_id,
                    item_id: cooldown.item_id,
                    cooldown_end_unix_secs: (cooldown.cooldown_end_ms / 1_000).min(i64::MAX as u64)
                        as i64,
                    category_id: cooldown.category_id,
                    category_end_unix_secs: (cooldown.category_end_ms / 1_000).min(i64::MAX as u64)
                        as i64,
                })
                .collect(),
        )
    } else {
        None
    };

    let spell_charges = if spell_history
        .as_ref()
        .is_some_and(|history| history.charges_loaded)
    {
        Some(
            spell_history
                .as_ref()
                .expect("loaded history resolved above")
                .charges
                .iter()
                .flat_map(|(&category_id, charges)| {
                    charges
                        .iter()
                        .map(move |charge| PlayerSpellChargeSaveLikeCpp {
                            category_id,
                            recharge_start_unix_secs: (charge.recharge_start_ms / 1_000)
                                .min(i64::MAX as u64)
                                as i64,
                            recharge_end_unix_secs: (charge.recharge_end_ms / 1_000)
                                .min(i64::MAX as u64)
                                as i64,
                        })
                })
                .collect(),
        )
    } else {
        None
    };

    let action_buttons = if let Some(action_buttons) = player
        .action_buttons_loaded_like_cpp()
        .then(|| player.action_buttons_snapshot_like_cpp())
    {
        let (spec, trait_config_id) = (game.talents.active_group_like_cpp(), 0);
        Some(PlayerActionButtonsSaveLikeCpp {
            spec,
            trait_config_id,
            rows: action_buttons
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(button, packed_action)| {
                    if packed_action == 0 {
                        return None;
                    }
                    Some(PlayerActionButtonSaveLikeCpp {
                        button: u8::try_from(button).ok()?,
                        packed_action,
                    })
                })
                .collect(),
        })
    } else {
        None
    };

    let equipment_sets = match (
        &game.equipment_sets,
        game.equipment_sets.is_loaded_like_cpp(),
    ) {
        (sets, true) => Some(
            sets.sets_like_cpp()
                .values()
                .map(|equipment_set| PlayerEquipmentSetSaveLikeCpp {
                    set_guid: equipment_set.guid,
                    set_id: equipment_set.set_id,
                    set_type: match equipment_set.set_type {
                        EntitiesPlayerEquipmentSetTypeLikeCpp::Equipment => {
                            PlayerEquipmentSetTypeLikeCpp::Equipment
                        }
                        EntitiesPlayerEquipmentSetTypeLikeCpp::Transmog => {
                            PlayerEquipmentSetTypeLikeCpp::Transmog
                        }
                    },
                    state: match equipment_set.state {
                        PlayerEquipmentSetUpdateStateLikeCpp::Unchanged => {
                            PlayerEquipmentSetStateLikeCpp::Unchanged
                        }
                        PlayerEquipmentSetUpdateStateLikeCpp::Changed => {
                            PlayerEquipmentSetStateLikeCpp::Changed
                        }
                        PlayerEquipmentSetUpdateStateLikeCpp::New => {
                            PlayerEquipmentSetStateLikeCpp::New
                        }
                        PlayerEquipmentSetUpdateStateLikeCpp::Deleted => {
                            PlayerEquipmentSetStateLikeCpp::Deleted
                        }
                    },
                    name: equipment_set.set_name.clone(),
                    icon: equipment_set.set_icon.clone(),
                    ignore_mask: equipment_set.ignore_mask,
                    assigned_spec_index: equipment_set.assigned_spec_index,
                    pieces: equipment_set
                        .pieces
                        .iter()
                        .map(|guid| guid.counter() as u64)
                        .collect(),
                    appearances: equipment_set.appearances.to_vec(),
                    enchants: equipment_set.enchants,
                })
                .collect(),
        ),
        _ => None,
    };

    let void_storage = match Some((&game.void_storage_items, game.void_storage_loaded)) {
        Some((items, true)) => Some(
            items
                .iter()
                .enumerate()
                .map(|(slot, item)| PlayerVoidStorageSlotSaveLikeCpp {
                    slot: u8::try_from(slot).expect("void-storage slot fits u8"),
                    item: item.as_ref().map(|item| PlayerVoidStorageSaveLikeCpp {
                        item_id: item.item_id,
                        item_entry: item.item_entry,
                        creator_guid: item.creator_guid.counter() as u64,
                        fixed_scaling_level: item.fixed_scaling_level,
                        random_properties_id: item.random_properties_id,
                        random_properties_seed: item.random_properties_seed,
                        context: item.context,
                    }),
                })
                .collect(),
        ),
        _ => None,
    };

    // C++ `_SaveQuestStatus` only consumes entries present in `m_QuestStatusSave`; it does
    // not rewrite every loaded quest during Player::SaveToDB. Rust's quest mutation paths
    // already persist their changed quest directly, but there is no coherent dirty-set seam
    // yet. Rewriting every active quest here can delete objective rows that were not mapped
    // into represented state, so preserve them until that dirty tracking exists.

    let tutorials = if *inputs.tutorials_changed {
        if *inputs.tutorials_loaded_coherently {
            Some(PlayerTutorialsSaveLikeCpp {
                tutorials: *inputs.tutorials,
                already_persisted: *inputs.tutorials_loaded_from_db,
            })
        } else {
            None
        }
    } else {
        None
    };

    let instance_lock_times = game
        .instance_reset_times
        .iter()
        .map(
            |(&instance_id, &release_time)| PlayerInstanceLockTimeSaveLikeCpp {
                instance_id,
                release_time,
            },
        )
        .collect();
    let session_secs: u32 = inputs
        .login_time
        .as_ref()
        .map(|time| time.elapsed().as_secs() as u32)
        .unwrap_or(0);
    let played_time = PlayerPlayedTimeSaveLikeCpp {
        total_time: inputs.total_played_time.saturating_add(session_secs),
        level_time: inputs.level_played_time.saturating_add(session_secs),
    };

    let reputations = wow_progression::ReputationMgrLikeCpp::borrowing_like_cpp(&game.reputation)
        .pending_save_rows_like_cpp()
        .into_iter()
        .map(
            |(faction_id, standing, flags)| PlayerReputationSaveLikeCpp {
                faction_id,
                standing,
                flags,
            },
        )
        .collect();

    let cuf_profiles = match Some((&game.cuf_profiles, game.cuf_profiles_loaded)) {
        Some((profiles, true)) => Some(
            (0..wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP)
                .map(|id| PlayerCufProfileSlotSaveLikeCpp {
                    profile_id: id as u8,
                    profile: profiles.get(id).and_then(Option::as_ref).map(|profile| {
                        PlayerCufProfileSaveLikeCpp {
                            profile_name: profile.profile_name.clone(),
                            frame_height: profile.frame_height,
                            frame_width: profile.frame_width,
                            sort_by: profile.sort_by,
                            health_text: profile.health_text,
                            bool_options: profile.bool_options,
                            top_point: profile.top_point,
                            bottom_point: profile.bottom_point,
                            left_point: profile.left_point,
                            top_offset: profile.top_offset,
                            bottom_offset: profile.bottom_offset,
                            left_offset: profile.left_offset,
                        }
                    }),
                })
                .collect(),
        ),
        _ => None,
    };

    Some(PlayerCharacterSaveRequestLikeCpp {
        player_guid: guid_counter,
        account_id: core.account_id,
        wall_clock_unix_secs: inputs.now_unix_secs,
        character,
        spells,
        skills,
        glyphs,
        talents,
        spell_cooldowns,
        spell_charges,
        action_buttons,
        equipment_sets,
        void_storage,
        tutorials,
        instance_lock_times,
        played_time,
        reputations,
        cuf_profiles,
    })
}

fn represented_talent_for_save_like_cpp(
    talent_store: Option<&wow_data::TalentStore>,
    spell_store: Option<&wow_data::SpellStore>,
    talent_id: u32,
    rank: u8,
) -> bool {
    let Some(talent) = talent_store.and_then(|store| store.get(talent_id)) else {
        return false;
    };
    let Some(spell_id) = talent.spell_rank.get(usize::from(rank)).copied() else {
        return false;
    };
    if spell_id <= 0 {
        return false;
    }
    let Some(spell_store) = spell_store else {
        return true;
    };
    wow_data::represented_spell_valid_with_seen_like_cpp(
        spell_store,
        spell_id,
        &mut HashSet::new(),
    )
}
