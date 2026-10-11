// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `query` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) async fn handle_item_text_query(&mut self, query: ItemTextQuery) {
        self.build_item_text_query_handler_cx_like_cpp()
            .handle_item_text_query(query);
    }
}
