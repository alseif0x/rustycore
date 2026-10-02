use crate::session::state::SessionCatalogs;
use std::collections::BTreeSet;

impl SessionCatalogs {
    pub fn same_effect_stack_rule_aura_types_like_cpp(
        &self,
        group_id: u32,
    ) -> Option<&BTreeSet<i32>> {
        self.spell_catalogs
            .spell_group_stack_rule_store
            .as_ref()
            .and_then(|store| store.same_effect_stack_rule_aura_types_like_cpp(group_id))
    }
}
