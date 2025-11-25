use std::collections::{HashMap, HashSet};

use crate::constants::JUPITER_PROGRAM_IDS;
use crate::pb::sf::jupiter::v1::{
    AccountOwnerRecords, EnrichedAccount, JupiterInstruction, JupiterInstructions, TokenPriceList,
    TradingDataList,
};
use substreams::errors::Error;
use substreams_solana::{base58, pb::sf::solana::r#type::v1::Block};
use crate::{is_relevant_tx, parse_filters};

#[substreams::handlers::map]
pub fn map_jupiter_instructions(
    params: String, // <--- Add this input
    block: Block,
    owner_records: AccountOwnerRecords,
    trading_data: TradingDataList,
    token_prices: TokenPriceList,
) -> Result<JupiterInstructions, Error> {
    let filter_addresses = parse_filters(&params);

    let owner_index = build_owner_index(owner_records);
    let price_index = build_price_index(token_prices);
    let trades_by_tx = group_trades_by_tx(trading_data);

    let mut instructions = Vec::new();
    let block_time = block
        .block_time
        .as_ref()
        .map(|ts| ts.timestamp.max(0) as u64)
        .unwrap_or_default();

    for trx in block.transactions() {
        // 2. Apply Filter
        if !is_relevant_tx(&trx, &filter_addresses) {
            continue;
        }

        let tx_id = trx.id();
        let trade_data = trades_by_tx.get(&tx_id);

        for instruction in trx.walk_instructions() {
            let program_id = instruction.program_id().to_string();
            if !is_jupiter_program(&program_id) {
                continue;
            }

            let enriched_accounts = instruction
                .accounts()
                .iter()
                .map(|address| enrich_account(address.to_string(), &owner_index, &price_index))
                .collect::<Vec<_>>();

            let mut data = instruction.data().clone();
            if data.is_empty() {
                if let Some(trades) = trade_data {
                    if let Some(first_trade) = trades.first() {
                        data = first_trade.data.clone();
                    }
                }
            }

            instructions.push(JupiterInstruction {
                program_id,
                transaction_id: tx_id.clone(),
                accounts: enriched_accounts,
                data,
                slot: block.slot,
                block_time,
            });
        }
    }

    Ok(JupiterInstructions { instructions })
}

fn build_owner_index(records: AccountOwnerRecords) -> HashMap<String, (String, String)> {
    records
        .records
        .into_iter()
        .map(|record| {
            let account = base58::encode(&record.account);
            let owner = base58::encode(&record.owner);
            let mint = base58::encode(&record.mint);
            (account, (owner, mint))
        })
        .collect()
}

fn build_price_index(token_prices: TokenPriceList) -> HashSet<String> {
    token_prices
        .items
        .into_iter()
        .map(|price| price.mint_address)
        .collect()
}

fn group_trades_by_tx(trading_data: TradingDataList) -> HashMap<String, Vec<crate::pb::sf::jupiter::v1::TradingData>> {
    let mut map: HashMap<String, Vec<_>> = HashMap::new();
    for trade in trading_data.items {
        map.entry(trade.transaction_id.clone())
            .or_default()
            .push(trade);
    }
    map
}

fn enrich_account(
    address: String,
    owner_index: &HashMap<String, (String, String)>,
    price_index: &HashSet<String>,
) -> EnrichedAccount {
    if let Some((owner, mint)) = owner_index.get(&address) {
        return EnrichedAccount {
            address,
            owner: owner.clone(),
            mint: mint.clone(),
        };
    }

    let mint = if price_index.contains(&address) {
        address.clone()
    } else {
        String::new()
    };

    EnrichedAccount {
        address,
        owner: String::new(),
        mint,
    }
}

fn is_jupiter_program(program_id: &str) -> bool {
    JUPITER_PROGRAM_IDS.iter().any(|entry| entry == &program_id)
}

