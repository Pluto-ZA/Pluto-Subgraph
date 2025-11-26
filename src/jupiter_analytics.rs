use std::collections::{HashMap, HashSet};
// todo remove, not needed
use crate::pb::sf::jupiter::v1::{JupiterAnalytics, JupiterInstructions, ProgramStat};
use substreams::errors::Error;

#[substreams::handlers::map]
pub fn map_jupiter_analytics(instructions: JupiterInstructions) -> Result<JupiterAnalytics, Error> {
    let mut account_set = HashSet::new();
    let mut mint_set = HashSet::new();
    let mut program_counts: HashMap<String, u64> = HashMap::new();

    for instruction in instructions.instructions.iter() {
        *program_counts
            .entry(instruction.program_id.clone())
            .or_insert(0) += 1;

        for account in instruction.accounts.iter() {
            account_set.insert(account.address.clone());
            if !account.mint.is_empty() {
                mint_set.insert(account.mint.clone());
            }
        }
    }

    let mut top_programs = program_counts
        .into_iter()
        .map(|(program_id, instruction_count)| ProgramStat {
            program_id,
            instruction_count,
        })
        .collect::<Vec<_>>();
    top_programs.sort_by(|a, b| b.instruction_count.cmp(&a.instruction_count));
    top_programs.truncate(5);

    Ok(JupiterAnalytics {
        total_instructions: instructions.instructions.len() as u64,
        unique_accounts: account_set.len() as u64,
        unique_mints: mint_set.len() as u64,
        top_programs,
    })
}

