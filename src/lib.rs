pub mod constants;
pub mod pb;
pub mod spl_account_store;
pub mod jupiter_trading_store;
pub mod token_price_store;
pub mod jupiter_instructions;
pub mod balances;
pub mod jupiter_analytics;

use substreams_database_change::pb::database::DatabaseChanges;
use substreams_database_change::tables::Tables;
pub use spl_account_store::map_spl_initialized_account;
pub use jupiter_trading_store::map_jupiter_trading_data;
pub use token_price_store::map_token_prices;
pub use jupiter_instructions::map_jupiter_instructions;
pub use jupiter_analytics::map_jupiter_analytics;
pub use balances::map_balance_changes;
use crate::pb::sf::jupiter::v1::BalanceChanges;

#[substreams::handlers::map]
pub fn db_out(changes: BalanceChanges) -> Result<DatabaseChanges, substreams::errors::Error> {
    let mut tables = Tables::new();

    for change in changes.params {
        // "wallet_balance_changes" must match your ClickHouse CREATE TABLE name
        let key = format!("{}:{}:{}", change.tx_id, change.owner, change.mint);

        tables
            .create_row("wallet_balance_changes", key)
            .set("block_date", change.block_date)
            .set("block_time", change.block_time)
            .set("block_slot", change.block_slot)
            .set("tx_id", change.tx_id)
            .set("owner", change.owner)
            .set("mint", change.mint)
            .set("change_amount", change.change_amount)
            .set("new_balance", change.new_balance)
            .set("decimals", change.decimals)
            .set("network_fee", change.network_fee.to_string());
    }

    Ok(tables.to_database_changes())
}