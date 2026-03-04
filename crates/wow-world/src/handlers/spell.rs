// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cast handlers — CMSG_CAST_SPELL, CMSG_CANCEL_CAST, CMSG_CANCEL_CHANNELLING.
//!
//! Phase 2 ("efectos mecánicos"): 
//!   1. Parse CMSG_CAST_SPELL.
//!   2. Validate known spell + cooldown.
//!   3. If cast_time > 0: send SMSG_SPELL_START, store active_spell_cast, wait.
//!   4. When cast completes: execute_spell() → apply effects + cooldown.
//!   5. If instant: execute immediately.
//!
//! Future phases will add:
//!   - Movement cancellation (CMSG_MOVE_* while casting)
//!   - Channelling spells (tick-based damage)
//!   - Interrupts & silences
//!
//! Reference: C# Game/Handlers/SpellHandler.cs, Game/Spells/Spell.cs

use tracing::{debug, info, warn};

use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::spell::{
    CastFailed, CastSpellRequest, SpellCastVisual, SpellStartPkt, SpellTargetData,
};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::WorldSession;

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CastSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_cast_spell",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelCast,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_cast",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelChannelling,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_channelling",
    }
}

// ── Handler implementations ───────────────────────────────────────

impl WorldSession {
    /// Handle `CMSG_CAST_SPELL` (0x329C).
    ///
    /// Phase 2: cast timers + cooldowns + mechanical effects.
    /// 
    /// Flow:
    /// 1. Validate spell is known.
    /// 2. Validate cooldown.
    /// 3. If cast_time > 0: initiate cast (SMSG_SPELL_START), wait for tick_active_spell_cast().
    /// 4. If instant: execute immediately.
    pub async fn handle_cast_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        let player_guid = match self.player_guid {
            Some(g) => g,
            None => {
                warn!("handle_cast_spell: no player_guid");
                return;
            }
        };

