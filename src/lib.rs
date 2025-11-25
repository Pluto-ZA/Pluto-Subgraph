pub mod constants;
pub mod pb;
pub mod spl_account_store;
pub mod jupiter_trading_store;
pub mod token_price_store;
pub mod jupiter_instructions;
pub mod jupiter_analytics;
pub use spl_account_store::map_spl_initialized_account;
pub use jupiter_trading_store::map_jupiter_trading_data;
pub use token_price_store::map_token_prices;
pub use jupiter_instructions::map_jupiter_instructions;
pub use jupiter_analytics::map_jupiter_analytics;

use std::collections::HashSet;
use substreams_solana::pb::sf::solana::r#type::v1::ConfirmedTransaction;

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