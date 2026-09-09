//! Loot-race gameobject operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

/// Validate the two serialized C++ `HandleLootMoneyOpcode` observations.
///
/// Both clients send `CMSG_LOOT_MONEY`. For `LOOT_CHEST`, C++ deliberately
/// keeps `shareMoney=false`: the first serialized requester receives the whole
/// pool with `SoleLooter=true`, then `Loot::LootMoney()` sets gold to zero. The
/// second requester receives one zero notification. Each request still calls
/// `Loot::NotifyMoneyRemoved`, so both active viewers observe two removals.
pub(crate) fn validate_serialized_gameobject_money_wire_outcome_like_cpp(
    evidence: &[WireEvidence; 2],
    expected_source: (u64, u64),
    expected_pool: u64,
) -> Result<usize> {
    if expected_pool == 0 {
        bail!("loot-race cannot prove a positive serialized money winner");
    }

    let positive = MoneyNotify {
        money: expected_pool,
        money_mod: 0,
        sole_looter: true,
    };
    let zero = MoneyNotify {
        money: 0,
        money_mod: 0,
        sole_looter: true,
    };

    let mut winner = None;
    for (participant, entry) in evidence.iter().enumerate() {
        match entry.money_notifies.as_slice() {
            [notify] if *notify == positive => {
                if winner.replace(participant).is_some() {
                    bail!("loot-race observed more than one positive chest-money winner");
                }
            }
            [notify] if *notify == zero => {}
            _ => {
                bail!(
                    "loot-race participant {participant} observed money notifications {:?}; C++ LOOT_CHEST requires one requester-local whole-pool {expected_pool} or serialized zero notification, both SoleLooter=true",
                entry.money_notifies
            );
            }
        }

        let matching_removals = entry
            .coin_removed
            .iter()
            .filter(|source| **source == expected_source)
            .count();
        if entry.coin_removed.len() != 2 || matching_removals != 2 {
            bail!(
                "loot-race participant {participant} observed CoinRemoved sources {:?}; C++ calls NotifyMoneyRemoved once for each of the two serialized requests",
                entry.coin_removed
            );
        }
    }

    winner.ok_or_else(|| anyhow!("loot-race emitted no positive chest-money winner"))
}
