pub mod tx_type;

use crate::common::owner::Owner;
use crate::common::tx_tool::tx_type::InstructionType;
use crate::raydium::tpe::MakeInstructionsResult;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::clock::Slot;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::{v0, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;
use std::str::FromStr;

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
