pub mod util;

use crate::common::pda::get_ata_address;
use crate::raydium::account::util::parse_token_account_resp;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use tokio::join;
use crate::common::owner::Owner;

pub struct Account {
    pub owner: Option<Owner>,
    pub token_accounts: Vec<TokenAccount>,
}
#[derive(Debug)]
pub struct TokenAccount {
    pub pubkey: Option<Pubkey>,
    pub mint: Pubkey,
    pub is_associated: Option<bool>,
    pub amount: rug::Integer,
    pub is_native: bool,
    pub program_id: Pubkey,
}

impl Account {
    pub fn new(owner: Option<Owner>) -> Self {
        Self {
            owner,
            token_accounts: Vec::new(),
        }
    }

    pub fn owner_pubkey(&self) -> Result<Pubkey, String> {
        match &self.owner {
            None => Err("owner was not set".to_string()),
            Some(x) => Ok(x.pubkey()),
        }
    }

    // @TODO it's currently force_update only
    pub async fn fetch_wallet_token_accounts(
        &mut self,
        client: &RpcClient,
        /*force_update: bool,*/ commitment: CommitmentConfig,
    ) -> Result<&Vec<TokenAccount>, String> {
        let owner = &self.owner.as_ref().unwrap().pubkey();
        let (sol_account_resp, owner_token_account_resp, owner_token_2022_account_resp) = join!(
            client.get_account_with_commitment(owner, commitment),
            client.get_token_accounts_by_owner_with_commitment(
                owner,
                TokenAccountsFilter::ProgramId(spl_token::ID),
                commitment
            ),
            client.get_token_accounts_by_owner_with_commitment(
                owner,
                TokenAccountsFilter::ProgramId(spl_token_2022::ID),
                commitment
            ),
        );
        let sol_account_resp = sol_account_resp
            .map_err(|e| e.to_string())?
            .value
            .ok_or(format!("Account {} was not found.", owner.to_string()))?;
        let owner_token_account_resp = owner_token_account_resp.map_err(|e| e.to_string())?;
        let owner_token_2022_account_resp =
            owner_token_2022_account_resp.map_err(|e| e.to_string())?;
        let rpc_token_accounts = owner_token_account_resp
            .value
            .into_iter()
            .chain(owner_token_2022_account_resp.value.into_iter())
            .collect();
        let token_accounts =
            parse_token_account_resp(owner, Some(sol_account_resp), rpc_token_accounts);
        self.token_accounts = token_accounts;
        Ok(&self.token_accounts)
    }
    pub fn get_token_account(
        &self,
        mint: &Pubkey,
        token_program: Option<&Pubkey>,
        is_associated_only: bool,
    ) -> Option<&TokenAccount> {
        let token_program = token_program.unwrap_or_else(|| &spl_token::ID);
        let ata = self.get_associated_token_account(&mint, Some(token_program));
        let mut accs: Vec<&TokenAccount> = self
            .token_accounts
            .iter()
            .filter(|x| x.mint.eq(&mint) && (!is_associated_only || x.pubkey.eq(&Some(ata))))
            .collect();
        accs.sort_by_key(|x| &x.amount);
        accs.last().map(|v| &**v)
    }
    pub fn get_associated_token_account(
        &self,
        mint: &Pubkey,
        program_id: Option<&Pubkey>,
    ) -> Pubkey {
        get_ata_address(&self.owner_pubkey().unwrap(), mint, program_id).0
    }
}
