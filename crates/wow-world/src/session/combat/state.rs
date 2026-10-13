//! Represented combat state and PvP flags at the Session boundary.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_combat_ratings_game_table(&mut self, table: Arc<CombatRatingsGameTableLikeCpp>) {
        self.catalogs.combat_ratings_game_table = Some(table);
    }

    pub fn set_regen_game_tables(&mut self, tables: Arc<RegenGameTablesLikeCpp>) {
        self.catalogs.regen_game_tables = Some(tables);
    }

    pub fn set_pvp_realm_like_cpp(&mut self, is_pvp_realm: bool) {
        self.view.is_pvp_realm_like_cpp = is_pvp_realm;
    }
    pub fn set_ffa_pvp_realm_like_cpp(&mut self, is_ffa_pvp_realm: bool) {
        self.view.is_ffa_pvp_realm_like_cpp = is_ffa_pvp_realm;
    }
    #[cfg(test)]
    pub(crate) fn represented_combat_stat_recalculations_like_cpp(
        &self,
    ) -> &[RepresentedCombatStatRecalculationLikeCpp] {
        self.inventory
            .represented_combat_stat_recalculations_for_test_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/combat/state/f3_shims.rs"]
mod f3_shims;
