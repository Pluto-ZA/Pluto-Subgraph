use substreams_solana::pb::sf::solana::r#type::v1 as solana;
use crate::pb::sf::jupiter::v1::{BalanceChange, BalanceChanges};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, NaiveDateTime, Utc};
use substreams_solana::base58;

// --- CONSTANTS ---
const WRAPPED_SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

// --- PROGRAM IDS ---

// JUPITER AGGREGATOR (All Versions)
const JUPITER_V6_PROGRAM: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const JUPITER_V4_PROGRAM: &str = "JUP4Fb2cqiUsePSIwy4c6inR7aUBkRSd8kPU86hAhuq";
const JUPITER_V3_PROGRAM: &str = "JUP3c2Uh3WA4Ng34tw6kPd2G4C5BB21Xo36Je1s32Ph";
const JUPITER_V2_PROGRAM: &str = "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo"; // <--- ADDED V2
const JUPITER_LIMIT_PROGRAM: &str = "jupoNjAxXgZqQXzgFSSmu9cGIqmq8cz3vipr5Z9i9d";

// OTHER DEX / SWAPS (Direct interactions)
const PHOENIX_PROGRAM: &str = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY";
const RAYDIUM_V4_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const ORCA_WHIRLPOOL_PROGRAM: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

// NFT MARKETS
const MAGIC_EDEN_V2_PROGRAM: &str = "M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K";
const TENSOR_SWAP_PROGRAM: &str = "TSWAPaqyCSx2KABkXpnVkqXVHkBNbLKeW8SagU25N1";

// LENDING / BORROWING
const KAMINO_LENDING_PROGRAM: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
const MARGINFI_V2_PROGRAM: &str = "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVac6";
const SOLEND_PROGRAM: &str = "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3ZUE8uB5y";

// STAKING (Liquid & Native)
const STAKE_PROGRAM: &str = "Stake11111111111111111111111111111111111111";
const MARINADE_PROGRAM: &str = "MarBmsSgKXdrN1egZf5sqe1CJNPUbNEXRPn4nsPAafF";
const JITO_STAKE_PROGRAM: &str = "Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb";

// PERPETUALS
const DRIFT_V2_PROGRAM: &str = "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH";

