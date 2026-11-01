// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The account item-appearance and transmog-illusion login loads (#1363
//! slice 13c).
//!
//! C++ `CollectionMgr::LoadAccountItemAppearances` and
//! `CollectionMgr::LoadAccountTransmogIllusions`, run by the continue-login
//! coordinator. Moved from the World `handlers/character/account/collections.rs`
//! verbatim except for the mechanical `self` -> `host` rewrite: the lifecycle
//! port is read through the battle-pet host's `lifecycle`, and the
//! two one-line `WorldSession` inventory forwards are inlined as their bodies.

use std::sync::Arc;

use tracing::warn;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp,
};

use crate::VendorSessionHostLikeCpp;

pub async fn load_account_item_appearances_like_cpp<H: VendorSessionHostLikeCpp>(host: &mut H) {
    let Some(port) = host
        .lifecycle()
        .player_lifecycle_port_like_cpp()
        .map(Arc::clone)
    else {
        let (state, mut hub) = host.split_inventory_mut();
        state.load_represented_account_item_appearances_like_cpp(&mut hub, [], []);
        return;
    };

    let bnet_account_id = host.core().battlenet_account_id();
    let (appearance_blocks, favorite_appearances) = match port
        .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::ItemAppearances {
            bnet_account_id,
        })
        .await
    {
        AccountCollectionLoadOutcomeLikeCpp::Loaded(
            AccountCollectionLoadedLikeCpp::ItemAppearances {
                appearance_blocks,
                favorite_appearance_ids,
            },
        ) => {
            let appearance_blocks = match appearance_blocks {
                AccountCollectionRowsLikeCpp::Loaded(rows) => rows
                    .into_iter()
                    .map(|row| (row.block_index, row.mask))
                    .collect(),
                AccountCollectionRowsLikeCpp::Failed { reason } => {
                    warn!(
                        account = host.core().account_id,
                        bnet_account = bnet_account_id,
                        "Failed to load account item appearances: {reason}"
                    );
                    Vec::new()
                }
            };
            let favorite_appearances = match favorite_appearance_ids {
                AccountCollectionRowsLikeCpp::Loaded(rows) => rows,
                AccountCollectionRowsLikeCpp::Failed { reason } => {
                    warn!(
                        account = host.core().account_id,
                        bnet_account = bnet_account_id,
                        "Failed to load account favorite item appearances: {reason}"
                    );
                    Vec::new()
                }
            };
            (appearance_blocks, favorite_appearances)
        }
        AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                bnet_account = bnet_account_id,
                "Failed to load account item appearances: {reason}"
            );
            (Vec::new(), Vec::new())
        }
        AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
            warn!(
                account = host.core().account_id,
                bnet_account = bnet_account_id,
                "Player lifecycle port returned the wrong account collection for item appearances"
            );
            (Vec::new(), Vec::new())
        }
    };

    let (state, mut hub) = host.split_inventory_mut();
    state.load_represented_account_item_appearances_like_cpp(
        &mut hub,
        appearance_blocks,
        favorite_appearances,
    );
}

pub async fn load_account_transmog_illusions_like_cpp<H: VendorSessionHostLikeCpp>(host: &mut H) {
    let Some(port) = host
        .lifecycle()
        .player_lifecycle_port_like_cpp()
        .map(Arc::clone)
    else {
        let (state, mut hub) = host.split_inventory_mut();
        state.load_represented_account_transmog_illusions_like_cpp(&mut hub, []);
        return;
    };

    let bnet_account_id = host.core().battlenet_account_id();
    let illusion_blocks = match port
        .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::TransmogIllusions {
            bnet_account_id,
        })
        .await
    {
        AccountCollectionLoadOutcomeLikeCpp::Loaded(
            AccountCollectionLoadedLikeCpp::TransmogIllusions { illusion_blocks },
        ) => illusion_blocks
            .into_iter()
            .map(|row| (row.block_index, row.mask))
            .collect(),
        AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                bnet_account = bnet_account_id,
                "Failed to load account transmog illusions: {reason}"
            );
            Vec::new()
        }
        AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
            warn!(
                account = host.core().account_id,
                bnet_account = bnet_account_id,
                "Player lifecycle port returned the wrong account collection for transmog illusions"
            );
            Vec::new()
        }
    };

    let (state, mut hub) = host.split_inventory_mut();
    state.load_represented_account_transmog_illusions_like_cpp(&mut hub, illusion_blocks);
}
