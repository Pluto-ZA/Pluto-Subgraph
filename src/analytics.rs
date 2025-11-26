use crate::pb::sf::jupiter::v1::{WalletAggregates, TokenBalance};
use substreams::store::{StoreGet, StoreGetFloat64, Deltas, DeltaFloat64};
use substreams::errors::Error;
use std::collections::HashMap;

#[substreams::handlers::map]
pub fn map_stats_updates(
    // We listen to changes in the balance store
    balance_deltas: Deltas<DeltaFloat64>,
    // We listen to changes in the volume store
    volume_deltas: Deltas<DeltaFloat64>,
) -> Result<WalletAggregates, Error> {

    let mut aggregates = WalletAggregates {
        wallet: "Updates_In_This_Block".to_string(),
        total_trading_volume_usd: 0.0,
        monthly_trading_volume_usd: HashMap::new(),
        portfolio: vec![],
    };

    // 1. Capture Balance Updates
    for delta in balance_deltas.deltas {
        // Key is "wallet:mint"
        let parts: Vec<&str> = delta.key.split(':').collect();
        if parts.len() == 2 {
            aggregates.portfolio.push(TokenBalance {
                mint: parts[1].to_string(),
                amount: delta.new_value // This is the new running total
            });
        }
    }

    // 2. Capture Volume Updates
    for delta in volume_deltas.deltas {
        // Key is "wallet:total" or "wallet:YYYY-MM"
        let parts: Vec<&str> = delta.key.split(':').collect();
        if parts.len() == 2 {
            if parts[1] == "total" {
                aggregates.total_trading_volume_usd = delta.new_value;
            } else {
                // It's a month
                aggregates.monthly_trading_volume_usd.insert(parts[1].to_string(), delta.new_value);
            }
        }
    }

    Ok(aggregates)
}