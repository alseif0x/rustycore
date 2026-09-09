//! Body analyzer state and its direct-child flow collectors.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

pub(super) struct BodyAnalyzer<'a, 'b> {
    pub(super) context: RecordContext<'a>,
    pub(super) accumulator: &'b mut AccessAccumulator,
    pub(super) errors: &'b mut Vec<String>,
    pub(super) symbols: &'b ModuleSymbols,
    pub(super) enclosing: String,
    pub(super) visibility: String,
    pub(super) cfg: Vec<String>,
    pub(super) scopes: Vec<BTreeMap<String, VariableInfo>>,
    pub(super) local_path_alias_scopes: Vec<BTreeMap<String, Vec<String>>>,
    pub(super) anonymous_trait_scopes: Vec<BTreeSet<String>>,
    /// Trait of the impl whose body this is, canonicalised exactly as the
    /// registration keys it. `Self::CONST` in a trait impl names
    /// `<Owner as Trait>::CONST`, which the inherent key does not reach (#204).
    pub(super) active_trait: Option<String>,
    pub(super) generic_trait_bounds: BTreeMap<String, BTreeSet<String>>,
    pub(super) generic_trait_bound_args: BTreeMap<(String, String), Vec<VariableInfo>>,
    pub(super) generic_trait_bound_associated:
        BTreeMap<(String, String), BTreeMap<String, VariableInfo>>,
    pub(super) flow_cache: std::cell::RefCell<BTreeMap<(usize, u64), Flow>>,
    pub(super) subtree_flow_cache: std::cell::RefCell<BTreeMap<(usize, u64), Flow>>,
    pub(super) closure_effects: std::cell::RefCell<BTreeMap<usize, BTreeMap<String, VariableInfo>>>,
    pub(super) closure_result_infos: std::cell::RefCell<BTreeMap<usize, VariableInfo>>,
    pub(super) block_result_infos: std::cell::RefCell<BTreeMap<usize, VariableInfo>>,
    pub(super) replacement_result_infos: std::cell::RefCell<BTreeMap<usize, VariableInfo>>,
    pub(super) loop_flow_collectors: Vec<LoopFlowCollector>,
    pub(super) block_exit_collectors: Vec<BlockExitCollector>,
    pub(super) return_exit_collectors: Vec<Option<Vec<BTreeMap<String, VariableInfo>>>>,
    pub(super) return_value_collectors: Vec<VariableInfo>,
    pub(super) context_version: u64,
    pub(super) suppress_records: bool,
    /// Macro shadows declared above the item whose body this analyzes.
    pub(super) visible_macro_shadows: BTreeSet<String>,
}

#[derive(Default)]
pub(super) struct LoopFlowCollector {
    pub(super) label: Option<String>,
    pub(super) exits: Option<Vec<BTreeMap<String, VariableInfo>>>,
    pub(super) back_edges: Option<Vec<BTreeMap<String, VariableInfo>>>,
}

#[derive(Default)]
pub(super) struct BlockExitCollector {
    pub(super) label: String,
    pub(super) exits: Option<Vec<BTreeMap<String, VariableInfo>>>,
    pub(super) result: VariableInfo,
}

pub(super) struct DirectChildFlowCollector<'analyzer, 'a, 'b> {
    pub(super) analyzer: &'analyzer BodyAnalyzer<'a, 'b>,
    pub(super) flow: Flow,
    pub(super) at_root: bool,
}

impl<'ast> Visit<'ast> for DirectChildFlowCollector<'_, '_, '_> {
    fn visit_expr(&mut self, expression: &'ast Expr) {
        if self.at_root {
            self.at_root = false;
            visit::visit_expr(self, expression);
        } else {
            self.flow.union(self.analyzer.subtree_flow(expression));
        }
    }
}
