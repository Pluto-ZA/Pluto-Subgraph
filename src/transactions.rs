use crate::pb::sf::jupiter::v1::{BalanceChange, TransactionHistory, TransactionItem};
use crate::{is_relevant_tx, parse_filters};
use std::collections::HashMap;
use substreams::errors::Error;
use substreams_solana::pb::sf::solana::r#type::v1::{Block, ConfirmedTransaction};

#[substreams::handlers::map]
pub fn map_relevant_transactions(params: String, block: Block) -> Result<TransactionHistory, Error> {
    let filter_addresses = parse_filters(&params);
    let mut transactions = Vec::new();

    let block_time = block
        .block_time
        .as_ref()
        .map(|ts| ts.timestamp.max(0) as u64)
        .unwrap_or_default();

    for trx in block.transactions() {
        // 1. Filter
        if !is_relevant_tx(&trx, &filter_addresses) {
            continue;
        }

        let signature = trx.transaction
            .as_ref() // Access the Option<Transaction>
            .and_then(|t| t.signatures.first()) // Get the first signature if it exists
            .map(|bytes| bs58::encode(bytes).into_string()) // Encode it
            .unwrap_or_else(|| "Unknown".to_string()); // Fallback

        let meta = match &trx.meta {
            Some(m) => m,
            None => continue,
        };

        // 2. Resolve all accounts (Static + Lookup Tables) to map Index -> Address
        let all_accounts = resolve_accounts(&trx);

        // 3. Extract sender and involved accounts
        let sender = all_accounts.first().cloned().unwrap_or_default();
        let involved_accounts = all_accounts.clone();

        // 4. Calculate Balance Changes
        let mut balance_changes = Vec::new();

        // A. Native SOL Changes (Lamports -> SOL)
        for (i, address) in all_accounts.iter().enumerate() {
            let pre = *meta.pre_balances.get(i).unwrap_or(&0);
            let post = *meta.post_balances.get(i).unwrap_or(&0);

            if pre != post {
                let change_lamports = post as i64 - pre as i64;
                balance_changes.push(BalanceChange {
                    address: address.clone(),
                    mint: "".to_string(), // Empty for SOL
                    change: change_lamports as f64 / 1_000_000_000.0, // Convert to SOL
                    post_balance: post as f64 / 1_000_000_000.0,
                });
            }
        }

        // B. SPL Token Changes
        // Map (AccountIndex, Mint) -> PreAmount
        let mut pre_token_map: HashMap<(u32, String), f64> = HashMap::new();
        for b in &meta.pre_token_balances {
            let amount = b.ui_token_amount.as_ref().map(|x| x.ui_amount).unwrap_or(0.0);
            pre_token_map.insert((b.account_index, b.mint.clone()), amount);
        }

        // Iterate Post Balances to find changes
        for b in &meta.post_token_balances {
            let post_amount = b.ui_token_amount.as_ref().map(|x| x.ui_amount).unwrap_or(0.0);
            let pre_amount = pre_token_map
                .get(&(b.account_index, b.mint.clone()))
                .copied()
                .unwrap_or(0.0);

            if (post_amount - pre_amount).abs() > f64::EPSILON {
                // Get the owner address from the account index
                let address = all_accounts
                    .get(b.account_index as usize)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());

                balance_changes.push(BalanceChange {
                    address,
                    mint: b.mint.clone(),
                    change: post_amount - pre_amount,
                    post_balance: post_amount,
                });
            }
        }

        // Remove 'pre' entries that were closed (not present in post)?
        // Usually closed accounts appear in post with 0 balance, so the loop above handles it.

        transactions.push(TransactionItem {
            signature,
            slot: block.slot,
            block_time,
            success: meta.err.is_none(),
            sender,
            involved_accounts,
            fee: meta.fee,
            log_messages: meta.log_messages.clone(),
            balance_changes,
        });
    }

    Ok(TransactionHistory { transactions })
}

// Helper: Constructs the full list of accounts (Static Keys + Address Lookup Tables)
fn resolve_accounts(tx: &ConfirmedTransaction) -> Vec<String> {
    let mut accounts = Vec::new();

    if let Some(transaction) = &tx.transaction {
        if let Some(message) = &transaction.message {
            // 1. Static Keys
            for key in &message.account_keys {
                accounts.push(bs58::encode(key).into_string());
            }

            // 2. Loaded Addresses (Version 0 Transactions)
            if let Some(meta) = &tx.meta {
                for key in &meta.loaded_writable_addresses {
                    accounts.push(bs58::encode(key).into_string());
                }
                for key in &meta.loaded_readonly_addresses {
                    accounts.push(bs58::encode(key).into_string());
                }
            }
        }
    }
    accounts
}