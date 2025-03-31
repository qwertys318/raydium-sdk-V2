use crate::api::tpe::ClmmKeys;
use crate::common::owner::OwnerInfo;
use crate::common::pubkey::WSOL_MINT;
use crate::common::tx_tool::TxBuilder;
use crate::raydium::clmm::instrument::ClmmInstrument;
use crate::raydium::clmm::tpe::ComputeClmmPoolInfo;
use crate::raydium::clmm::utils::constants::{
    MAX_SQRT_PRICE_X64_SUB_ONE, MIN_SQRT_PRICE_X64_ADD_ONE,
};
use crate::raydium::clmm::utils::math::SqrtPriceMath;
use crate::raydium::module_base::ModuleBase;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;

pub mod instrument;
pub mod tpe;
pub mod utils;

pub struct Clmm {
    pub base: ModuleBase,
}

impl Clmm {
    pub fn new(base: ModuleBase) -> Self {
        Self { base }
    }

    /// @TODO It does not create\fetch token accounts.
    /// @TODO So, make sure you created it in advance
    /// @TODO And clmm.base.account.fetch_wallet_token_accounts() was executed.
    pub async fn swap(
        &self,
        latest_blockhash: solana_hash::Hash,
        pool_info: &ComputeClmmPoolInfo,
        prop_pool_keys: Option<ClmmKeys>,
        input_mint: &Pubkey,
        amount_in: rug::Integer,
        amount_out_min: rug::Integer,
        owner_info: OwnerInfo,
        remaining_accounts: Vec<Pubkey>,
        price_limit: Option<rug::Float>,
        observation_id: Pubkey,
        fee_payer: Option<Pubkey>,
        is_associated_only: bool,
    ) -> Result<VersionedTransaction, String> {
        let mut tx_builder = self.create_tx_builder(fee_payer)?;
        let base_in = input_mint == &pool_info.pool_state.token_mint0;
        let (mint_a_use_sol_balance, mint_b_use_sol_balance) = match owner_info.use_sol_balance {
            None => (false, false),
            Some(use_sol_balance) => (
                use_sol_balance && pool_info.pool_state.token_mint0.to_string() == WSOL_MINT,
                use_sol_balance && pool_info.pool_state.token_mint1.to_string() == WSOL_MINT,
            ),
        };
        let sqrt_price_limit_x64 =
            if price_limit.is_none() || price_limit.as_ref().unwrap().is_zero() {
                if base_in {
                    MIN_SQRT_PRICE_X64_ADD_ONE
                } else {
                    MAX_SQRT_PRICE_X64_SUB_ONE
                }
            } else {
                SqrtPriceMath::price_to_sqrt_price_x64(
                    price_limit.as_ref().unwrap(),
                    pool_info.pool_state.mint_decimals0,
                    pool_info.pool_state.mint_decimals1,
                )
            };
        let acc_a_is_associated_only = if mint_a_use_sol_balance {
            false
        } else {
            is_associated_only
        };
        let owner_token_account_a = self
            .base
            .scope
            .account
            .get_token_account(
                &pool_info.pool_state.token_mint0,
                /* @TODO */ None,
                acc_a_is_associated_only,
            )
            .ok_or(format!("Token account A {} was not found", pool_info.pool_state.token_mint0))?;
        let acc_b_is_associated_only = if mint_b_use_sol_balance {
            false
        } else {
            is_associated_only
        };
        let owner_token_account_b = self
            .base
            .scope
            .account
            .get_token_account(
                &pool_info.pool_state.token_mint1,
                /* @TODO */ None,
                acc_b_is_associated_only,
            )
            .ok_or(format!("Token account B {} was not found", pool_info.pool_state.token_mint1))?;
        let pool_keys = prop_pool_keys.ok_or("@TODO self.get_clmm_pool_keys")?;
        let owner_info = instrument::OwnerInfo {
            wallet: self.base.scope.owner_pubkey().unwrap(),
            token_account_a: owner_token_account_a.pubkey.unwrap(),
            token_account_b: owner_token_account_b.pubkey.unwrap(),
        };
        tx_builder.add_instruction(ClmmInstrument::make_swap_base_in_instructions(
            pool_info,
            &pool_keys,
            &observation_id,
            &owner_info,
            input_mint,
            amount_in,
            amount_out_min,
            sqrt_price_limit_x64,
            remaining_accounts,
        ));

        // @TODO tx_builder.add_custom_compute_budget(compute_budget_config);
        // @TODO tx_builder.add_tip_instruction(tx_tip_config);
        Ok(tx_builder.build_v0(latest_blockhash))
    }
    fn create_tx_builder(&self, fee_payer: Option<Pubkey>) -> Result<TxBuilder, String> {
        Ok(TxBuilder::new(
            self.base.scope.account.owner.as_ref().unwrap(),
            fee_payer.unwrap_or(self.base.scope.owner_pubkey()?.clone()),
        ))
    }
}
