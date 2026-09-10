//! Update-field regressions.
//!
//! Separated from update_fields.rs under #685.

use super::*;

fn descriptor(kind: UpdateFieldSectionKind, name: &str) -> Option<&'static UpdateFieldDescriptor> {
    kind.descriptors()
        .iter()
        .find(|descriptor| descriptor.name == name)
}

mod scenarios;
