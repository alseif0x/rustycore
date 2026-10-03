//! Narrow pre-publication raw-record mutation; rules belong to the domain caller.
//! No insertion/removal, DB write, clone or hotfix-notification metadata.
use super::SpellCatalog;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy)]
pub struct SummonPropertiesPatch {
    pub id: u32,
    pub title: Option<i32>,
    pub control: Option<i32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SummonPropertiesPatchCounts {
    pub applied: usize,
    pub missing: usize,
}
impl SpellCatalog {
    /// Requires exclusive mutable ownership; an Arc reader cannot invoke this.
    /// DB2HotfixGenerator skips missing rows without resurrecting removed IDs.
    /// This method does not insert client hotfix metadata (notifyClient=false).
    pub fn apply_summon_properties_patches(
        &mut self,
        patches: impl IntoIterator<Item = SummonPropertiesPatch>,
    ) -> SummonPropertiesPatchCounts {
        let mut counts = SummonPropertiesPatchCounts::default();
        for patch in patches {
            if let Some(row) = self.summon_properties.get_mut(&patch.id) {
                if let Some(title) = patch.title {
                    row.title = title;
                }
                if let Some(control) = patch.control {
                    row.control = control;
                }
                counts.applied += 1;
            } else {
                counts.missing += 1;
            }
        }
        counts
    }
}
