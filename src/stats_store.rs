use crate::pb::sf::jupiter::v1::TradingDataList;
use substreams::store::{StoreAdd, StoreAddFloat64, StoreNew, StoreSetProto};
use substreams::store::DeltaFloat64;

// Constants for Stablecoins (for volume estimation)
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

#[substreams::handlers::store]
pub fn store_trading_volume(data: TradingDataList, output: StoreAddFloat64) {
    for trade in data.items {
        // 1. Identify the Wallet (Usually the first account in accounts list for Jupiter)
        // Adjust index based on specific Jupiter version/instruction structure if needed
        let wallet = match trade.accounts.first() {
            Some(w) => w.clone(),
            None => continue,
        };

        // 2. Estimate USD Value
        // Note: Real production apps need the `token_price_store` to calculate this accurately.
        // Here we use a heuristic: parsing logs or instruction data would be better,
        // but let's assume you have a helper or simplified logic:
        let volume_usd = calculate_trade_volume(&trade);

        if volume_usd > 0.0 {
            // A. Store Total Volume: "wallet_addr:total"
            output.add(0, format!("{}:total", wallet), volume_usd);

            // B. Store Monthly Volume: "wallet_addr:YYYY-MM"
            let date_str = format_timestamp_to_month(trade.block_time);
            output.add(0, format!("{}:{}", wallet, date_str), volume_usd);
        }
    }
}

// Helper to format UNIX timestamp to "YYYY-MM"
fn format_timestamp_to_month(ts: u64) -> String {
    // Basic formatting (requires chrono or manual math)
    // Simplified manual math for WASM safety:
    let seconds_per_day = 86400;
    let days = ts / seconds_per_day;
    let year = 1970 + (days / 365); // Rough approximation
    let month = (days % 365) / 30 + 1; // Very rough, use `chrono` crate in production
    format!("{:04}-{:02}", year, month)
}

// Placeholder for volume calculation logic
fn calculate_trade_volume(_trade: &crate::pb::sf::jupiter::v1::TradingData) -> f64 {
    // TODO: Inspect `trade.data` (instruction data) or logs to find the amount
    // swapped and match it against a known stablecoin.
    // Return 0.0 if unknown.
    100.0 // Mock value for demonstration
}