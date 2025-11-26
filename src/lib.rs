pub mod constants;
pub mod pb;
pub mod spl_account_store;
pub mod jupiter_trading_store;
pub mod token_price_store;
pub mod jupiter_instructions;
pub mod jupiter_analytics;
pub mod transactions;
pub mod stats_store;
pub mod portfolio_store;
pub mod analytics;
pub mod global_prices;

pub use spl_account_store::map_spl_initialized_account;
pub use jupiter_trading_store::map_jupiter_trading_data;
pub use token_price_store::store_token_prices;
pub use jupiter_instructions::map_jupiter_instructions;
pub use jupiter_analytics::map_jupiter_analytics;
pub use transactions::map_relevant_transactions;
pub use global_prices::map_global_token_prices;

use std::collections::{HashMap, HashSet};
use substreams_solana::pb::sf::solana::r#type::v1::ConfirmedTransaction;
use crate::pb::sf::jupiter::v1::BalanceChange;

// Helper: Parse comma-separated params into a HashSet
pub fn parse_filters(params: &str) -> HashSet<String> {
    params
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn is_relevant_tx(tx: &ConfirmedTransaction, targets: &HashSet<String>) -> bool {
    // If no filters are provided, return true (process everything)
    // OR return false (process nothing) depending on your requirement.
    // Assuming if params are empty = process everything:
    if targets.is_empty() {
        return true;
    }

    if let Some(transaction) = &tx.transaction {
        if let Some(message) = &transaction.message {
            // Check all accounts (program_ids, signers, writable, readonly)
            for account_key in &message.account_keys {
                let address_str = bs58::encode(account_key).into_string();
                if targets.contains(&address_str) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn calculate_balance_changes(trx: &ConfirmedTransaction) -> Vec<BalanceChange> {
    let mut changes = Vec::new();

    let meta = match &trx.meta {
        Some(m) => m,
        None => return changes,
    };

    // Helper to resolve accounts (Static + Lookup Tables)
    let mut all_accounts = Vec::new();
    if let Some(transaction) = &trx.transaction {
        if let Some(message) = &transaction.message {
            for key in &message.account_keys {
                all_accounts.push(bs58::encode(key).into_string());
            }
            // Add Loaded Addresses (Lookup Tables)
            if let Some(meta) = &trx.meta {
                for key in &meta.loaded_writable_addresses { all_accounts.push(bs58::encode(key).into_string()); }
                for key in &meta.loaded_readonly_addresses { all_accounts.push(bs58::encode(key).into_string()); }
            }
        }
    }

    // 1. SOL Changes
    for (i, address) in all_accounts.iter().enumerate() {
        let pre = *meta.pre_balances.get(i).unwrap_or(&0);
        let post = *meta.post_balances.get(i).unwrap_or(&0);
        if pre != post {
            changes.push(BalanceChange {
                address: address.clone(),
                mint: "".to_string(), // Empty = SOL
                change: (post as f64 - pre as f64) / 1_000_000_000.0,
                post_balance: post as f64 / 1_000_000_000.0,
            });
        }
    }

    // 2. Token Changes
    let mut pre_token_map: HashMap<(u32, String), f64> = HashMap::new();
    for b in &meta.pre_token_balances {
        let amount = b.ui_token_amount.as_ref().map(|x| x.ui_amount).unwrap_or(0.0);
        pre_token_map.insert((b.account_index, b.mint.clone()), amount);
    }

    for b in &meta.post_token_balances {
        let post = b.ui_token_amount.as_ref().map(|x| x.ui_amount).unwrap_or(0.0);
        let pre = pre_token_map.get(&(b.account_index, b.mint.clone())).copied().unwrap_or(0.0);

        if (post - pre).abs() > f64::EPSILON {
            let address = all_accounts.get(b.account_index as usize).cloned().unwrap_or_default();
            changes.push(BalanceChange {
                address,
                mint: b.mint.clone(),
                change: post - pre,
                post_balance: post,
            });
        }
    }

    changes
}