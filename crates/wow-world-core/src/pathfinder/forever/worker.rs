// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: selected Forever navigation package consumer.
//! Operations routed through the pathfinder's existing dedicated worker.

use super::super::WorldMMapPathfinderLikeCpp;
use super::ForeverNavigationOwner;
use super::types::{
    ForeverNavigationError, ForeverNavigationWorkerAction, ForeverNavigationWorkerResponse,
};

pub(in crate::pathfinder) fn handle_action(
    pathfinder: &mut WorldMMapPathfinderLikeCpp,
    action: ForeverNavigationWorkerAction,
) -> Result<ForeverNavigationWorkerResponse, ForeverNavigationError> {
    let owner: &mut ForeverNavigationOwner = pathfinder
        .forever_navigation
        .as_mut()
        .ok_or(ForeverNavigationError::NotConfigured)?;
    owner.handle_action(&mut pathfinder.mmap_manager, action)
}
