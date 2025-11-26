use crate::pb::sf::jupiter::v1::TokenPriceList;
use substreams::store::{StoreNew, StoreSet, StoreSetFloat64};

#[substreams::handlers::store]
pub fn store_token_prices(price_list: TokenPriceList, output: StoreSetFloat64) {
    for item in price_list.items {
        output.set(0, item.mint_address, &item.price_usd);
    }
}