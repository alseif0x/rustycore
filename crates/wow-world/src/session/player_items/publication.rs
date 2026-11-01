//! Values updates and packets published for represented item state.
//!
//! Moved out of the Session root under #597. The new-item push result and its
//! group broadcast moved to `wow_world_inventory::send_new_item_plan` (#1363
//! slice 12b7); the file stays mounted because a source-contract scenario
//! reads it.
