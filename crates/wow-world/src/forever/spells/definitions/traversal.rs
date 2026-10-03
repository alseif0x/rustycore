//! Startup replay inputs for source container traversal, IDs only.
//! Helpers include unnamed keys because they affect std::unordered_map layout.
//! Server requests retain duplicates because source calls emplace for each row.
//! Both input vectors retire when global correction consumes the startup owner.
use super::{Definition, Key, NameSource, SpellDefinitionSeeds};
use std::collections::BTreeMap;

pub(super) struct OwnedInputs {
    helpers: Vec<Key>,
    server: Vec<Key>,
}
impl OwnedInputs {
    pub(super) fn new(helpers: Vec<Key>, server: Vec<Key>) -> Self {
        Self { helpers, server }
    }
}
pub struct SpellTraversalInputs<'a> {
    source: &'a OwnedInputs,
    definitions: &'a BTreeMap<Key, Definition>,
}
impl SpellTraversalInputs<'_> {
    pub fn helper_insertions(&self) -> &[(u32, i16)] {
        &self.source.helpers
    }
    pub fn server_requests(&self) -> &[(u32, i16)] {
        &self.source.server
    }
    pub fn client_keys(&self) -> impl Iterator<Item = (u32, i16)> + '_ {
        self.definitions.iter().filter_map(|(&key, definition)| {
            matches!(definition.name, NameSource::Client(_)).then_some(key)
        })
    }
}
impl SpellDefinitionSeeds {
    pub fn traversal_inputs(&self) -> Option<SpellTraversalInputs<'_>> {
        self.traversal_inputs
            .as_ref()
            .map(|source| SpellTraversalInputs {
                source,
                definitions: &self.definitions,
            })
    }
}
