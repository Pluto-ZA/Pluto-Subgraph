use substreams_solana::pb::sf::solana::r#type::v1 as solana;
use crate::pb::sf::jupiter::v1::{BalanceChange, BalanceChanges};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, NaiveDateTime, Utc};

// Standard Wrapped SOL Mint address to represent Native SOL
const WRAPPED_SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

#[substreams::handlers::map]
pub fn map_balance_changes(params: String, block: solana::Block) -> Result<BalanceChanges, substreams::errors::Error> {
    let mut balance_changes = vec![];

    // 0. Parse Whitelist Params
    // Format: "addr1,addr2,addr3"
    // If empty, 'use_whitelist' becomes false, acting as a firehose (tracks everyone)
    let whitelist: HashSet<String> = params
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let use_whitelist = !whitelist.is_empty();

    let timestamp = block.block_time.as_ref().map(|t| t.timestamp).unwrap_or(0);

    // Create a readable date string
    let dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap_or_default(), Utc);
    let block_date = dt.format("%Y-%m-%d").to_string();
    let slot = block.slot;

    for trx in block.transactions {
        // Skip failed transactions
        if let Some(meta) = &trx.meta {
            if meta.err.is_some() {
                continue;
            }

            // Extract Transaction ID
            let tx_id = match &trx.transaction {
                Some(inner_tx) => {
                    if inner_tx.signatures.is_empty() {
                        continue;
                    }
                    bs58::encode(&inner_tx.signatures[0]).into_string()
                },
                None => continue,
            };

            // Get the list of accounts involved in this transaction
            let accounts = match &trx.transaction {
                Some(t) => &t.message.as_ref().unwrap().account_keys,
                None => continue,
            };

            // ---------------------------------------------------------
            // 1. HANDLE NATIVE SOL CHANGES
            // ---------------------------------------------------------
            if meta.pre_balances.len() == meta.post_balances.len() {
                for (i, pre_lamports) in meta.pre_balances.iter().enumerate() {
                    let post_lamports = meta.post_balances[i];

                    // Skip if no change (Optimization)
                    if *pre_lamports == post_lamports {
                        continue;
                    }

                    // Resolve Address
                    if i < accounts.len() {
                        let address = bs58::encode(&accounts[i]).into_string();

                        // --- WHITELIST CHECK ---
                        if use_whitelist && !whitelist.contains(&address) {
                            continue;
                        }

                        let pre_amt = *pre_lamports as f64 / LAMPORTS_PER_SOL;
                        let post_amt = post_lamports as f64 / LAMPORTS_PER_SOL;

                        // Filter dust
                        if (post_amt - pre_amt).abs() < f64::EPSILON {
                            continue;
                        }

                        balance_changes.push(BalanceChange {
                            block_date: block_date.clone(),
                            block_time: timestamp as u64,
                            block_slot: slot,
                            tx_id: tx_id.clone(),
                            owner: address,
                            mint: WRAPPED_SOL_MINT.to_string(),
                            change_amount: (post_amt - pre_amt).to_string(),
                            new_balance: post_amt.to_string(),
                            decimals: 9,
                        });
                    }
                }
            }

            // ---------------------------------------------------------
            // 2. HANDLE SPL TOKEN CHANGES
            // ---------------------------------------------------------

            // Map Pre-Balances for Tokens
            let mut pre_balances: HashMap<(u32, String), f64> = HashMap::new();

            for balance in &meta.pre_token_balances {
                if balance.owner.is_empty() { continue; }

                // Note: We don't filter Pre-Balances here because we need them
                // to calculate the delta for the Post-Balances later.

                let amount: f64 = balance.ui_token_amount.as_ref()
                    .map(|a| a.ui_amount)
                    .unwrap_or(0.0);

                pre_balances.insert((balance.account_index, balance.mint.clone()), amount);
            }

            for post_balance in &meta.post_token_balances {
                if post_balance.owner.is_empty() { continue; }

                // --- WHITELIST CHECK ---
                if use_whitelist && !whitelist.contains(&post_balance.owner) {
                    continue;
                }

                let account_idx = post_balance.account_index;
                let mint = post_balance.mint.clone();

                let post_amount: f64 = post_balance.ui_token_amount.as_ref()
                    .map(|a| a.ui_amount)
                    .unwrap_or(0.0);

                let decimals = post_balance.ui_token_amount.as_ref()
                    .map(|a| a.decimals)
                    .unwrap_or(0);

                let pre_amount = pre_balances.get(&(account_idx, mint.clone())).copied().unwrap_or(0.0);

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