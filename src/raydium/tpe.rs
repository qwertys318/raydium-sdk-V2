use solana_sdk::instruction::Instruction;
use crate::common::tx_tool::tx_type::InstructionType;

pub struct MakeInstructionsResult {
    // pub signers: Vec<()>,
    pub instructions: Vec<Instruction>,
    pub instruction_types: Vec<InstructionType>,
    // pub address: Option<T>,
    pub lookup_table_address: Vec<String>,
}