use crate::pb::sf::jupiter::v1::{WalletAggregates, WalletAggregatesList, TokenBalance};
use substreams::store::{StoreGet, StoreGetFloat64, Deltas, DeltaFloat64};
use substreams::errors::Error;
use std::collections::HashMap;

#[substreams::handlers::map]
pub fn map_stats_updates(
    balance_deltas: Deltas<DeltaFloat64>,
    volume_deltas: Deltas<DeltaFloat64>,
    prices: StoreGetFloat64,
) -> Result<WalletAggregatesList, Error> {

    // Use a Map to group updates by Wallet Address
    let mut aggregates_map: HashMap<String, WalletAggregates> = HashMap::new();

    // --- Helper to get or create the Aggregate object for a specific wallet ---
    fn new_aggregate(wallet: &str) -> WalletAggregates {
        WalletAggregates {
            wallet: wallet.to_string(),
            total_trading_volume_usd: 0.0,
            monthly_trading_volume_usd: HashMap::new(),
            portfolio: vec![],
        }
    }


    // 1. Process Balance Updates
    for delta in balance_deltas.deltas {
        // Key Format: "wallet_address:mint_address"
        // We split by the first colon only, to be safe
        let parts: Vec<&str> = delta.key.splitn(2, ':').collect();

        if parts.len() == 2 {
            let wallet = parts[0];
            let mint = parts[1];
            let quantity = delta.new_value;

            // Get Price
            let price = prices.get_last(mint).unwrap_or(0.0);
            let value_usd = quantity * price;

            // Update specific wallet
            let agg = aggregates_map
                .entry(wallet.to_string())
                .or_insert_with(|| new_aggregate(wallet));

            agg.portfolio.push(TokenBalance {
                mint: mint.to_string(),
                amount: quantity,
                value_usd,
            });
        }
    }

    // 2. Process Volume Updates
    for delta in volume_deltas.deltas {
        // Key Format: "wallet_address:total" OR "wallet_address:YYYY-MM"
        let parts: Vec<&str> = delta.key.splitn(2, ':').collect();

        if parts.len() == 2 {
            let wallet = parts[0];
            let suffix = parts[1];

            let agg = aggregates_map
                .entry(wallet.to_string())
                .or_insert_with(|| new_aggregate(wallet));

            if suffix == "total" {
                agg.total_trading_volume_usd = delta.new_value;
            } else {
                agg.monthly_trading_volume_usd.insert(suffix.to_string(), delta.new_value);
            }
        }
    }

    // 3. Convert Map to Output List
    let items: Vec<WalletAggregates> = aggregates_map.into_values().collect();

    Ok(WalletAggregatesList { items })
}