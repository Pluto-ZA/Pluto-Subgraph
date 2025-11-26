use crate::constants::JUPITER_PROGRAM_IDS;
use crate::pb::sf::jupiter::v1::{TradingData, TradingDataList};
use substreams::errors::Error;
use substreams_solana::pb::sf::solana::r#type::v1::Block;
use crate::{calculate_balance_changes, is_relevant_tx, parse_filters};

#[substreams::handlers::map]
pub fn map_jupiter_trading_data(params: String, block: Block) -> Result<TradingDataList, Error> {
    let filter_addresses = parse_filters(&params);

    let mut items = Vec::new();
    let block_time = block
        .block_time
        .as_ref()
        .map(|ts| ts.timestamp.max(0) as u64)
        .unwrap_or_default();

    for trx in block.transactions() {
        if !is_relevant_tx(&trx, &filter_addresses) {
            continue;
        }

        let tx_id = trx.id();

        for instruction in trx.walk_instructions() {
            let program_id = instruction.program_id().to_string();
            if !is_jupiter_program(&program_id) {
                continue;
            }

            let accounts = instruction
                .accounts()
                .iter()
                .map(|address| address.to_string())
                .collect::<Vec<_>>();

            items.push(TradingData {
                program_id,
                transaction_id: tx_id.clone(),
                accounts,
                data: instruction.data().clone(),
                slot: block.slot,
                block_time,
                balance_changes: calculate_balance_changes(&trx),
            });
        }
    }

    Ok(TradingDataList { items })
}



fn is_jupiter_program(program_id: &str) -> bool {
    JUPITER_PROGRAM_IDS.iter().any(|id| id == &program_id)
}