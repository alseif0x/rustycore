use super::SpellStore;

impl SpellStore {
    pub fn install_effective_cast_costs_like_cpp(
        &mut self,
        costs: super::SpellCastCostStoreLikeCpp,
    ) {
        self.cast_costs_like_cpp = Some(costs);
    }

    pub fn effective_cast_costs_like_cpp(&self) -> Option<&super::SpellCastCostStoreLikeCpp> {
        self.cast_costs_like_cpp.as_ref()
    }
}
