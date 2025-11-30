use substreams_solana::pb::sf::solana::r#type::v1 as solana;
use crate::pb::sf::jupiter::v1::{BalanceChange, BalanceChanges};
use std::collections::HashMap;
use chrono::{DateTime, NaiveDateTime, Utc};
#[substreams::handlers::map]
pub fn map_balance_changes(block: solana::Block) -> Result<BalanceChanges, substreams::errors::Error> {
    let mut balance_changes = vec![];

    let timestamp = block.block_time.as_ref().map(|t| t.timestamp).unwrap_or(0);

    // Create a readable date string (YYYY-MM-DD)
    let dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap_or_default(), Utc);
    let block_date = dt.format("%Y-%m-%d").to_string();

    let slot = block.slot;

    for trx in block.transactions {
        // Skip failed transactions
        if let Some(meta) = &trx.meta {
            if meta.err.is_some() {
                continue;
            }

            let tx_id = match &trx.transaction {
                Some(inner_tx) => {
                    if inner_tx.signatures.is_empty() {
                        continue;
                    }
                    bs58::encode(&inner_tx.signatures[0]).into_string()
                },
                None => continue,
            };

            // 1. Map Pre-Balances: Key = (Account Index, Mint) -> Amount
            // We use account index because it's consistent within the tx meta
            let mut pre_balances: HashMap<(u32, String), f64> = HashMap::new();

            for balance in &meta.pre_token_balances {
                // Ensure we have an owner to attribute this to
                if balance.owner.is_empty() { continue; }

                let amount: f64 = balance.ui_token_amount.as_ref()
                    .map(|a| a.ui_amount)
                    .unwrap_or(0.0);

                pre_balances.insert((balance.account_index, balance.mint.clone()), amount);
            }

            // 2. Iterate Post-Balances and compare
            for post_balance in &meta.post_token_balances {
                if post_balance.owner.is_empty() { continue; }

                let account_idx = post_balance.account_index;
                let mint = post_balance.mint.clone();

                let post_amount: f64 = post_balance.ui_token_amount.as_ref()
                    .map(|a| a.ui_amount)
                    .unwrap_or(0.0);

                let decimals = post_balance.ui_token_amount.as_ref()
                    .map(|a| a.decimals)
                    .unwrap_or(0);

                // Get the pre-balance (default to 0 if this is a new account/mint interaction)
                let pre_amount = pre_balances.get(&(account_idx, mint.clone())).copied().unwrap_or(0.0);

                // 3. If balance changed, record it
                if (post_amount - pre_amount).abs() > f64::EPSILON {
                    balance_changes.push(BalanceChange {
                        block_date: block_date.clone(),
                        block_time:  timestamp as u64,
                        block_slot: slot,
                        tx_id: tx_id.clone(),
                        owner: post_balance.owner.clone(),
                        mint: mint,
                        change_amount: (post_amount - pre_amount).to_string(),
                        new_balance: post_amount.to_string(),
                        decimals,
                    });
                }
            }
        }
    }

    Ok(BalanceChanges { params: balance_changes })
}