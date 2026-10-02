// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;

impl crate::session::HubMut<'_> {
    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_confirm_barbers_choice_like_cpp(
        &mut self,
        request: RepresentedConfirmBarbersChoiceLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures
                .presentation
                .represented_confirm_barbers_choice_requests_like_cpp
                .push(request);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedAlterAppearanceLikeCpp {
    pub new_sex: u8,
    pub customizations: Vec<wow_packet::packets::character::ChrCustomizationChoice>,
    pub customized_race: i32,
    pub customized_chr_model_id: i32,
    pub cost: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedConfirmBarbersChoiceLikeCpp {
    pub customizations: Vec<wow_packet::packets::character::ChrCustomizationChoice>,
    pub cost: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedConfirmRespecWipeLikeCpp {
    pub respec_master: ObjectGuid,
    pub respec_type: u8,
}

/// Evidence for C++ `sScriptMgr->OnPlayerTalentsReset(this, noCost)`.
#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedTalentResetScriptHookLikeCpp {
    pub no_cost: bool,
}

/// Evidence for `unit->CastSpell(_player, 14867, true)` after talent reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedTalentRespecVisualSpellCastLikeCpp {
    pub caster_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub spell_id: u32,
    pub triggered: bool,
    pub spell_runtime_unrepresented: bool,
}

/// Evidence for the two C++ `Player::ResetTalents` achievement criteria updates.
#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedTalentRespecCriteriaEventLikeCpp {
    MoneySpentOnRespecs { amount: u32 },
    TotalRespecs { quantity: u32 },
}
