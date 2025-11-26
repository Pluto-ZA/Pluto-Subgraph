use crate::pb::sf::jupiter::v1::TransactionHistory;
use substreams::store::{StoreAdd, StoreAddFloat64, StoreNew};

#[substreams::handlers::store]
pub fn store_portfolio_balances(history: TransactionHistory, output: StoreAddFloat64) {
    for tx in history.transactions {
        for change in tx.balance_changes {
            let mint = if change.mint.is_empty() { "SOL".to_string() } else { change.mint };

            // We store the DELTA (change)
            // The Store will automatically accumulate it to keep the running total.
            let key = format!("{}:{}", change.address, mint);
            output.add(0, key, change.change);
        }
    }
}