// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Item template and use admission for Player.

use super::super::super::*;

impl Player {
    pub fn can_use_item_template(&self, args: CanUseItemTemplateArgs<'_>) -> InventoryResult {
        if args.proto.is_none() {
            return InventoryResult::ItemNotFound;
        }

        if args.internal_item {
            return InventoryResult::CantEquipEver;
        }

        if args.faction_horde && args.team != TEAM_HORDE_ID {
            return InventoryResult::CantEquipEver;
        }

        if args.faction_alliance && args.team != TEAM_ALLIANCE_ID {
            return InventoryResult::CantEquipEver;
        }

        if !args.allowable_class_matches || !args.allowable_race_matches {
            return InventoryResult::CantEquipEver;
        }

        if args.required_skill != 0 {
            if args.required_skill_value == 0 {
                return InventoryResult::ProficiencyNeeded;
            }

            if args.required_skill_value < args.required_skill_rank {
                return InventoryResult::CantEquipSkill;
            }
        }

        if args.required_spell != 0 && !args.has_required_spell {
            return InventoryResult::ProficiencyNeeded;
        }

        if !args.skip_required_level_check && args.player_level < args.base_required_level {
            return InventoryResult::CantEquipLevelI;
        }

        if args.holiday_id != 0 && !args.holiday_active {
            return InventoryResult::ClientLockedOut;
        }

        if args.required_reputation_faction != 0
            && args.player_reputation_rank < args.required_reputation_rank
        {
            return InventoryResult::CantEquipReputation;
        }

        if matches!(args.effect0_spell_id, Some(483 | 55_884))
            && args.effect1_spell_id.is_some()
            && args.has_effect1_spell
        {
            return InventoryResult::InternalBagError;
        }

        if args
            .artifact_specialization
            .is_some_and(|spec| spec != args.primary_specialization)
        {
            return InventoryResult::CantUseItem;
        }

        InventoryResult::Ok
    }

    pub fn can_use_item(&self, mut args: CanUseItemArgs<'_>) -> InventoryResult {
        let Some(source) = args.source_item else {
            return InventoryResult::ItemNotFound;
        };

        if !args.is_alive && args.not_loading {
            return InventoryResult::PlayerDead;
        }

        let Some(proto) = args.proto else {
            return InventoryResult::ItemNotFound;
        };

        if source.is_binded_not_with(self.guid(), proto, args.source_bop_trade_allowed_for_player) {
            return InventoryResult::NotOwner;
        }

        if args.player_level < args.item_required_level {
            return InventoryResult::CantEquipLevelI;
        }

        args.template_args.proto = args.proto;
        args.template_args.skip_required_level_check = true;
        let template_result = self.can_use_item_template(args.template_args);
        if template_result != InventoryResult::Ok {
            return template_result;
        }

        if args.item_skill != 0 {
            let allow_equip = args.proto_is_heirloom
                && proto.class_id == ItemClass::Armor
                && !args.has_item_skill
                && match args.player_class {
                    CLASS_HUNTER | CLASS_SHAMAN => args.item_skill == SKILL_MAIL,
                    CLASS_PALADIN | CLASS_WARRIOR => args.item_skill == SKILL_PLATE_MAIL,
                    _ => false,
                };

            if !allow_equip && args.item_skill_value == 0 {
                return InventoryResult::ProficiencyNeeded;
            }
        }

        InventoryResult::Ok
    }
}
