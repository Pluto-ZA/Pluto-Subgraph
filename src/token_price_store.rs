use std::collections::HashSet;

use crate::pb::sf::jupiter::v1::{TokenPrice, TokenPriceList, TradingDataList};
use substreams::errors::Error;

// todo adjust to store token prices
#[substreams::handlers::map]
pub fn map_token_prices(trading_data: TradingDataList) -> Result<TokenPriceList, Error> {
    let mut seen = HashSet::new();
    let mut prices = Vec::new();

    for trade in trading_data.items {
        if let Some(account) = trade.accounts.first() {
            if seen.insert(account.clone()) {
                prices.push(TokenPrice {
                    mint_address: account.clone(),
                    // todo
                    price_usd: 0.0,
                    volume_24h: 0.0,
                    price_change_24h: 0.0,
                    slot: trade.slot,
                });
            }
        }
    }

    Ok(TokenPriceList { items: prices })
}
