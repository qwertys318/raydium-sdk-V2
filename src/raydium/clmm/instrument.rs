use crate::api::tpe::ClmmKeys;
use crate::common::pubkey::MEMO_PROGRAM_ID;
use crate::common::tx_tool::tx_type::InstructionType;
use crate::raydium::clmm::tpe::ComputeClmmPoolInfo;
use crate::raydium::tpe::MakeInstructionsResult;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use crate::raydium::clmm::utils::pda::get_pda_ex_bitmap_account;

const ANCHOR_DATA_BUF_SWAP: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];

pub struct OwnerInfo {
    pub wallet: Pubkey,
    pub token_account_a: Pubkey,
    pub token_account_b: Pubkey,
}

pub struct ClmmInstrument {}
impl ClmmInstrument {
    pub fn swap_instruction(
        program_id: Pubkey,
        payer: Pubkey,
        pool_id: Pubkey,
        amm_config_id: Pubkey,
        input_token_account: Pubkey,
        output_token_account: Pubkey,
        input_vault: Pubkey,
        output_vault: Pubkey,
        input_mint: Pubkey,
        output__mint: Pubkey,
        tick_array: Vec<Pubkey>,
        observation_id: Pubkey,
        amount: rug::Integer,
        other_amount_threshold: rug::Integer,
        sqrt_price_limit_x64: u128,
        is_base_input: bool,
        ex_tick_array_bitmap: Option<Pubkey>,
    ) -> Instruction {
        let mut keys = vec![
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(amm_config_id, false),
            AccountMeta::new(pool_id, false),
            AccountMeta::new(input_token_account, false),
            AccountMeta::new(output_token_account, false),
            AccountMeta::new(input_vault, false),
            AccountMeta::new(output_vault, false),
            AccountMeta::new(observation_id, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_token_2022::ID, false),
            AccountMeta::new_readonly(Pubkey::from_str_const(MEMO_PROGRAM_ID), false),
            AccountMeta::new_readonly(input_mint, false),
            AccountMeta::new_readonly(output__mint, false),
        ];
        if let Some(x) = ex_tick_array_bitmap {
            keys.push(AccountMeta::new(x, false));
        }
        for x in tick_array {
            keys.push(AccountMeta::new(x, false));
        }

        let data = (
            ANCHOR_DATA_BUF_SWAP,
            amount.to_u64().unwrap(),
            other_amount_threshold.to_u64().unwrap(),
            sqrt_price_limit_x64,
            is_base_input,
        );

        Instruction::new_with_bincode(program_id, &data, keys)
    }
    pub fn make_swap_base_in_instructions(
        pool_info: &ComputeClmmPoolInfo,
        pool_keys: &ClmmKeys,
        observation_id: &Pubkey,
        owner_info: &OwnerInfo,
        input_mint: &Pubkey,
        amount_in: rug::Integer,
        amount_out_min: rug::Integer,
        sqrt_price_limit_x64: u128,
        remaining_accounts: Vec<Pubkey>,
    ) -> MakeInstructionsResult {
        let lookup_table_address = match &pool_keys.base.lookup_table_account {
            None => vec![],
            Some(x) => vec![x.clone()],
        };
        let is_input_mint_a = &pool_info.pool_state.token_mint0 == input_mint;
        let (
            input_token_account,
            output_token_account,
            input_vault,
            output_vault,
            input_mint,
            output_mint,
        ) = if is_input_mint_a {
            (
                owner_info.token_account_a,
                owner_info.token_account_b,
                pool_keys.base.vault.a,
                pool_keys.base.vault.b,
                pool_info.pool_state.token_mint0,
                pool_info.pool_state.token_mint1,
            )
        } else {
            (
                owner_info.token_account_b,
                owner_info.token_account_a,
                pool_keys.base.vault.b,
                pool_keys.base.vault.a,
                pool_info.pool_state.token_mint1,
                pool_info.pool_state.token_mint0,
            )
        };
        let swap_instruction = Self::swap_instruction(
            pool_info.program_id,
            owner_info.wallet,
            pool_info.id,
            pool_info.pool_state.amm_config,
            input_token_account,
            output_token_account,
            input_vault,
            output_vault,
            input_mint,
            output_mint,
            remaining_accounts,
            *observation_id,
            amount_in,
            amount_out_min,
            sqrt_price_limit_x64,
            true,
            Some(get_pda_ex_bitmap_account(&pool_info.program_id, &pool_info.id).0)
        );
        MakeInstructionsResult {
            // signers: vec![],
            instructions: vec![swap_instruction],
            instruction_types: vec![InstructionType::ClmmSwapBaseIn],
            lookup_table_address,
        }
    }
}