#[substreams::handlers::map]
pub fn map_balance_changes(params: String, block: solana::Block) -> Result<BalanceChanges, substreams::errors::Error> {
    let mut balance_changes = vec![];

    // 0. Parse Whitelist
    let whitelist: HashSet<String> = params
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let use_whitelist = !whitelist.is_empty();

    // Timestamp & Date
    let timestamp = block.block_time.as_ref().map(|t| t.timestamp).unwrap_or(0);
    let dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap_or_default(), Utc);
    let block_date = dt.format("%Y-%m-%d").to_string();
    let slot = block.slot;

    for trx in block.transactions {
        if let Some(meta) = &trx.meta {
            // 1. Skip Failed Transactions
            if meta.err.is_some() {
                continue;
            }

            // Transaction ID
            let tx_id = match &trx.transaction {
                Some(inner_tx) => {
                    if inner_tx.signatures.is_empty() { continue; }
                    base58::encode(&inner_tx.signatures[0])
                },
                None => continue,
            };

            // Unpack Message
            let message = match trx.transaction.as_ref().and_then(|t| t.message.as_ref()) {
                Some(m) => m,
                None => continue,
            };
            let accounts = &message.account_keys;

            // ---------------------------------------------------------
            // IDENTIFY TRANSACTION TYPE
            // ---------------------------------------------------------

            let mut detected_type = "SEND".to_string();
            let mut highest_priority = 0;

            // Mapping Logic
            let get_program_label = |prog_id: &str| -> Option<&str> {
                match prog_id {
                    // Jupiter Aggregator
                    JUPITER_V6_PROGRAM |
                    JUPITER_V4_PROGRAM |
                    JUPITER_V3_PROGRAM |
                    JUPITER_V2_PROGRAM | // <--- Matched here
                    JUPITER_LIMIT_PROGRAM => Some("SWAP_JUPITER"),

                    // Direct DEX Swaps
                    PHOENIX_PROGRAM => Some("SWAP_PHOENIX"),
                    RAYDIUM_V4_PROGRAM => Some("SWAP_RAYDIUM"),
                    ORCA_WHIRLPOOL_PROGRAM => Some("SWAP_ORCA"),

                    // NFT
                    MAGIC_EDEN_V2_PROGRAM => Some("NFT_TRADE_MAGIC_EDEN"),
                    TENSOR_SWAP_PROGRAM => Some("NFT_TRADE_TENSOR"),

                    // Lending
                    KAMINO_LENDING_PROGRAM => Some("LEND_KAMINO"),
                    MARGINFI_V2_PROGRAM => Some("LEND_MARGINFI"),
                    SOLEND_PROGRAM => Some("LEND_SOLEND"),

                    // Staking
                    STAKE_PROGRAM => Some("STAKE_NATIVE"),
                    MARINADE_PROGRAM => Some("STAKE_MARINADE"),
                    JITO_STAKE_PROGRAM => Some("STAKE_JITO"),

                    // Perps
                    DRIFT_V2_PROGRAM => Some("PERP_DRIFT"),

                    _ => None,
                }
            };

            // Priority Logic
            let mut check_instruction = |prog_index: usize| {
                if prog_index < accounts.len() {
                    let prog_id = base58::encode(&accounts[prog_index]);

                    if let Some(label) = get_program_label(&prog_id) {
                        let priority = match label {
                            // NFT Trades are specific; usually override swaps
                            s if s.starts_with("NFT") => 5,

                            // Lending/Perps often use swaps internally, so we prioritize them
                            // if the Lending Program is the top-level invoker.
                            s if s.starts_with("LEND") => 4,
                            s if s.starts_with("PERP") => 4,

                            // Swaps
                            s if s.starts_with("SWAP") => 3,

                            // Staking
                            s if s.starts_with("STAKE") => 2,

                            _ => 1,
                        };

                        if priority > highest_priority {
                            highest_priority = priority;
                            detected_type = label.to_string();
                        }
                    }
                }
            };

            // A. Top-Level Instructions
            for inst in &message.instructions {
                check_instruction(inst.program_id_index as usize);
            }

            // B. Inner Instructions (CPI)
            for inner in &meta.inner_instructions {
                for inst in &inner.instructions {
                    check_instruction(inst.program_id_index as usize);
                }
            }

            // ---------------------------------------------------------
            // 1. NATIVE SOL CHANGES
            // ---------------------------------------------------------
            if meta.pre_balances.len() == meta.post_balances.len() {
                for (i, pre_lamports) in meta.pre_balances.iter().enumerate() {
                    let post_lamports = meta.post_balances[i];

                    if *pre_lamports == post_lamports { continue; }

                    if i < accounts.len() {
                        let address = base58::encode(&accounts[i]);

                        if use_whitelist && !whitelist.contains(&address) { continue; }

                        let pre_amt = *pre_lamports as f64 / LAMPORTS_PER_SOL;
                        let post_amt = post_lamports as f64 / LAMPORTS_PER_SOL;

                        // Dust filter
                        if (post_amt - pre_amt).abs() < f64::EPSILON { continue; }

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
                            change_type: detected_type.clone(),
                        });
                    }
                }
            }

            // ---------------------------------------------------------
            // 2. SPL TOKEN CHANGES
            // ---------------------------------------------------------
            let mut pre_balances: HashMap<(u32, String), f64> = HashMap::new();

            for balance in &meta.pre_token_balances {
                let amount: f64 = balance.ui_token_amount.as_ref().map(|a| a.ui_amount).unwrap_or(0.0);
                pre_balances.insert((balance.account_index, balance.mint.clone()), amount);
            }

            for post_balance in &meta.post_token_balances {
                if post_balance.owner.is_empty() { continue; }

                if use_whitelist && !whitelist.contains(&post_balance.owner) { continue; }

                let account_idx = post_balance.account_index;
                let mint = post_balance.mint.clone();

                let post_amount: f64 = post_balance.ui_token_amount.as_ref().map(|a| a.ui_amount).unwrap_or(0.0);
                let decimals = post_balance.ui_token_amount.as_ref().map(|a| a.decimals).unwrap_or(0);

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
                        change_type: detected_type.clone(),
                    });
                }
            }
        }
    }

    Ok(BalanceChanges { params: balance_changes })
}