        let req = match CastSpellRequest::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_CAST_SPELL: {e}"
                );
                return;
            }
        };

        let spell_id = req.spell_id;
        let cast_id = req.cast_id;

        debug!(
            account = self.account_id,
            spell_id = spell_id,
            cast_id = ?cast_id,
            target = ?req.target.unit,
            "CMSG_CAST_SPELL"
        );

        // ── Validation: Known spell ─────────────────────────────────────
        if !self.known_spells.contains(&spell_id) {
            warn!(
                account = self.account_id,
                spell_id = spell_id,
                "Cast attempt for unknown spell"
            );
            self.send_packet(&CastFailed {
                cast_id,
                spell_id,
                reason: 2,    // SpellCastResult::NotKnown
                fail_arg1: 0,
                fail_arg2: 0,
            });
            return;
        }

        // ── Get spell info ──────────────────────────────────────────────
        let spell_info: &wow_data::SpellInfo = match &self.spell_store {
            Some(store) => match store.get(spell_id) {
                Some(info) => info,
                None => {
                    warn!(
                        account = self.account_id,
                        spell_id = spell_id,
                        "Spell not found in store"
                    );
                    self.send_packet(&CastFailed {
                        cast_id,
                        spell_id,
                        reason: 2,
                        fail_arg1: 0,
                        fail_arg2: 0,
                    });
                    return;
                }
            },
            None => {
                warn!(account = self.account_id, "No spell store available");
                self.send_packet(&CastFailed {
                    cast_id,
                    spell_id,
                    reason: 2,
                    fail_arg1: 0,
                    fail_arg2: 0,
                });
                return;
            }
        };

        // ── Validation: Cooldown ────────────────────────────────────────
        // Check global cooldown (GCD)
        if let Some(last_cast) = self.last_spell_cast_time {
            let cooldown_ms = spell_info.effective_cooldown_ms();
            let elapsed_ms = last_cast.elapsed().as_millis() as u32;
            
            if elapsed_ms < cooldown_ms {
                debug!(
                    account = self.account_id,
                    spell_id = spell_id,
                    remaining_ms = cooldown_ms - elapsed_ms,
                    "Spell on global cooldown"
                );
                self.send_packet(&CastFailed {
                    cast_id,
                    spell_id,
                    reason: 10,   // SpellCastResult::NotReady
                    fail_arg1: 0,
                    fail_arg2: 0,
                });
                return;
            }
        }
        
        // Check per-spell cooldown
        if spell_info.recovery_time_ms > 0 {
            if let Some(last_spell_cast) = self.last_spell_cast_time_per_spell.get(&spell_id) {
                let elapsed_ms = last_spell_cast.elapsed().as_millis() as u32;
                let cooldown_ms = spell_info.recovery_time_ms;
                
                if elapsed_ms < cooldown_ms {
                    debug!(
                        account = self.account_id,
                        spell_id = spell_id,
                        remaining_ms = cooldown_ms - elapsed_ms,
                        "Spell on per-spell cooldown"
                    );
                    self.send_packet(&CastFailed {
                        cast_id,
                        spell_id,
                        reason: 10,   // SpellCastResult::NotReady
                        fail_arg1: 0,
                        fail_arg2: 0,
                    });
                    return;
                }
            }
        }

        // ── Build target ────────────────────────────────────────────────
        let (target_flags, target_guid) = if !req.target.unit.is_empty() {
            (0x2u32, req.target.unit)  // SpellCastTargetFlags::Unit
        } else {
            (0x2u32, player_guid)      // self-target
        };

        let spell_target = SpellTargetData {
            flags: target_flags,
            unit: target_guid,
            item: wow_core::ObjectGuid::EMPTY,
        };

        // ── Initiate cast or execute immediately ─────────────────────────
        if spell_info.has_cast_time() {
            // Cast with delay — send SMSG_SPELL_START and store state
            debug!(
                account = self.account_id,
                spell_id = spell_id,
                cast_time_ms = spell_info.cast_time_ms,
                "Starting cast with timer"
            );

            let start_pkt = SpellStartPkt {
                caster: player_guid,
                cast_id,
                spell_id,
                visual: SpellCastVisual {
                    spell_visual_id: req.visual.spell_visual_id,
                    script_visual_id: 0,
                },
                target: spell_target,
                cast_time_ms: spell_info.cast_time_ms,
            };
            self.send_packet(&start_pkt);

            // Store active cast state
            self.active_spell_cast = Some(crate::session::SpellCastState {
                spell_id,
                target_guid,
                cast_id,
                cast_start_time: std::time::Instant::now(),
                cast_time_ms: spell_info.cast_time_ms,
                spell_visual: SpellCastVisual {
                    spell_visual_id: req.visual.spell_visual_id,
                    script_visual_id: 0,
                },
            });

            info!(
                account = self.account_id,
                spell_id = spell_id,
                "Cast initiated ({}ms cast time)",
                spell_info.cast_time_ms
            );
        } else {
            // Instant cast — execute immediately
            debug!(
                account = self.account_id,
                spell_id = spell_id,
                "Instant cast, executing immediately"
            );
            if let Err(e) = self.execute_spell(spell_id, target_guid).await {
                warn!(account = self.account_id, "Instant spell execution failed: {}", e);
                return;
            }

            info!(
                account = self.account_id,
                spell_id = spell_id,
                "Instant spell executed"
            );
        }
    }

    /// Handle `CMSG_CANCEL_CAST` — player cancels an in-progress cast.
    pub async fn handle_cancel_cast(&mut self, _pkt: wow_packet::WorldPacket) {
        // TODO: Phase 3 — implement cancel cast
    }

    /// Handle `CMSG_CANCEL_CHANNELLING` — player stops a channelled spell.
    pub async fn handle_cancel_channelling(&mut self, _pkt: wow_packet::WorldPacket) {
        // TODO: Phase 3 — implement cancel channelling
    }
}
