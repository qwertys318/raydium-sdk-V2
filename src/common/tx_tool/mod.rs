pub mod tx_type;

use crate::common::owner::Owner;
use crate::common::tx_tool::tx_type::InstructionType;
use crate::raydium::tpe::MakeInstructionsResult;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::{v0, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;
use solana_sdk::system_instruction;

pub struct TxBuilder<'a> {
    owner: &'a Owner,
    fee_payer: Pubkey,
    instructions: Vec<Instruction>,
    // end_instructions: ,
    // signers: Vec<>,
    instruction_types: Vec<InstructionType>,
    // end_instruction_types: ,
    lookup_table_address: Vec<String>,
}
pub struct ComputeBudgetConfig {
    units: u32,
    micro_lamports: u64,
}
impl ComputeBudgetConfig {
    pub fn new(units: u32, micro_lamports: u64) -> Self {
        Self { units, micro_lamports }
    }
}
pub struct TxTipConfig {
    tip_account: Pubkey,
    lamports: u64,
}
impl TxTipConfig {
    pub fn new(tip_account: Pubkey, lamports: u64) -> Self {
        Self { tip_account, lamports }
    }
}
impl<'a> TxBuilder<'a> {
    pub fn new(owner: &'a Owner, fee_payer: Pubkey) -> Self {
        Self {
            owner,
            fee_payer,
            instructions: vec![],
            instruction_types: vec![],
            lookup_table_address: vec![],
        }
    }
    pub fn add_tip_instruction(&mut self, cfg: TxTipConfig) {
        //@TODO endinstructions
        self.instructions.push(system_instruction::transfer(
            &self.fee_payer,
            &cfg.tip_account,
            cfg.lamports,
        ));
        self.instruction_types.push(InstructionType::TransferTip);
    }
    pub fn add_custom_compute_budget(&mut self, cfg: ComputeBudgetConfig) {
        self.instructions.splice(
            0..0,
            vec![
                ComputeBudgetInstruction::set_compute_unit_price(cfg.micro_lamports),
                ComputeBudgetInstruction::set_compute_unit_limit(cfg.units),
            ],
        );
        self.instruction_types.splice(
            0..0,
            vec![
                InstructionType::SetComputeUnitPrice,
                InstructionType::SetComputeUnitLimit,
            ],
        );
    }
    pub fn add_instruction(&mut self, mut instruction: MakeInstructionsResult) {
        //@TODO endInstructions, signers, endInstructionTypes
        self.instructions.append(&mut instruction.instructions);
        self.instruction_types
            .append(&mut instruction.instruction_types);
        let def_pub = Pubkey::default().to_string();
        self.lookup_table_address.extend(
            &mut instruction
                .lookup_table_address
                .into_iter()
                .filter(|x| x != &def_pub),
        );
    }
    pub fn build_v0(self, recent_blockhash: Hash) -> VersionedTransaction {
        // @TODO
        let lookup_table: Vec<AddressLookupTableAccount> = vec![];
        VersionedTransaction::try_new(
            VersionedMessage::V0(
                v0::Message::try_compile(
                    &self.fee_payer,
                    &self.instructions,
                    &lookup_table,
                    recent_blockhash,
                )
                .unwrap(),
            ),
            &[self.owner.keypair().unwrap()],
        )
        .unwrap()
    }
}
