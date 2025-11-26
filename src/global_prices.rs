use crate::pb::sf::jupiter::v1::{TokenPrice, TokenPriceList};
use crate::calculate_balance_changes;
use substreams::errors::Error;
use substreams_solana::pb::sf::solana::r#type::v1::Block;
use std::collections::HashMap;

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

#[substreams::handlers::map]
pub fn map_global_token_prices(block: Block) -> Result<TokenPriceList, Error> {
    let mut block_prices: HashMap<String, f64> = HashMap::new();

    for trx in block.transactions() {
        // 1. Skip failed transactions
        if let Some(meta) = &trx.meta {
            if meta.err.is_some() { continue; }
        }

        // 2. We do NOT use `is_relevant_tx` here. We want GLOBAL data.

        // 3. Analyze Balance Changes
        let changes = calculate_balance_changes(&trx);

        // 4. Find Price Discovery Patterns (Stable <-> Token)
        let mut stable_amt = 0.0;
        let mut other_amt = 0.0;
        let mut other_mint = String::new();

        for c in changes {
            if c.mint == USDC_MINT || c.mint == USDT_MINT {
                stable_amt = c.change.abs();
            } else {
                other_amt = c.change.abs();
                other_mint = if c.mint.is_empty() { SOL_MINT.to_string() } else { c.mint };
            }
        }

        // Filter out tiny dust trades to avoid bad price data
        if stable_amt > 1.0 && other_amt > 0.000001 && !other_mint.is_empty() {
            let price = stable_amt / other_amt;
            // We overwrite; usually the last tx in the block is the most "current"
            block_prices.insert(other_mint, price);
        }
    }

    // Convert map to output list
    let items = block_prices.into_iter().map(|(mint, price)| {
        TokenPrice {
            mint_address: mint,
            price_usd: price,
            volume_24h: 0.0,
            price_change_24h: 0.0,
            slot: block.slot,
        }
    }).collect();

    Ok(TokenPriceList { items })
}